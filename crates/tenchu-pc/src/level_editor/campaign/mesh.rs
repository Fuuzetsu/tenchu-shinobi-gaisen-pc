use glam::Vec3;
use tenchu_assets::{
    Result,
    model::{Corner, Face, Geometry, Part},
    terrain::Node,
};

#[derive(Clone, Copy)]
struct Vertex {
    point: Vec3,
    uv: [f32; 2],
    color: [f32; 3],
}

impl Vertex {
    fn mix(self, other: Self, t: f32) -> Self {
        Self {
            point: self.point.lerp(other.point, t),
            uv: std::array::from_fn(|i| self.uv[i] + (other.uv[i] - self.uv[i]) * t),
            color: std::array::from_fn(|i| self.color[i] + (other.color[i] - self.color[i]) * t),
        }
    }
}

fn split(
    polygon: Vec<Vertex>,
    axis: usize,
    value: f32,
    positive: bool,
) -> (Vec<Vertex>, Vec<Vertex>) {
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    let Some(&last) = polygon.last() else {
        return (inside, outside);
    };
    let distance = |v: Vertex| (v.point[axis] - value) * if positive { 1.0 } else { -1.0 };
    let mut previous = last;
    for current in polygon {
        let (a, b) = (distance(previous), distance(current));
        if (a >= 0.0) != (b >= 0.0) {
            let crossing = previous.mix(current, a / (a - b));
            inside.push(crossing);
            outside.push(crossing);
        }
        if b >= 0.0 {
            inside.push(current);
        } else {
            outside.push(current);
        }
        previous = current;
    }
    (inside, outside)
}

pub(super) fn height(node: Node, point: Vec3, t2: bool) -> f32 {
    let [x1, z1, x2, z2] = node.bounds.map(|v| f32::from(v) * 10.0);
    let inclusive = if t2 { 0.0 } else { 10.0 };
    let slope = match (t2, node.attributes & 0xc000) {
        (false, 0x4000) | (true, 0x8000) => (point.x - x1) / (x2 - x1 + inclusive),
        (false, 0x8000) | (true, 0xc000) => (point.z - z1) / (z2 - z1 + inclusive),
        _ => 0.0,
    };
    (f32::from(node.y) + slope * f32::from(node.dy)) * 10.0
}

fn append(geometry: &mut Geometry, source: &Face, polygon: &[Vertex], offset: Vec3) -> Result<()> {
    for i in 1..polygon.len().saturating_sub(1) {
        let triangle = [polygon[0], polygon[i], polygon[i + 1]];
        if (triangle[1].point - triangle[0].point)
            .cross(triangle[2].point - triangle[0].point)
            .length_squared()
            < 0.01
        {
            continue;
        }
        let mut face = source.clone();
        face.mode &= !8;
        face.corners.clear();
        for vertex in triangle {
            let index = geometry
                .vertices
                .len()
                .try_into()
                .map_err(|_| "Terrain edit produces too many vertices in one chunk")?;
            let mut point = [0i16; 3];
            for axis in 0..3 {
                point[axis] = ((vertex.point[axis] - offset[axis]).round() as i32)
                    .try_into()
                    .map_err(|_| "Terrain edit exceeds this chunk's local coordinate range")?;
            }
            geometry.vertices.push(point);
            face.corners.push(Corner {
                vertex: index,
                normal: None,
                color: vertex.color.map(|v| v.round().clamp(0.0, 255.0) as u8),
                uv: vertex.uv.map(|v| v.round().clamp(0.0, 255.0) as u8),
            });
        }
        geometry.faces.push(face);
    }
    Ok(())
}

/// Cut only the selected floor footprint out of source polygons, preserving UVs
/// outside it. Raised/lowered surfaces get side walls instead of stretching the
/// neighbouring floor away from its collision height.
pub(super) fn changes_surface(original: Node, edited: Node) -> bool {
    original.y != edited.y
        || original.dy != edited.dy
        || original.attributes & 0xc000 != edited.attributes & 0xc000
        || original.division != edited.division
}

