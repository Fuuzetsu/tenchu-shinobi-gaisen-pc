use super::*;

struct Target {
    direction: i16,
    distance: i32,
}

impl<E: Events> Context<'_, E> {
    pub(super) fn assisted_melee(&self, actor: &Actor, resources: &Resources) -> bool {
        self.controls.attack_assist
            && self.index == self.player
            && self.enabled
            && self.input_allowed
            && *self.script == 0
            && !matches!(self.camera.mode, 6 | 20)
            && matches!(actor.motion.action, 0x900..=0x90c | 0x910)
            && actor.motion.clip.is_some_and(|clip| {
                resources.weapons.attack(clip).is_some_and(|a| a.bones != 0)
                    && actor.imported_attack().map_or_else(
                        || !matches!(clip, 0x205 | 700 | 0x418 | 0x510 | 0x176),
                        |attack| attack.contacts.iter().any(Option::is_some),
                    )
            })
    }

    fn melee_target(
        &self,
        actor: &Actor,
        index: usize,
        terrain: &Terrain<'_>,
        resources: &Resources,
    ) -> Result<Option<Target>> {
        let Some(other) = self.others.get(index, actor) else {
            return Ok(None);
        };
        if index == self.index
            || actor.flags & crate::tenchu2::physics::FALLING != 0
            || actor.terrain.floor.is_none()
            || actor.terrain.height > i32::from(actor.height)
            || other.health <= 0
            || other.state == State::Dead
            || other.flags & (8 | 0x10) != 0
            || matches!(other.kind & 0xf0, 0 | 0x10 | 0x60)
        {
            return Ok(None);
        }
        let delta = other.position.wrapping_sub(actor.position);
        if delta.x.unsigned_abs() >= 3000
            || delta.z.unsigned_abs() >= 3000
            || delta.y.unsigned_abs() > 1200
        {
            return Ok(None);
        }
        let distance = resources
            .math
            .length_wide(Position::new(delta.x, 0, delta.z));
        let direction =
            crate::tenchu2::ai::direction(&resources.math, delta.x, delta.z, actor.rotation.y)?;
        if distance >= 3000 || direction.unsigned_abs() > 4096 / 6 {
            return Ok(None);
        }
        let mut from = actor.position;
        from.y = from.y.wrapping_sub(i32::from(actor.height / 2));
        let mut to = other.position;
        to.y = to.y.wrapping_sub(i32::from(other.height / 2));
        let mut cursor = terrain.cursor.clone();
        if crate::tenchu2::terrain::segment(
            terrain.map,
            &mut cursor,
            from,
            to.wrapping_sub(from),
            &resources.math,
        )?
        .is_some()
        {
            return Ok(None);
        }
        Ok(Some(Target {
            direction,
            distance,
        }))
    }

    pub(super) fn select_melee_target(
        &self,
        actor: &Actor,
        terrain: &Terrain<'_>,
        resources: &Resources,
    ) -> Result<Option<usize>> {
        let mut best = None;
        let mut score = i32::MAX;
        for index in 0..self.others.len() {
            if let Some(target) = self.melee_target(actor, index, terrain, resources)? {
                if actor.target == Some(index) {
                    return Ok(Some(index));
                }
                let priority = target.distance + 3 * i32::from(target.direction).abs();
                if priority < score {
                    best = Some(index);
                    score = priority;
                }
            }
        }
        Ok(best)
    }

    pub(super) fn steer_melee_target(
        &self,
        actor: &mut Actor,
        terrain: &Terrain<'_>,
        resources: &Resources,
    ) -> Result<()> {
        let target = actor
            .target
            .map(|index| self.melee_target(actor, index, terrain, resources))
            .transpose()?
            .flatten();
        if let Some(target) = target {
            let turn = target.direction.clamp(-136, 136);
            if turn != 0 {
                actor.rotation.y = actor.rotation.y.wrapping_add(turn);
                actor.update_velocity(Velocity::Motion, resources)?;
            }
        } else {
            actor.target = None;
        }
        Ok(())
    }
}
