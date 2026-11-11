use crate::{debug::camera::Camera, settings::Navigation, visuals::surfaces::Surfaces};
use glam::{Vec2, Vec3};
use tenchu_core::input;

mod gaisen;
pub use gaisen::viewpoint;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct NativeInput {
    pub yaw: f32,
    pub pitch: i16,
    pub sticks: [Vec2; 2],
    pub deadzone: u16,
    pub align_aim: bool,
}

#[derive(Clone, Copy)]
pub struct Target {
    pub id: usize,
    pub root: usize,
    pub position: Vec3,
    pub yaw: i16,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Orbit {
    pub looking: bool,
    pub yaw: f32,
    pitch: f32,
    previous: Vec3,
    center: Vec3,
    tick: Option<u64>,
    subject: Option<(usize, usize)>,
    distance: f32,
}

pub fn stick(value: Vec2, deadzone: u16) -> Vec2 {
    let deadzone = f32::from(deadzone.clamp(5, 40)) / 100.0;
    let length = value.length();
    if !length.is_finite() || length <= deadzone {
        Vec2::ZERO
    } else {
        value / length * ((length - deadzone) / (1.0 - deadzone)).min(1.0)
    }
}

pub(crate) fn directions(v: Vec2) -> u16 {
    let mut bits = 0;
    for (value, positive, negative) in [
        (v.x, input::RIGHT, input::LEFT),
        (v.y, input::UP, input::DOWN),
    ] {
        if value > 0.35 {
            bits |= positive;
        }
        if value < -0.35 {
            bits |= negative;
        }
    }
    bits
}

pub(crate) fn movement_vector(buttons: u16, analog: Vec2, deadzone: u16) -> Vec2 {
    let digital = Vec2::new(
        (i32::from(buttons & input::RIGHT != 0) - i32::from(buttons & input::LEFT != 0)) as f32,
        (i32::from(buttons & input::UP != 0) - i32::from(buttons & input::DOWN != 0)) as f32,
    );
    if digital != Vec2::ZERO {
        digital.normalize()
    } else {
        stick(analog, deadzone)
    }
}

pub(crate) fn heading(yaw: f32) -> i16 {
    ((yaw + std::f32::consts::PI) * 4096.0 / std::f32::consts::TAU).round() as i16
}

pub(crate) fn relative_movement(
    yaw: f32,
    buttons: u16,
    v: Vec2,
    facing: i16,
    steer: bool,
) -> (u16, Option<i16>) {
    let buttons = buttons & !(input::UP | input::DOWN | input::LEFT | input::RIGHT);
    if v == Vec2::ZERO {
        return (buttons, None);
    }
    let (sin, cos) = yaw.sin_cos();
    let world = Vec2::new(cos, -sin) * v.x + Vec2::new(sin, cos) * v.y;
    if steer && buttons & input::SQUARE == 0 {
        let yaw = (-world.x).atan2(-world.y) * 4096.0 / std::f32::consts::TAU;
        (buttons | input::UP, Some(yaw.round() as i16))
    } else {
        let yaw = f32::from(facing) * std::f32::consts::TAU / 4096.0;
        let forward = Vec2::new(-yaw.sin(), -yaw.cos());
        let local = Vec2::new(
            world.dot(Vec2::new(forward.y, -forward.x)),
            world.dot(forward),
        );
        (buttons | directions(local.normalize_or_zero()), None)
    }
}

impl Orbit {
    pub fn observe(&mut self, tick: u64, target: impl Into<Target>) {
        let target = target.into();
        let subject = (target.id, target.root);
        let center = target.position - Vec3::Y * 1100.0;
        if self.subject != Some(subject)
            || self.tick.is_none_or(|previous| tick < previous)
            || center.distance_squared(self.center) > 20_000.0_f32.powi(2)
        {
            self.recenter(target);
            self.previous = center;
            self.center = center;
            self.distance = 0.0;
        } else if self.tick != Some(tick) {
            self.previous = self.center;
            self.center = center;
        }
        self.subject = Some(subject);
        self.tick = Some(tick);
    }

    pub fn recenter(&mut self, target: impl Into<Target>) {
        self.yaw =
            f32::from(target.into().yaw) * std::f32::consts::TAU / 4096.0 + std::f32::consts::PI;
        self.pitch = -0.22;
    }

    fn look(&mut self, delta: Vec2, settings: &Navigation) {
        self.yaw = (self.yaw + delta.x).rem_euclid(std::f32::consts::TAU);
        self.pitch =
            (self.pitch + delta.y * if settings.invert_y { -1.0 } else { 1.0 }).clamp(-1.15, 0.65);
    }

    pub fn mouse(&mut self, dx: f64, dy: f64, settings: &Navigation) {
        self.look(
            Vec2::new(dx as f32, -dy as f32)
                * (0.0015 * f32::from(settings.mouse_sensitivity) / 100.0),
            settings,
        );
    }

    pub fn analog_look(&mut self, value: Vec2, elapsed: f32, settings: &Navigation) {
        let v = stick(value, settings.deadzone);
        self.look(
            v * v.length().sqrt()
                * Vec2::new(3.0, 2.0)
                * elapsed.clamp(0.0, 0.1)
                * f32::from(settings.stick_sensitivity)
                / 100.0,
            settings,
        );
    }

    pub fn aim_pitch(&self) -> i16 {
        (-self.pitch * 4096.0 / std::f32::consts::TAU)
            .round()
            .clamp(-910.0, 910.0) as i16
    }

    pub fn camera(
        &mut self,
        alpha: f32,
        elapsed: f32,
        settings: &Navigation,
        surfaces: &Surfaces,
    ) -> Camera {
        let target = self.previous.lerp(self.center, alpha.clamp(0.0, 1.0));
        self.camera_at(target, elapsed, settings, surfaces)
    }

    pub fn camera_at(
        &mut self,
        target: Vec3,
        elapsed: f32,
        settings: &Navigation,
        surfaces: &Surfaces,
    ) -> Camera {
        let mut camera = Camera {
            position: target,
            yaw: self.yaw,
            pitch: self.pitch,
            fov: 55.0,
        };
        let forward = camera.forward();
        let right = forward.cross(Vec3::NEG_Y).normalize();
        let up = right.cross(forward);
        let desired = f32::from(settings.distance) * 100.0;
        let end = target - forward * desired;
        let mut distance = desired;
        for offset in [
            Vec3::ZERO,
            right * 100.0,
            right * -100.0,
            up * 100.0,
            up * -100.0,
        ] {
            if let Some(hit) =
                surfaces.cast_excluding(target + offset, end + offset, crate::render_mesh::WATER)
            {
                distance = distance.min((hit.fraction * desired - 140.0).max(50.0));
            }
        }
        if self.distance == 0.0 || distance < self.distance {
            self.distance = distance;
        } else {
            self.distance +=
                (distance - self.distance) * (1.0 - (-10.0 * elapsed.clamp(0.0, 0.1)).exp());
        }
        camera.position = target - forward * self.distance;
        camera
    }
}

#[cfg(test)]
mod tests;
