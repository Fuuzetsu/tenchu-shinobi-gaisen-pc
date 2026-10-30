use crate::{debug::camera::Camera, settings::Navigation, visuals::surfaces::Surfaces};
use glam::{Vec2, Vec3};
use tenchu_core::{game::Game, input, math::Position};

#[derive(Default)]
pub struct Orbit {
    pub looking: bool,
    pub yaw: f32,
    pitch: f32,
    previous: Vec3,
    center: Vec3,
    tick: Option<u64>,
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

fn directions(v: Vec2) -> u16 {
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

impl Orbit {
    pub fn observe(&mut self, game: &Game) {
        let center =
            Vec3::from(game.player().position.to_array().map(|v| v as f32)) - Vec3::Y * 1100.0;
        if self.tick.is_none_or(|tick| game.tick < tick)
            || center.distance_squared(self.center) > 20_000.0_f32.powi(2)
        {
            self.recenter(game);
            self.previous = center;
            self.center = center;
            self.distance = 0.0;
        } else if self.tick != Some(game.tick) {
            self.previous = self.center;
            self.center = center;
        }
        self.tick = Some(game.tick);
    }

    pub fn recenter(&mut self, game: &Game) {
        self.yaw = f32::from(game.player().rotation[1]) * std::f32::consts::TAU / 4096.0
            + std::f32::consts::PI;
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

    pub fn movement(
        &self,
        game: &Game,
        buttons: u16,
        analog: Vec2,
        settings: &Navigation,
    ) -> (u16, Option<i16>) {
        let digital = Vec2::new(
            (i32::from(buttons & input::RIGHT != 0) - i32::from(buttons & input::LEFT != 0)) as f32,
            (i32::from(buttons & input::UP != 0) - i32::from(buttons & input::DOWN != 0)) as f32,
        );
        let v = if digital != Vec2::ZERO {
            digital.normalize()
        } else {
            stick(analog, settings.deadzone)
        };
        if buttons & input::L1 != 0 || !game.player_can_orbit() {
            return (buttons | directions(v), None);
        }
        if buttons & input::R1 != 0 {
            return ((buttons & !input::R1) | directions(v), None);
        }
        let buttons = buttons & !(input::UP | input::DOWN | input::LEFT | input::RIGHT);
        if v == Vec2::ZERO {
            return (buttons, None);
        }
        let (sin, cos) = self.yaw.sin_cos();
        let world = Vec2::new(cos, -sin) * v.x + Vec2::new(sin, cos) * v.y;
        if game.player_can_steer() {
            let yaw = (-world.x).atan2(-world.y) * 4096.0 / std::f32::consts::TAU;
            (buttons | input::UP, Some(yaw.round() as i16))
        } else {
            let yaw = f32::from(game.player().rotation[1]) * std::f32::consts::TAU / 4096.0;
            let forward = Vec2::new(-yaw.sin(), -yaw.cos());
            let local = Vec2::new(
                world.dot(Vec2::new(forward.y, -forward.x)),
                world.dot(forward),
            );
            (buttons | directions(local.normalize_or_zero()), None)
        }
    }

    pub fn camera(
        &mut self,
        alpha: f32,
        elapsed: f32,
        settings: &Navigation,
        surfaces: &Surfaces,
    ) -> Camera {
        let target = self.previous.lerp(self.center, alpha.clamp(0.0, 1.0));
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

pub fn viewpoint(camera: Camera) -> tenchu_core::game::navigation::Viewpoint {
    let position = |v: Vec3| Position::from(v.to_array().map(|v| v.round() as i32));
    tenchu_core::game::navigation::Viewpoint {
        position: position(camera.position),
        target: position(camera.position + camera.forward() * 2000.0),
    }
}

#[cfg(test)]
mod tests;
