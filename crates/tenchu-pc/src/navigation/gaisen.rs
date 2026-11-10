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
            return (buttons, Some(heading(self.yaw)));
        }
        let v = movement_vector(buttons, analog, settings.deadzone);
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
        relative_movement(
            self.yaw,
            buttons,
            v,
            game.player().rotation[1],
            game.player_can_steer(),
        )
    }
}

pub fn viewpoint(camera: Camera) -> tenchu_core::game::navigation::Viewpoint {
    let position = |v: Vec3| Position::from(v.to_array().map(|v| v.round() as i32));
    tenchu_core::game::navigation::Viewpoint {
        position: position(camera.position),
        target: position(camera.position + camera.forward() * 2000.0),
    }
}
