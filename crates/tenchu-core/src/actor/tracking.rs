use super::*;
use crate::roster::Roster;
use tenchu_assets::weapon::kind::*;

struct Target {
    model: ModelId,
    direction: i16,
    distance: i32,
}

impl Actor {
    pub(super) fn assisted_melee(&self, ctx: &Context, action: i16) -> bool {
        matches!(action, MOT_ATTACK..=MOT_ATTACK_LUNGE)
            && !matches!(
                self.motion_id(ctx, action),
                0xaa | 0xab | 0xac | 0xf5 | 0xe9 | 0x1a4
            )
            && (self.unarmed_action(action)
                || !matches!(
                    ctx.mission.characters[self.definition].weapon,
                    0 | JAW | SEVEN | TEPPO | GUN | YUMI | KATAYUMI
                ))
    }

    fn melee_target(&self, ctx: &Context, other: &Actor) -> Result<Option<Target>> {
        if other.id == self.id
            || self.map.height != 0
            || other.health <= 0
            || other.status == State::Dead
            || other.attribute & ATTR_SUSPEND != 0
            || other.kind & 0xf0 == 0x90
            || other.scripted
        {
            return Ok(None);
        }
        let delta = other.position.wrapping_sub(self.position);
        if delta.x.unsigned_abs() >= 3000
            || delta.z.unsigned_abs() >= 3000
            || delta.y.unsigned_abs() > 1200
        {
            return Ok(None);
        }
        let distance = ctx.math.length_wide(Position::new(delta.x, 0, delta.z));
        let direction = ctx.math.direction(delta.x, delta.z, self.rotation[1])?;
        if distance >= 3000 || direction.unsigned_abs() > 4096 / 6 {
            return Ok(None);
        }
        let mut from = self.position;
        from.y -= i32::from(ctx.mission.characters[self.definition].height) / 2;
        let mut to = other.position;
        to.y -= i32::from(ctx.mission.characters[other.definition].height) / 2;
        let mut step = to.wrapping_sub(from);
        let mut count = 1;
        while step.iter().any(|v| v.unsigned_abs() > 125) {
            step = step.map(|v| v >> 1);
            count *= 2;
        }
        let mut cursor = *ctx.cursor;
        if terrain::passage(
            &ctx.mission.terrain,
            &mut cursor,
            from,
            step.map(|v| v as i16).to_array(),
            count + 1,
        )?
        .is_some()
        {
            return Ok(None);
        }
        Ok(Some(Target {
            model: other.root_model,
            direction,
            distance,
        }))
    }

    pub(super) fn select_melee_target(
        &self,
        ctx: &Context,
        roster: &Roster,
        retain: bool,
    ) -> Result<Option<ModelId>> {
        let mut best = None;
        let mut score = i32::MAX;
        for &id in &roster.order {
            let Some(other) = roster.actors[id].as_ref() else {
                continue;
            };
            if let Some(target) = self.melee_target(ctx, other)? {
                if retain && self.target == Some(target.model) {
                    return Ok(Some(target.model));
                }
                let priority = target.distance + 3 * i32::from(target.direction).abs();
                if priority < score {
                    best = Some(target.model);
                    score = priority;
                }
            }
        }
        Ok(best)
    }

    pub(super) fn steer_melee_target(&mut self, ctx: &Context, roster: &Roster) -> Result<()> {
        let target = roster
            .order
            .iter()
            .filter_map(|&id| roster.actors[id].as_ref())
            .find(|a| Some(a.root_model) == self.target)
            .map(|a| self.melee_target(ctx, a))
            .transpose()?
            .flatten();
        if let Some(target) = target {
            let turn = target.direction.clamp(-136, 136);
            if turn != 0 {
                self.rotation[1] = self.rotation[1].wrapping_add(turn);
                self.motion_velocity(ctx);
            }
        } else {
            self.target = None;
        }
        Ok(())
    }
}
