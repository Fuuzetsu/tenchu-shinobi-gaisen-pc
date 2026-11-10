use super::{Assets, World};
use crate::{
    Result,
    tenchu2::{
        actor::{State, Velocity},
        physics,
    },
};

impl World {
    pub fn player_can_orbit(&self) -> bool {
        let actor = &self.actors[self.player].actor;
        self.mission.input_allowed()
            && self.script != 3
            && actor.health > 0
            && !matches!(
                actor.state,
                State::Wall | State::Ledge | State::Item | State::Dead
            )
            && !matches!(actor.motion.action, 0x514 | 0x515 | 0x521..=0x537 | 0x920..=0x92f)
            && !matches!(self.camera.mode, 6 | 19 | 20)
    }

    pub fn player_can_steer(&self) -> bool {
        let actor = &self.actors[self.player].actor;
        self.player_can_orbit()
            && actor.flags & (physics::HIT | physics::FALLING) == 0
            && matches!(actor.motion.action, 0..=4 | 0x100..=0x104 | 0x200..=0x204 | 0x500 | 0x501 | 0x511 | 0x512)
    }

    pub fn steer_player(&mut self, heading: i16, assets: &Assets) -> Result<()> {
        if !self.player_can_steer() {
            return Ok(());
        }
        let actor = &mut self.actors[self.player].actor;
        let delta = heading.wrapping_sub(actor.rotation.y);
        if delta == 0 {
            return Ok(());
        }
        actor.rotation.y = heading;
        if (actor.velocity.x != 0 || actor.velocity.z != 0)
            && matches!(actor.motion.action, 3 | 0x103 | 0x201 | 0x500 | 0x511)
        {
            actor.update_velocity(Velocity::Motion, &assets.resources)?;
        } else {
            let [sin, cos] = assets.resources.math.pair(delta);
            let [x, _, z] = actor.velocity.map(i32::from).to_array();
            actor.velocity.x = ((x * cos + z * sin) >> 12) as i16;
            actor.velocity.z = ((z * cos - x * sin) >> 12) as i16;
        }
        self.publish(self.player, assets);
        Ok(())
    }
}
