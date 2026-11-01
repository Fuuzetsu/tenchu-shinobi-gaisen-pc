use super::*;

impl Controller<'_, '_> {
    pub(super) fn evade_shuriken(&mut self) -> Result<bool> {
        if !self.state.quick_shuriken_dodge
            || self.state.ignore_player
            || self.actor.scripted
            || !self.player_target()
            || self.player.status != ActorState::AimShuriken
            || self.actor.kind >= 0xa0
            || self.actor.health <= 0
            || matches!(
                self.actor.think.combat,
                programs::Combat::None | programs::Combat::Disabled | programs::Combat::Player
            )
            || self.attribute & ATTR_WEAPON_DRAWN == 0
            || self.actor.evasion_clock.wrapping_sub(self.clock) > 0
            || !matches!(self.actor.status, ActorState::Engage | ActorState::Chase)
            || !matches!(
                self.actor.motion.action,
                MOT_ENGAGE_STANCE
                    | MOT_ENGAGE_TURN_L
                    | MOT_ENGAGE_TURN_R
                    | MOT_CHASE
                    | MOT_CHASE_BACK
            )
            || !(2500..14000).contains(&self.distance())
            || self.degree().abs() > 700
        {
            return Ok(false);
        }
        let delta = self.actor.position.wrapping_sub(self.player.position);
        if delta.y.abs() > 2000
            || self
                .ctx
                .math
                .direction(delta.x, delta.z, self.player.facing)?
                .abs()
                > 200
        {
            return Ok(false);
        }
        let mut from = self.actor.position;
        from.y += 300 - i32::from(self.ctx.mission.characters[self.actor.definition].height);
        let mut step = self.player.position.wrapping_sub(from);
        step.y -= i32::from(self.player.height);
        let mut count = 1;
        while step.iter().any(|v| v.abs() > 500) {
            count *= 2;
            step = step.map(|v| v >> 1);
        }
        if terrain::passage(
            &self.ctx.mission.terrain,
            self.ctx.cursor,
            from,
            step.map(|v| v as i16).to_array(),
            count,
        )?
        .is_some()
        {
            return Ok(false);
        }
        let actions = if self.random() & 1 == 0 {
            [MOT_CHASE_DASH_LEFT, MOT_CHASE_DASH_RIGHT]
        } else {
            [MOT_CHASE_DASH_RIGHT, MOT_CHASE_DASH_LEFT]
        };
        if actions.into_iter().any(|action| self.motion(action)) {
            self.actor.pad = Default::default();
            self.actor.evasion_clock = self.clock.wrapping_add(30);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
