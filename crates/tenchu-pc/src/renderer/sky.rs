use crate::render_mesh::{DOUBLE_SIDED, SCREEN, SCREEN_WIDE, SKY, Vertex};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3};

pub fn geometry(camera: Mat4) -> [Vertex; 6] {
    let inverse = camera.inverse();
    let corners = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
    [0, 1, 2, 1, 3, 2].map(|i| {
        let (x, y) = corners[i];
        let near = inverse.project_point3(Vec3::new(x, y, 0.0));
        let far = inverse.project_point3(Vec3::new(x, y, 0.9));
        Vertex {
            position: [x * 160.0, -y * 120.0, 0.99999994],
            normal: (far - near).to_array(),
            flags: SCREEN | SCREEN_WIDE | DOUBLE_SIDED | SKY,
            opacity: 1.0,
            ..Vertex::zeroed()
        }
    })
}
