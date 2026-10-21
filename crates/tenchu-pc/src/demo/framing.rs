use super::Scene;
use crate::{debug::camera::Camera, render_mesh::Vertex};
use glam::Vec3;

fn blocked(vertices: &[Vertex], eye: Vec3, target: Vec3) -> bool {
    let ray = target - eye;
    let length = ray.length();
    let direction = ray / length;
    vertices.chunks_exact(3).any(|triangle| {
        let a = Vec3::from(triangle[0].position);
        let b = Vec3::from(triangle[1].position);
        let c = Vec3::from(triangle[2].position);
        let ab = b - a;
        let ac = c - a;
        let h = direction.cross(ac);
        let determinant = ab.dot(h);
        if determinant.abs() < 0.00001 {
            return false;
        }
        let s = eye - a;
        let u = s.dot(h) / determinant;
        let q = s.cross(ab);
        let v = direction.dot(q) / determinant;
        let distance = ac.dot(q) / determinant;
        u >= 0.0 && v >= 0.0 && u + v <= 1.0 && distance > 50.0 && distance < length - 400.0
    })
}

pub fn unobstructed(scene: &mut Scene, vertices: &[Vertex]) -> bool {
    let yaw = scene.yaw;
    for pitch in [scene.pitch, -0.3, -0.65, -1.45] {
        for angle in [0, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 6, -6, 7, -7, 8] {
            let candidate = Camera {
                position: Vec3::ZERO,
                yaw: yaw + angle as f32 * std::f32::consts::PI / 8.0,
                pitch,
                fov: 55.0,
            };
            let eye = scene.target - candidate.forward() * scene.distance;
            if !blocked(vertices, eye, scene.target) {
                scene.yaw = candidate.yaw;
                scene.pitch = pitch;
                return true;
            }
        }
    }
    false
}
