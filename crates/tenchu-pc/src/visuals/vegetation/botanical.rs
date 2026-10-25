use super::{Kind, Material, Vertex, render_mesh};
use glam::{Mat3, Vec2, Vec3};

fn random(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 8) as f32 / 16777216.0
}

pub fn canopy(
    source: &[Vertex],
    materials: &[Material],
    kind: Kind,
    dense: bool,
    budget: &mut usize,
) -> Vec<Vertex> {
    let mut out = source.to_vec();
    let mut seed = 0x762a41c9;
    for t in source.chunks_exact(3) {
        let Some(material) = t[0]
            .material
            .checked_sub(1)
            .and_then(|i| materials.get(i as usize))
        else {
            continue;
        };
        let p: [Vec3; 3] = std::array::from_fn(|i| Vec3::from(t[i].position));
        let ab = p[1] - p[0];
        let ac = p[2] - p[0];
        let area = ab.cross(ac).length() * 0.5;
        let normal = ab.cross(ac).normalize_or_zero();
        let span = ab.length().max(ac.length());
        let count = (area / if dense { 35000.0 } else { 65000.0 })
            .ceil()
            .clamp(2.0, if dense { 80.0 } else { 48.0 }) as usize;
        let uv: [Vec2; 3] = std::array::from_fn(|i| Vec2::from(t[i].uv));
        let units = ((ab.length() + ac.length())
            / ((uv[1] - uv[0]).length() + (uv[2] - uv[0]).length()).max(1.0))
        .clamp(20.0, 300.0);
        let rect = material
            .region
            .map_or([0.0, 0.0, 1.0, 1.0], |r| r.rect.map(f32::from));
        for _ in 0..count {
            if *budget < 12 {
                return out;
            }
            let root = random(&mut seed).sqrt();
            let b = root * random(&mut seed);
            let weights = [1.0 - root, b, root - b];
            let center_uv = uv[0] * weights[0] + uv[1] * weights[1] + uv[2] * weights[2];
            let texel = (center_uv - Vec2::new(rect[0], rect[1])) / Vec2::new(rect[2], rect[3]);
            let x = (texel.x.clamp(0.0, 0.9999) * material.image.size[0] as f32) as usize;
            let y = (texel.y.clamp(0.0, 0.9999) * material.image.size[1] as f32) as usize;
            let rgba = &material.image.rgba[(y * material.image.size[0] as usize + x) * 4..][..4];
            if rgba[3] < 200 || !leaf_color(rgba, kind) {
                continue;
            }
            let center = p[0] * weights[0]
                + p[1] * weights[1]
                + p[2] * weights[2]
                + normal * ((random(&mut seed) - 0.5) * span.min(3500.0) * 0.28);
            let angle = random(&mut seed) * std::f32::consts::TAU;
            let rotation = Mat3::from_axis_angle(normal, angle);
            let tangent = rotation * ab.normalize_or_zero();
            let half = units * (0.45 + random(&mut seed) * 0.6);
            let bitangent = normal.cross(tangent);
            let color: [f32; 3] =
                std::array::from_fn(|a| (0..3).map(|i| t[i].color[a] * weights[i]).sum::<f32>());
            let tint = 0.88 + random(&mut seed) * 0.20;
            {
                let fold = Mat3::from_axis_angle(tangent, (random(&mut seed) - 0.5) * 1.4);
                let across = fold * bitangent;
                let facing = tangent.cross(across).normalize_or_zero();
                let rim = [
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(-0.4, -0.8),
                    Vec2::new(0.4, -0.8),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.4, 0.8),
                    Vec2::new(-0.4, 0.8),
                ];
                for i in [0, 1, 5, 1, 2, 5, 2, 4, 5, 2, 3, 4] {
                    let c = rim[i];
                    let sample = (center_uv + c * 1.4).clamp(
                        Vec2::new(rect[0], rect[1]),
                        Vec2::new(rect[0] + rect[2] - 1.0, rect[1] + rect[3] - 1.0),
                    );
                    out.push(Vertex {
                        position: (center
                            + tangent * c.x * half
                            + across * c.y * half * 0.7
                            + facing * half * 0.18 * (1.0 - c.x.abs()))
                        .to_array(),
                        uv: sample.to_array(),
                        normal: facing.to_array(),
                        color: color.map(|c| c * tint),
                        flags: t[0].flags
                            | render_mesh::DOUBLE_SIDED
                            | render_mesh::FOLIAGE
                            | render_mesh::FLUTTER,
                        ..t[0]
                    });
                }
                *budget -= 12;
            }
        }
    }
    out
}