pub(super) fn sculpt(parts: &mut [Part], original: Node, edited: Node, t2: bool) -> Result<usize> {
    if !changes_surface(original, edited) {
        return Ok(0);
    }
    let [x1, z1, x2, z2] = original.bounds.map(|v| f32::from(v) * 10.0);
    let inclusive = if t2 { 0.0 } else { 10.0 };
    let [x2, z2] = [x2 + inclusive, z2 + inclusive];
    if x1 >= x2 || z1 >= z2 {
        return Err("Terrain surface has no area".into());
    }
    let rects = if original.division == u16::MAX && edited.division == u16::MAX {
        vec![([x1, z1, x2, z2], true)]
    } else {
        (0..16)
            .filter(|i| (original.division | edited.division) & (1 << i) != 0)
            .map(|i| {
                let x = (i % 4) as f32;
                let z = (i / 4) as f32;
                (
                    [
                        x1 + (x2 - x1) * x / 4.0,
                        z1 + (z2 - z1) * z / 4.0,
                        x1 + (x2 - x1) * (x + 1.0) / 4.0,
                        z1 + (z2 - z1) * (z + 1.0) / 4.0,
                    ],
                    edited.division & (1 << i) != 0,
                )
            })
            .collect()
    };
    let mut changed = 0;
    let mut vertices = Vec::new();
    for (rect, solid) in rects {
        let mut edges = std::collections::BTreeMap::new();
        let mut tails = Vec::new();
        for (part_index, part) in parts.iter_mut().enumerate() {
            let offset = Vec3::from_array(part.position.map(|v| f32::from(v) * 10.0));
            for (geometry_index, geometry) in part.geometry.iter_mut().enumerate() {
                if !geometry.joints.is_empty() {
                    continue;
                }
                let mut faces = std::mem::take(&mut geometry.faces);
                if let Some(end) = faces.iter().position(|face| face.flags & 1 == 0) {
                    tails.push((part_index, geometry_index, faces.split_off(end)));
                }
                for face in faces {
                    if face.flags & 1 == 0 || face.corners.len() < 3 {
                        geometry.faces.push(face);
                        continue;
                    }
                    let bounds = face.corners.iter().fold(
                        [
                            f32::INFINITY,
                            f32::INFINITY,
                            f32::NEG_INFINITY,
                            f32::NEG_INFINITY,
                        ],
                        |[x1, z1, x2, z2], corner| {
                            let point = offset
                                + Vec3::from_array(
                                    geometry.vertices[corner.vertex as usize].map(f32::from),
                                );
                            [
                                x1.min(point.x),
                                z1.min(point.z),
                                x2.max(point.x),
                                z2.max(point.z),
                            ]
                        },
                    );
                    if bounds[2] <= rect[0]
                        || bounds[0] >= rect[2]
                        || bounds[3] <= rect[1]
                        || bounds[1] >= rect[3]
                    {
                        geometry.faces.push(face);
                        continue;
                    }
                    vertices.clear();
                    vertices.extend(face.corners.iter().map(|c| Vertex {
                        point: offset
                            + Vec3::from_array(geometry.vertices[c.vertex as usize].map(f32::from)),
                        uv: c.uv.map(f32::from),
                        color: c.color.map(f32::from),
                    }));
                    if (vertices[1].point - vertices[0].point)
                        .cross(vertices[2].point - vertices[0].point)
                        .y
                        .abs()
                        < 0.1
                        || vertices
                            .iter()
                            .any(|v| (v.point.y - height(original, v.point, t2)).abs() > 150.0)
                    {
                        geometry.faces.push(face);
                        continue;
                    }
                    for triangle in crate::render_mesh::face_order(&face).chunks_exact(3) {
                        let mut inside: Vec<_> = triangle.iter().map(|&i| vertices[i]).collect();
                        let mut outside = Vec::new();
                        for (axis, value, positive) in [
                            (0, rect[0], true),
                            (0, rect[2], false),
                            (2, rect[1], true),
                            (2, rect[3], false),
                        ] {
                            let (keep, discard) = split(inside, axis, value, positive);
                            inside = keep;
                            outside.push(discard);
                        }
                        for polygon in outside {
                            append(geometry, &face, &polygon, offset)?;
                        }
                        if inside.len() < 3 {
                            continue;
                        }
                        changed += 1;
                        if !solid {
                            continue;
                        }
                        let raised: Vec<_> = inside
                            .iter()
                            .map(|v| {
                                let mut out = *v;
                                out.point.y +=
                                    height(edited, v.point, t2) - height(original, v.point, t2);
                                out
                            })
                            .collect();
                        append(geometry, &face, &raised, offset)?;
                        for i in 0..inside.len() {
                            let j = (i + 1) % inside.len();
                            let a = inside[i].point.to_array().map(|v| v.round() as i32);
                            let b = inside[j].point.to_array().map(|v| v.round() as i32);
                            if a == b {
                                continue;
                            }
                            let key = if a < b { (a, b) } else { (b, a) };
                            if let std::collections::btree_map::Entry::Vacant(entry) =
                                edges.entry(key)
                            {
                                entry.insert(Some((
                                    part_index,
                                    geometry_index,
                                    face.clone(),
                                    [inside[i], inside[j], raised[j], raised[i]],
                                    offset,
                                )));
                            } else {
                                edges.insert(key, None);
                            }
                        }
                    }
                }
            }
        }
        for (part, geometry, mut face, mut wall, offset) in edges.into_values().flatten() {
            let u = face.corners.iter().map(|c| c.uv[0]).min().unwrap_or(0) as f32;
            let u2 = face.corners.iter().map(|c| c.uv[0]).max().unwrap_or(0) as f32;
            let v = face.corners.iter().map(|c| c.uv[1]).min().unwrap_or(0) as f32;
            let v2 = face.corners.iter().map(|c| c.uv[1]).max().unwrap_or(0) as f32;
            for (vertex, uv) in wall.iter_mut().zip([[u, v2], [u2, v2], [u2, v], [u, v]]) {
                vertex.uv = uv;
            }
            face.double_sided = true;
            append(&mut parts[part].geometry[geometry], &face, &wall, offset)?;
        }
        for (part, geometry, tail) in tails {
            parts[part].geometry[geometry].faces.extend(tail);
        }
    }
    Ok(changed)
}
