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
    unobstructed_area(scene, vertices, 0.0)
}

pub fn unobstructed_area(scene: &mut Scene, vertices: &[Vertex], radius: f32) -> bool {
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
            if [Vec3::ZERO, Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                .into_iter()
                .all(|offset| !blocked(vertices, eye, scene.target + offset * radius))
            {
                scene.yaw = candidate.yaw;
                scene.pitch = pitch;
                return true;
            }
        }
    }
    false
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Path {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub end_eye: Option<[f32; 3]>,
    pub end_target: Option<[f32; 3]>,
    #[serde(default = "default_fov")]
    pub fov: f32,
}

fn default_fov() -> f32 {
    55.0
}

impl Path {
    pub fn validate(&self) -> tenchu_assets::Result<()> {
        let points = [
            self.eye,
            self.target,
            self.end_eye.unwrap_or(self.eye),
            self.end_target.unwrap_or(self.target),
        ]
        .map(Vec3::from);
        if !self.fov.is_finite()
            || !(10.0..=120.0).contains(&self.fov)
            || points
                .iter()
                .any(|p| !p.is_finite() || p.abs().max_element() > 320_000.0)
        {
            return Err("Invalid camera position or field of view".into());
        }
        let start = points[1] - points[0];
        let change = points[3] - points[2] - start;
        let t = (-start.dot(change) / change.length_squared().max(0.0001)).clamp(0.0, 1.0);
        if (start + change * t).length() < 50.0 {
            return Err("Camera path crosses its target".into());
        }
        Ok(())
    }

    pub fn sample(&self, progress: f32) -> Camera {
        let p = progress.clamp(0.0, 1.0);
        let p = p * p * (3.0 - 2.0 * p);
        let position = Vec3::from(self.eye).lerp(Vec3::from(self.end_eye.unwrap_or(self.eye)), p);
        let target =
            Vec3::from(self.target).lerp(Vec3::from(self.end_target.unwrap_or(self.target)), p);
        let direction = (target - position).normalize();
        Camera {
            position,
            yaw: direction.x.atan2(direction.z),
            pitch: (-direction.y).asin(),
            fov: self.fov,
        }
    }
}
