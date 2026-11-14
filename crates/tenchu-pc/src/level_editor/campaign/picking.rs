use super::*;
use glam::Vec3;
use tenchu_assets::model::Part;

#[derive(Default)]
pub(in crate::level_editor) struct Mesh {
    chunks: Vec<Chunk>,
}

struct Chunk {
    bounds: [Vec3; 2],
    triangles: Vec<[Vec3; 3]>,
}

pub(super) struct Hit {
    pub distance: f32,
    pub point: Vec3,
    pub triangle: [Vec3; 3],
}

impl Mesh {
    pub fn new(parts: &[Part]) -> Self {
        Self {
            chunks: parts
                .iter()
                .filter_map(|part| {
                    let offset = Vec3::from_array(part.position.map(|v| f32::from(v) * 10.0));
                    let mut triangles = Vec::new();
                    let mut bounds = [Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)];
                    for geometry in &part.geometry {
                        for face in geometry.faces.iter().take_while(|face| face.flags & 1 != 0) {
                            for indices in crate::render_mesh::face_order(face).chunks_exact(3) {
                                let triangle = std::array::from_fn(|i| {
                                    offset
                                        + Vec3::from_array(
                                            geometry.vertices
                                                [face.corners[indices[i]].vertex as usize]
                                                .map(f32::from),
                                        )
                                });
                                for point in triangle {
                                    bounds = [bounds[0].min(point), bounds[1].max(point)];
                                }
                                triangles.push(triangle);
                            }
                        }
                    }
                    (!triangles.is_empty()).then_some(Chunk { bounds, triangles })
                })
                .collect(),
        }
    }

    pub(super) fn pick(&self, origin: Vec3, direction: Vec3) -> Option<Hit> {
        let mut nearest: Option<Hit> = None;
        for chunk in &self.chunks {
            let mut near = 0.0_f32;
            let mut far = nearest.as_ref().map_or(250_000.0, |hit| hit.distance);
            for axis in 0..3 {
                if direction[axis].abs() < 1e-8 {
                    if origin[axis] < chunk.bounds[0][axis] || origin[axis] > chunk.bounds[1][axis]
                    {
                        far = -1.0;
                    }
                } else {
                    let a = (chunk.bounds[0][axis] - origin[axis]) / direction[axis];
                    let b = (chunk.bounds[1][axis] - origin[axis]) / direction[axis];
                    near = near.max(a.min(b));
                    far = far.min(a.max(b));
                }
            }
            if near > far {
                continue;
            }
            for &triangle in &chunk.triangles {
                if let Some(distance) = crate::debug::terrain::triangle(origin, direction, triangle)
                    && nearest.as_ref().is_none_or(|hit| distance < hit.distance)
                {
                    nearest = Some(Hit {
                        distance,
                        point: origin + direction * distance,
                        triangle,
                    });
                }
            }
        }
        nearest
    }
}

// ACM/MCD represent walls as the sides of height columns, not vertical planes.
// Resolve a visible wall to the closest column above it; a roof must match its
// height. Keep visible hit testing separate so an unmatched wall occludes floors.
pub(super) fn surface(
    hit: &Hit,
    nodes: impl Iterator<Item = ([usize; 2], Node)>,
    t2: bool,
) -> Option<[usize; 2]> {
    let [a, b, c] = hit.triangle;
    let normal = (b - a).cross(c - a).normalize_or_zero();
    let wall = normal.y.abs() < 0.15;
    nodes
        .filter_map(|(key, node)| {
            if node.division == 0 {
                return None;
            }
            let [x1, z1, x2, z2] = node.bounds.map(|v| f32::from(v) * 10.0);
            let pad = if t2 { 0.0 } else { 10.0 };
            if x1 >= x2 + pad || z1 >= z2 + pad {
                return None;
            }
            let x = hit.point.x.clamp(x1, x2 + pad);
            let z = hit.point.z.clamp(z1, z2 + pad);
            let horizontal = (hit.point.x - x).abs() + (hit.point.z - z).abs();
            if horizontal > 150.0 {
                return None;
            }
            let qx = ((x - x1) * 4.0 / (x2 + pad - x1)).floor().clamp(0.0, 3.0) as usize;
            let qz = ((z - z1) * 4.0 / (z2 + pad - z1)).floor().clamp(0.0, 3.0) as usize;
            if node.division & (1 << (qz * 4 + qx)) == 0 {
                return None;
            }
            let y = mesh::height(node, glam::vec3(x, 0.0, z), t2);
            let vertical = hit.point.y - y;
            if wall {
                let edge = (x - x1).min(x2 + pad - x).min(z - z1).min(z2 + pad - z);
                if vertical < -150.0 || edge > 150.0 {
                    return None;
                }
                Some((vertical.abs() + horizontal * 4.0 + edge, key))
            } else {
                (vertical.abs() <= 150.0).then_some((vertical.abs() + horizontal * 4.0, key))
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, key)| key)
}

#[cfg(all(test, target_os = "linux"))]
impl Editor {
    pub(crate) fn campaign_wall_fixture(&self) -> Option<(Vec3, Vec3, [usize; 2])> {
        let cache = self.campaign_cache.as_ref()?;
        for chunk in &self.campaign_mesh.chunks {
            for &[a, b, c] in &chunk.triangles {
                let normal = (b - a).cross(c - a).normalize_or_zero();
                if normal.y.abs() > 0.15 || normal.length_squared() < 0.5 {
                    continue;
                }
                let point = (a + b + c) / 3.0;
                let eye = point - normal * 1500.0;
                let Some(hit) = self.campaign_mesh.pick(eye, normal) else {
                    continue;
                };
                if (hit.point - point).length() > 10.0 {
                    continue;
                }
                if let Some(key) = surface(
                    &hit,
                    cache.nodes.iter().map(|(&k, &n)| (k, n)),
                    cache.source.t2(),
                ) && cache.nodes[&key].attributes & 0xc002 == 0
                {
                    return Some((eye, point, key));
                }
            }
        }
        None
    }
}
