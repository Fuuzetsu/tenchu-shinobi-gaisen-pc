use super::*;
use tenchu_core::{game::Game, math::Position};

impl From<&tenchu_core::actor::Actor> for Target {
    fn from(actor: &tenchu_core::actor::Actor) -> Self {
        Self {
            id: actor.id,
            root: actor.root_model,
            position: Vec3::from(actor.position.to_array().map(|v| v as f32)),
            yaw: actor.rotation[1],
        }
    }
}

impl Orbit {
    pub fn movement(
        &self,
        game: &Game,
        buttons: u16,
        analog: Vec2,
        settings: &Navigation,
    ) -> (u16, Option<i16>) {
        let pressed = buttons & !game.player().pad.held;
        let aim_item = matches!(
            game.hud.selected,
            Some(
                tenchu_core::items::Kind::Grapple
                    | tenchu_core::items::Kind::Shuriken
                    | tenchu_core::items::Kind::Teleport
            )
        );
        if settings.align_aim
            && game.player_can_steer()
            && game.player_can_orbit()
            && (pressed & input::L1 != 0 || pressed & input::TRIANGLE != 0 && aim_item)
        {
            let heading = (self.yaw + std::f32::consts::PI) * 4096.0 / std::f32::consts::TAU;
            return (buttons, Some(heading.round() as i16));
        }
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
            let buttons = if game.intelligence.guard_controls {
                buttons
            } else {
                buttons & !input::R1
            };
            return (buttons | directions(v), None);
        }
        let buttons = buttons & !(input::UP | input::DOWN | input::LEFT | input::RIGHT);
        if v == Vec2::ZERO {
            return (buttons, None);
        }
        let (sin, cos) = self.yaw.sin_cos();
        let world = Vec2::new(cos, -sin) * v.x + Vec2::new(sin, cos) * v.y;
        if game.player_can_steer() && buttons & input::SQUARE == 0 {
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
}

pub fn viewpoint(camera: Camera) -> tenchu_core::game::navigation::Viewpoint {
    let position = |v: Vec3| Position::from(v.to_array().map(|v| v.round() as i32));
    tenchu_core::game::navigation::Viewpoint {
        position: position(camera.position),
        target: position(camera.position + camera.forward() * 2000.0),
    }
}