fn leaf_color(c: &[u8], kind: Kind) -> bool {
    let [r, g, b] = [c[0] as f32, c[1] as f32, c[2] as f32];
    match kind {
        Kind::Blossoms => r > g * 0.95 && b > g * 0.75,
        Kind::Autumn => r > g * 1.1 && r > b * 1.8,
        _ => g > r * 0.8 && g > b * 0.9,
    }
}

#[derive(Clone, Copy)]
pub struct Shape {
    root: Vec3,
    height: f32,
}
impl Shape {
    pub fn new(vertices: &[Vertex]) -> Self {
        let (min, max) = vertices.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(min, max), v| (min.min(v.position.into()), max.max(v.position.into())),
        );
        Self {
            root: Vec3::new((min.x + max.x) * 0.5, max.y, (min.z + max.z) * 0.5),
            height: (max.y - min.y).max(1.0),
        }
    }
    pub fn variant(self, translation: [i32; 3]) -> Variant {
        let mut seed = translation.into_iter().fold(0x48c2e37bu32, |seed, n| {
            seed.wrapping_mul(16777619) ^ n as u32
        });
        let phase = random(&mut seed) * std::f32::consts::TAU;
        Variant {
            shape: self,
            rotation: Mat3::from_rotation_y((random(&mut seed) - 0.5) * 0.4),
            scale: Vec3::new(
                0.9 + random(&mut seed) * 0.2,
                0.96 + random(&mut seed) * 0.08,
                0.9 + random(&mut seed) * 0.2,
            ),
            phase,
        }
    }
}
pub struct Variant {
    shape: Shape,
    rotation: Mat3,
    scale: Vec3,
    phase: f32,
}
impl Variant {
    pub fn vertex(&self, vertex: &mut Vertex) {
        let p = Vec3::from(vertex.position) - self.shape.root;
        let weight = (-p.y / self.shape.height).clamp(0.0, 1.0);
        let lobe = (p.x * 0.001 + self.phase).sin() * (p.z * 0.0007 + self.phase).cos();
        let mut varied = p.lerp(self.rotation * (p * self.scale), weight);
        varied.x += (self.phase.sin() * 0.06 + lobe * 0.035) * self.shape.height * weight * weight;
        varied.z += (self.phase.cos() * 0.06 + lobe * 0.025) * self.shape.height * weight * weight;
        vertex.position = (varied + self.shape.root).to_array();
        vertex.normal = (self.rotation * (Vec3::from(vertex.normal) / self.scale))
            .normalize_or_zero()
            .to_array();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;
    #[test]
    fn stable_variants_hold_roots_and_create_different_crowns() {
        let shape = Shape {
            root: Vec3::ZERO,
            height: 3000.0,
        };
        let a = shape.variant([1000, 0, 0]);
        let b = shape.variant([5000, 0, 0]);
        let vertex = Vertex {
            position: [250.0, 0.0, 100.0],
            ..Vertex::zeroed()
        };
        let mut root = vertex;
        a.vertex(&mut root);
        assert_eq!(root.position, vertex.position);
        let mut crown = Vertex {
            position: [0.0, -3000.0, 0.0],
            ..vertex
        };
        let mut other = crown;
        let mut repeated = crown;
        a.vertex(&mut crown);
        b.vertex(&mut other);
        shape.variant([1000, 0, 0]).vertex(&mut repeated);
        assert_eq!(crown.position, repeated.position);
        assert!(Vec3::from(crown.position).distance(other.position.into()) > 50.0);
        assert!(Vec3::from(crown.position).distance(Vec3::new(0.0, -3000.0, 0.0)) < 500.0);
    }
}
