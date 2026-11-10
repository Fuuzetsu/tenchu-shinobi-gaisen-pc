use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
};
use bytemuck::Zeroable;
use glam::Vec3;
mod gaisen;
pub use gaisen::geometry;

pub fn path(points: &[Vec3]) -> Geometry {
    let mut vertices = Vec::new();
    for (segment, pair) in points.windows(2).enumerate() {
        let [a, b] = [pair[0], pair[1]];
        let direction = (b - a).normalize_or_zero();
        if direction == Vec3::ZERO {
            continue;
        }
        let right = direction.any_orthonormal_vector();
        let up = direction.cross(right);
        for side in 0..8 {
            let quad = std::array::from_fn::<_, 4, _>(|i| {
                let angle = (side + i % 2) as f32 * std::f32::consts::TAU / 8.0;
                let normal = right * angle.cos() + up * angle.sin();
                let weave = 0.8 + 0.2 * (angle * 3.0 - (segment + i / 2) as f32 * 2.5).sin();
                Vertex {
                    position: ((if i / 2 == 0 { a } else { b }) + normal * 16.0).to_array(),
                    normal: normal.to_array(),
                    color: [0.38 * weave, 0.30 * weave, 0.19 * weave],
                    opacity: 1.0,
                    flags: render_mesh::FOG | render_mesh::DOUBLE_SIDED,
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
            blend: 0,
            range: 0..count,
        }],
    }
}
