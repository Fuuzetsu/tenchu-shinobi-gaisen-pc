use super::{
    Program,
    behavior::{Behavior, Events, Frame},
    combat::Context,
    direction,
};
use crate::{
    Result,
    tenchu2::{actor::State, terrain},
};

impl Behavior {
    pub(super) fn evade_shuriken(
        &mut self,
        c: &mut Context<'_>,
        frame: &mut Frame<'_, impl Events>,
    ) -> Result<bool> {
        let Some(aim_yaw) = frame.quick_shuriken_yaw else {
            return Ok(false);
        };
        if frame.script != 0
            || frame.observation.ignore_player
            || c.actor.target != Some(frame.observation.player)
            || c.actor.health <= 0
            || !c.actor.weapon_drawn()
            || c.actor.state != State::Ready
            || !matches!(c.actor.motion.action, 0x100..=0x104)
            || c.steering.forced
            || !matches!(
                self.programs.combat,
                Program::Combat | Program::Approach | Program::GuardHome | Program::Flank
            )
            || self
                .last_evasion
                .is_some_and(|tick| frame.tick.wrapping_sub(tick) < 30)
        {
            return Ok(false);
        }
        let player = frame.observation.body;
        let delta = c.actor.position.wrapping_sub(player.position);
        let math = &c.resources.math;
        if delta.x.wrapping_abs() >= 14000
            || delta.z.wrapping_abs() >= 14000
            || delta.y.wrapping_abs() > 2000
            || !(2500..14000).contains(&math.length(delta))
            || i32::from(direction(math, delta.x, delta.z, aim_yaw)?).abs() > 200
            || i32::from(direction(
                math,
                delta.x.wrapping_neg(),
                delta.z.wrapping_neg(),
                c.actor.rotation.y,
            )?)
            .abs()
                > 700
        {
            return Ok(false);
        }
        let available = |action| {
            c.resources
                .registry
                .clip(c.actor.definition_kind(), action)
                .is_some_and(|clip| c.resources.clips.contains_key(&clip))
        };
        let right = c.steering.right && available(0x113);
        let left = c.steering.left && available(0x114);
        if !right && !left {
            return Ok(false);
        }
        let mut eye = c.actor.position;
        eye.y = eye.y.wrapping_add(300 - i32::from(c.actor.height));
        let mut target = player.position;
        target.y = target.y.wrapping_sub(i32::from(player.height));
        let mut cursor = terrain::Cursor::default();
        if terrain::segment(
            frame.terrain,
            &mut cursor,
            eye,
            target.wrapping_sub(eye),
            math,
        )?
        .is_some()
        {
            return Ok(false);
        }
        let action = if right && (!left || c.random.next15() & 1 == 0) {
            0x113
        } else {
            0x114
        };
        if !frame.events.action(c.actor, action, c.resources)? {
            return Ok(false);
        }
        c.actor.pad = Default::default();
        self.last_evasion = Some(frame.tick);
        Ok(true)
    }
}
