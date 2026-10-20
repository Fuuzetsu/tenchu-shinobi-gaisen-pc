use crate::controls::Controls;
use glam::{Mat4, Vec3};
use winit::keyboard::KeyCode;

#[derive(Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
}

impl Camera {
    pub fn from_game(game: &tenchu_core::game::Game) -> Self {
        let inverse = crate::renderer::view_matrix(game.rendered_view).inverse();
        let forward = inverse.transform_vector3(Vec3::NEG_Z).normalize();
        Self {
            position: inverse.transform_point3(Vec3::ZERO),
            yaw: forward.x.atan2(forward.z),
            pitch: (-forward.y).asin(),
            fov: (2.0 * (120.0 / f32::from(tenchu_core::projection::DISTANCE)).atan()).to_degrees(),
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            -self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, 50.0, 250_000.0)
            * Mat4::look_to_rh(self.position, self.forward(), Vec3::NEG_Y)
    }

    pub fn source_view(&self) -> tenchu_core::math::Matrix {
        let matrix = Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0))
            * Mat4::look_to_rh(self.position, self.forward(), Vec3::NEG_Y);
        let columns = matrix.to_cols_array_2d();
        tenchu_core::math::Matrix {
            translation: std::array::from_fn(|i| columns[3][i].round() as i32),
            rotation: std::array::from_fn(|r| {
                std::array::from_fn(|c| (columns[c][r] * 4096.0).round() as i16)
            }),
        }
    }

    pub fn look(&mut self, dx: f64, dy: f64) {
        self.yaw += dx as f32 * 0.003;
        self.pitch = (self.pitch - dy as f32 * 0.003).clamp(-1.56, 1.56);
    }

    pub fn advance(&mut self, controls: &Controls, elapsed: f32, speed: f32) {
        let axis = |positive, negative| {
            i32::from(controls.held(positive)) - i32::from(controls.held(negative))
        };
        let forward = self.forward();
        let right = forward.cross(Vec3::NEG_Y).normalize();
        let movement = forward * axis(KeyCode::KeyW, KeyCode::KeyS) as f32
            + right * axis(KeyCode::KeyD, KeyCode::KeyA) as f32
            + Vec3::NEG_Y * axis(KeyCode::KeyE, KeyCode::KeyQ) as f32;
        let boost = if controls.held(KeyCode::ShiftLeft) || controls.held(KeyCode::ShiftRight) {
            4.0
        } else {
            1.0
        };
        self.position += movement.normalize_or_zero() * speed * elapsed.min(0.1) * boost;
        self.position = self
            .position
            .clamp(Vec3::splat(-320_000.0), Vec3::splat(320_000.0));
    }

    pub fn focus(&mut self, target: Vec3, overhead: bool) {
        self.pitch = if overhead { -1.55 } else { -0.3 };
        self.position = target - self.forward() * if overhead { 18000.0 } else { 6000.0 };
    }
}

pub fn ray(matrix: Mat4, ndc: [f32; 2]) -> (Vec3, Vec3) {
    let inverse = matrix.inverse();
    let near = inverse.project_point3(Vec3::new(ndc[0], ndc[1], 0.0));
    let far = inverse.project_point3(Vec3::new(ndc[0], ndc[1], 1.0));
    (near, (far - near).normalize())
}
