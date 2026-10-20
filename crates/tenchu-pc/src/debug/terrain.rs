use glam::Vec3;
use tenchu_assets::{
    Result,
    terrain::{AreaMap, Node},
};

pub struct Surface {
    pub node: Node,
    pub corners: [Vec3; 4],
    pub color: [f32; 3],
}

pub fn surfaces(map: &AreaMap) -> Result<Vec<Surface>> {
    let mut surfaces = Vec::new();
    for row in &map.rows {
        for i in 0..row.count {
            let node = map.node(row.list + i * 16)?;
            let color = [
                (0x200, [1.0, 0.18, 0.3]),
                (0x100, [1.0, 0.48, 0.16]),
                (4, [0.12, 0.57, 1.0]),
                (8, [0.24, 0.86, 0.94]),
                (2, [0.74, 0.35, 1.0]),
                (0, [0.23, 0.84, 0.65]),
            ]
            .into_iter()
            .find(|(mask, _)| *mask == 0 || node.attributes & mask != 0)
            .unwrap()
            .1;
            let [x1, z1, x2, z2] = node.bounds.map(f32::from);
            let sx = x2 - x1 + 1.0;
            let sz = z2 - z1 + 1.0;
            for z in 0..4 {
                for x in 0..4 {
                    if node.division & (1 << (z * 4 + x)) == 0 {
                        continue;
                    }
                    let point = |cx: i32, cz: i32| {
                        let dx = sx * cx as f32 / 4.0;
                        let dz = sz * cz as f32 / 4.0;
                        let slope = match node.attributes & 0xc000 {
                            0x4000 => dx * f32::from(node.dy) / sx,
                            0x8000 => dz * f32::from(node.dy) / sz,
                            _ => 0.0,
                        };
                        Vec3::new(
                            (x1 + dx) * 10.0,
                            (f32::from(node.y) + slope) * 10.0,
                            (z1 + dz) * 10.0,
                        )
                    };
                    surfaces.push(Surface {
                        node,
                        corners: [
                            point(x, z),
                            point(x + 1, z),
                            point(x, z + 1),
                            point(x + 1, z + 1),
                        ],
                        color,
                    });
                }
            }
        }
    }
    Ok(surfaces)
}

fn triangle(origin: Vec3, direction: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let h = direction.cross(e2);
    let determinant = e1.dot(h);
    if determinant.abs() < 0.00001 {
        return None;
    }
    let s = origin - a;
    let u = s.dot(h) / determinant;
    let q = s.cross(e1);
    let v = direction.dot(q) / determinant;
    let distance = e2.dot(q) / determinant;
    (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && distance > 0.0 && distance < 250_000.0)
        .then_some(distance)
}

pub fn pick(surfaces: &[Surface], origin: Vec3, direction: Vec3) -> Option<[i32; 3]> {
    surfaces
        .iter()
        .flat_map(|s| {
            [[0, 1, 2], [1, 3, 2]]
                .into_iter()
                .filter_map(move |indices| {
                    let distance = triangle(origin, direction, indices.map(|i| s.corners[i]))?;
                    let mut point = (origin + direction * distance)
                        .to_array()
                        .map(|v| v.round() as i32);
                    if !s.node.contains(point[0] / 10, point[2] / 10) {
                        return None;
                    }
                    let y = tenchu_core::terrain::node_level(s.node, point[0] / 10, point[2] / 10)
                        .ok()?;
                    if y == tenchu_core::terrain::NONE {
                        return None;
                    }
                    point[1] = y * 10;
                    Some((distance, point))
                })
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p)
}
