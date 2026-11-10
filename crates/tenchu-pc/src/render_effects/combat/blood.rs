use crate::render_mesh::{self, Vertex};
use glam::{Mat3, Vec3};

mod gaisen;

pub(crate) struct Drop {
    pub position: Vec3,
    pub velocity: Vec3,
    pub count: u32,
    pub size: f32,
    pub seed: u32,
}

impl Drop {
    pub fn draw(&self, camera: Mat3, mut emit: impl FnMut(Vec3, [Vec3; 2], [f32; 3])) {
        let projected = self.velocity - camera.z_axis * self.velocity.dot(camera.z_axis);
        let along = projected.try_normalize().unwrap_or(camera.y_axis);
        let across = along.cross(camera.z_axis);
        for i in 0..self.count {
            let seed = self.seed.wrapping_add(i * 13);
            let radius = self.size * (0.45 + super::noise(seed));
            let center = self.position
                + Vec3::new(
                    super::noise(seed + 1) - 0.5,
                    super::noise(seed + 2) - 0.5,
                    super::noise(seed + 3) - 0.5,
                ) * self.size
                    * 3.0
                - self.velocity * (super::noise(seed + 4) * 1.4);
            emit(
                center,
                [
                    across * radius,
                    along * (radius + projected.length() * 0.22),
                ],
                [0.30 + super::noise(seed + 3) * 0.16, 0.006, 0.012],
            );
        }
    }
}

pub(crate) fn original_hd(vertices: &mut [Vertex], blend: u8) -> u8 {
    let Some(first) = vertices.first() else {
        return blend;
    };
    let textured = first.flags & render_mesh::TEXTURED != 0;
    for (vertex, i) in vertices.iter_mut().zip([0, 1, 2, 1, 3, 2]) {
        vertex.flags |= render_mesh::STAIN | render_mesh::BLOOD_HD;
        if !textured {
            vertex.flags |= render_mesh::ALPHA;
            vertex.uv = [
                if i & 1 == 0 { -1.0 } else { 1.0 },
                if i & 2 == 0 { -1.0 } else { 1.0 },
            ];
        }
    }
    if textured { blend } else { 5 }
}
