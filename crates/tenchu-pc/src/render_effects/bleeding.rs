use super::Surface;
use crate::render_mesh::{SCREEN, UI, Vertex};

pub(crate) fn draw(surfaces: &mut Vec<Surface>, position: [f32; 2], depth: f32) {
    let outline = [
        [0.0, -6.0],
        [3.4, -0.8],
        [4.0, 1.7],
        [2.8, 4.1],
        [0.0, 5.0],
        [-2.8, 4.1],
        [-4.0, 1.7],
        [-3.4, -0.8],
    ];
    let mut vertices = Vec::new();
    for (scale, color) in [(1.0, [0.08, 0.02, 0.02]), (0.75, [1.0, 0.16, 0.12])] {
        for i in 0..outline.len() {
            for point in [[0.0, 0.0], outline[i], outline[(i + 1) % outline.len()]] {
                vertices.push(Vertex {
                    position: [
                        position[0] + point[0] * scale,
                        position[1] + point[1] * scale,
                        depth * 0.000001,
                    ],
                    color,
                    normal: [0.0; 3],
                    uv: [0.0; 2],
                    opacity: 1.0,
                    page: 0,
                    palette: 0,
                    material: 0,
                    flags: SCREEN | UI,
                });
            }
        }
    }
    surfaces.push(Surface {
        depth,
        blend: 0,
        vertices,
    });
}
