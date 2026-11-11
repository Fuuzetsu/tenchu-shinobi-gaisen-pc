use super::World;
use crate::{
    game::bleeding,
    tenchu2::actor::{Actor, State},
};

impl Actor {
    pub(crate) fn can_bandage(&self) -> bool {
        self.health > 0
            && matches!(self.state, State::Normal | State::Ready | State::Crouched)
            && matches!(self.motion.action, 0..=4 | 0x100..=0x104 | 0x110 | 0x200..=0x204)
            && self.terrain.floor.is_some()
            && self.terrain.height <= i32::from(self.height)
            && self.flags & (8 | crate::tenchu2::physics::FALLING) == 0
    }

    pub(crate) fn use_bandage(&mut self) -> bool {
        self.can_bandage()
            && self
                .bleeding
                .as_mut()
                .is_some_and(bleeding::State::use_bandage)
    }
}

impl World {
    pub fn set_bleeding(&mut self, settings: bleeding::Settings) {
        bleeding::State::configure(
            &mut self.actors[self.player].actor.bleeding,
            settings,
            self.random.seed,
        );
        self.sync_bandages();
    }

    pub(super) fn sync_bandages(&mut self) {
        let actor = &self.actors[self.player].actor;
        self.controls.hud.set_bandages(
            actor.bleeding.as_ref().map_or(0, |state| state.bandages),
            &actor.inventory,
        );
    }

    pub(super) fn bleeding_tick(&mut self) {
        let paused = !self.mission.input_allowed() || self.script != 0;
        let actor = &mut self.actors[self.player].actor;
        let can_treat = actor.can_bandage() && !matches!(self.camera.mode, 6 | 20);
        if let Some(state) = &mut actor.bleeding {
            state.tick(&mut actor.health, can_treat, paused, actor.invulnerable);
        }
        self.sync_bandages();
    }
}
