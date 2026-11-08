use super::{Assets, World};
use crate::{
    Result,
    math::Position,
    tenchu2::horizontal_distance as distance,
    tenchu2::{actor::Velocity, ai::direction, rig::WeaponMode},
};

impl World {
    pub(super) fn initialize_activation(&mut self, assets: &Assets) -> Result<()> {
        let origin = self.actors[self.player].actor.position;
        for index in 0..self.actors.len() {
            let actor = &mut self.actors[index].actor;
            if index == self.player
                || exempt(actor.kind)
                || distance(actor.position, origin, &assets.resources.math) <= self.view_distance
            {
                continue;
            }
            actor.flags |= 8;
            actor.set_body_collision(
                self.instances[index].rig.joints[0],
                index,
                &mut self.collision,
                false,
            )?;
        }
        Ok(())
    }

    /// The budget is a simulation input, never inferred from native rendering time.
    pub fn update_activation(
        &mut self,
        eye: Position,
        look: Position,
        budget: u8,
        assets: &Assets,
    ) -> Result<()> {
        if !(4..=8).contains(&budget) {
            return Err("Tenchu 2 activation budget must be 4–8".into());
        }
        if !self.tick.is_multiple_of(30) {
            return Ok(());
        }
        let view = self.view_distance;
        let far = view.wrapping_add(4000);
        let mut active = Vec::new();
        let mut inactive = Vec::new();
        for (index, agent) in self.actors.iter().enumerate() {
            if agent.actor.flags & 8 == 0 {
                active.push(index);
            } else {
                inactive.push(index);
            }
        }
        partition(&mut active, |id| self.actors[id].actor.health == 0);
        partition(&mut inactive, |id| self.actors[id].actor.health != 0);
        let mut excess = active.len() as i32 - i32::from(budget);
        let mut free = -excess;
        for index in active {
            let agent = &mut self.actors[index];
            let actor = &mut agent.actor;
            if index == self.controls.camera_actor
                || exempt(actor.kind)
                || actor.position.y.wrapping_sub(eye.y).wrapping_abs() >= view
            {
                continue;
            }
            let length = distance(actor.position, eye, &assets.resources.math);
            if length < far {
                if excess <= 0
                    || actor.flags & 0x100000 != 0
                    || !(length > 12000 || actor.health == 0 && actor.motion.loops == i16::MIN)
                {
                    continue;
                }
                excess -= 1;
            }
            if actor.health == 0 {
                agent.behavior.home = actor.position;
            } else if distance(agent.behavior.home, eye, &assets.resources.math) > far {
                actor.position = agent.behavior.home;
                actor.previous = actor.position;
                actor.change_motion(
                    if actor.weapon_drawn() { 0x100 } else { 0 },
                    Velocity::Motion,
                    &assets.resources,
                )?;
                agent.behavior.patrol.turning = 1;
                agent.behavior.patrol.next = 0;
            }
            actor.flags |= 8;
            let rig = &mut self.instances[index].rig;
            actor.set_weapon(
                rig,
                index,
                &mut self.collision,
                &assets.resources,
                WeaponMode::Remove,
            )?;
            actor.set_body_collision(rig.joints[0], index, &mut self.collision, false)?;
        }
        let yaw = if free > 0 {
            direction(
                &assets.resources.math,
                look.x.wrapping_sub(eye.x),
                look.z.wrapping_sub(eye.z),
                0,
            )?
        } else {
            0
        };
        for index in inactive {
            let actor = &mut self.actors[index].actor;
            if actor.position.y.wrapping_sub(eye.y).wrapping_abs() >= view {
                continue;
            }
            let length = distance(actor.position, eye, &assets.resources.math);
            if !(view..far).contains(&length) {
                if free <= 0 || actor.health == 0 || length <= 12000 {
                    continue;
                }
                let angle = direction(
                    &assets.resources.math,
                    actor.position.x.wrapping_sub(eye.x),
                    actor.position.z.wrapping_sub(eye.z),
                    yaw,
                )?;
                if i32::from(angle).abs() <= 1024 {
                    continue;
                }
                free -= 1;
            }
            actor.flags &= !8;
            let rig = &mut self.instances[index].rig;
            if actor.health > 0 {
                actor.set_weapon(
                    rig,
                    index,
                    &mut self.collision,
                    &assets.resources,
                    WeaponMode::Refresh,
                )?;
            }
            actor.set_body_collision(rig.joints[0], index, &mut self.collision, true)?;
            let playing = actor.motion.loops >= 0;
            actor
                .motion
                .advance(&assets.resources.clips, &assets.spline, true)?;
            if playing {
                actor
                    .motion
                    .publish(&mut self.scene, &rig.joints, &assets.resources.math);
            }
        }
        Ok(())
    }
}

fn exempt(kind: i16) -> bool {
    kind & 0x5e == 0x5e || kind & 0x80 != 0
}

fn partition(indices: &mut [usize], front: impl Fn(usize) -> bool) {
    let Some(mut right) = indices.len().checked_sub(1) else {
        return;
    };
    let mut left = 0;
    while left < right {
        while left < right && front(indices[left]) {
            left += 1;
        }
        while left < right && !front(indices[right]) {
            right -= 1;
        }
        if left < right {
            indices.swap(left, right);
        }
        left += 1;
        right = right.saturating_sub(1);
    }
}
