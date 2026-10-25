use super::{Contact, Kind};
use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
};
use bytemuck::Zeroable;
use glam::{Mat3, Mat4, Vec3};
impl Contact {
    pub fn geometry(
        &self,
        camera: Mat4,
        alpha: f32,
        settings: &crate::settings::Effects,
    ) -> Geometry {
        let mut vertices = Vec::new();
        for mark in self
            .wet
            .iter()
            .filter(|_| settings.splashes || settings.scatter)
        {
            vertices.extend(mark.vertices.iter().map(|v| Vertex {
                opacity: v.opacity * (1.0 - mark.age / 20.0).powi(2),
                ..*v
            }));
        }
        let inverse = camera.inverse();
        let near = inverse.project_point3(Vec3::ZERO);
        let forward = (inverse.project_point3(Vec3::Z) - near).normalize_or_zero();
        let right = forward.cross(Vec3::NEG_Y).normalize_or_zero();
        let up = right.cross(forward);
        let mut particles: Vec<_> = self
            .particles
            .iter()
            .filter(|p| match p.kind {
                Kind::Petal | Kind::Leaf => settings.leaves || settings.scatter,
                Kind::Snow => settings.scatter,
                Kind::Drop => settings.splashes || settings.scatter,
                Kind::Spark => settings.sparks,
            })
            .collect();
        particles.sort_unstable_by(|a, b| {
            forward
                .dot(b.position - near)
                .total_cmp(&forward.dot(a.position - near))
        });
        for p in particles {
            let center = p.previous.lerp(p.position, alpha.clamp(0.0, 1.0));
            let fade = ((p.life - p.age)
                / if matches!(p.kind, Kind::Petal | Kind::Leaf) {
                    8.0
                } else {
                    p.life * 0.5
                })
            .clamp(0.0, 1.0);
            if matches!(p.kind, Kind::Petal | Kind::Leaf) {
                let normal = p.ground.unwrap_or_else(|| {
                    Mat3::from_rotation_z((p.age * 2.0 + p.phase).sin())
                        * (Mat3::from_rotation_x(p.age * 1.4 + p.phase) * Vec3::Y)
                });
                let tangent = Mat3::from_axis_angle(normal, p.phase + p.age * 0.3)
                    * normal.any_orthonormal_vector();
                let across = normal.cross(tangent);
                let points = [
                    [-1.0, 0.0],
                    [-0.4, -0.75],
                    [0.5, -0.6],
                    [1.0, 0.0],
                    [0.5, 0.6],
                    [-0.4, 0.75],
                ];
                for i in [0, 1, 5, 1, 2, 5, 2, 4, 5, 2, 3, 4] {
                    let [x, y] = points[i];
                    vertices.push(Vertex {
                        position: (center
                            + tangent * x * p.size
                            + across * y * p.size
                            + normal * (1.0 - x.abs()) * p.size * 0.12)
                            .to_array(),
                        normal: normal.to_array(),
                        color: p.color.map(|c| c * (if y < 0.0 { 0.88 } else { 1.0 })),
                        opacity: fade,
                        flags: render_mesh::FOG | render_mesh::ALPHA | render_mesh::DOUBLE_SIDED,
                        ..Vertex::zeroed()
                    });
                }
            } else {
                let (side, axis) = if p.kind == Kind::Spark || p.kind == Kind::Drop {
                    let axis = p.velocity.normalize_or_zero();
                    (
                        axis.cross(forward).normalize_or_zero() * p.size,
                        axis * p.velocity.length() * 0.3,
                    )
                } else {
                    (right * p.size, up * p.size)
                };
                let start = vertices.len();
                crate::visuals::weather::quad(
                    &mut vertices,
                    center,
                    side,
                    axis,
                    p.color,
                    fade * if p.kind == Kind::Snow { 0.5 } else { 0.8 },
                );
                if p.kind == Kind::Spark {
                    for v in &mut vertices[start..] {
                        v.flags |= render_mesh::EMITTER;
                    }
                }
            }
        }
        for ring in self
            .rings
            .iter()
            .filter(|_| settings.splashes || settings.scatter)
        {
            let age = (ring.age + alpha / 30.0) / 1.2;
            let radius = (30.0 + age * 550.0) * ring.strength.sqrt();
            let width = 7.0 * (1.0 - age) + 2.0;
            for segment in 0..32 {
                let quad = std::array::from_fn::<_, 4, _>(|i| {
                    let angle = (segment + i % 2) as f32 * std::f32::consts::TAU / 32.0;
                    let distance = radius + if i / 2 == 0 { -width } else { width };
                    Vertex {
                        position: (ring.position
                            + Vec3::new(angle.cos() * distance, -3.0, angle.sin() * distance))
                        .to_array(),
                        normal: Vec3::NEG_Y.to_array(),
                        color: [0.5, 0.69, 0.8],
                        opacity: (1.0 - age).max(0.0) * 0.38,
                        flags: render_mesh::FOG | render_mesh::ALPHA | render_mesh::DOUBLE_SIDED,
                        ..Vertex::zeroed()
                    }
                });
                vertices.extend([0, 1, 2, 1, 3, 2].map(|i| quad[i]));
            }
        }
        let count = vertices.len() as u32;
        Geometry {
            vertices,
            batches: vec![Batch {
                blend: 5,
                range: 0..count,
            }],
        }
    }
}
