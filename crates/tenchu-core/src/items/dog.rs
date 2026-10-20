use super::*;
use crate::{ids::*, scene::COLLIDE};

#[derive(Clone, Debug, Default)]
pub struct Dog {
    pub slave: Option<usize>,
    pub remaining: u16,
    pub player: usize,
}

impl Items {
    pub(super) fn finish_dog(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let dog = &self.slots[id].dog;
        if let Some(slave) = dog.slave {
            if actors.roster.actors.get(slave).is_none_or(Option::is_none)
                && actors.active.as_ref().is_none_or(|a| a.id != slave)
            {
                return Err("Summoned dog was removed before item disposal".into());
            }
            let actor = actors.get_mut(slave);
            let action = actor.return_normal(ctx, dog.player)?;
            actor.enter(ctx, action, true);
            actor.attribute |= ATTR_SUSPEND;
            actor.position = [999000; 3];
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
        }
        Ok(())
    }

    pub(super) fn dog(
        &mut self,
        id: usize,
        actors: &mut Actors,
        clock: i32,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        match item.mode {
            0 => {
                let mut position = ctx.scene.models[item.model].local.translation;
                item.rolling.step(&mut position, ctx)?;
                ctx.scene.set_translation(item.model, position);
                match item.rolling.status {
                    1 => return self.dispose(id, actors, ctx),
                    4 => {
                        let kind = item.kind;
                        self.dispose(id, actors, ctx)?;
                        return self.stay(kind, position, actors, ctx);
                    }
                    _ => {}
                }
                item.dog.remaining = item.dog.remaining.wrapping_sub(1);
                if item.dog.remaining as i16 <= 0 {
                    item.mode = 1;
                }
                self.update_coordinate(id, ctx);
                self.draw_icon(id, ctx)?;
            }
            1 => {
                if self
                    .dog_actor
                    .is_none_or(|a| !actors.roster.order.contains(&a))
                    || actors.roster.find(0xa9).is_none()
                {
                    let slave = actors.roster.spawn(0xa9, [999000; 3], 0, ctx)?;
                    actors.get_mut(slave).attribute |= ATTR_SUSPEND;
                    self.dog_actor = Some(slave);
                }
                let slave = self.dog_actor.unwrap();
                let mut position = ctx.scene.models[item.model].local.translation;
                let mut probe = position;
                probe[1] = probe[1].wrapping_sub(2000);
                let map = terrain::vector(&ctx.mission.terrain, ctx.cursor, probe, 500, 0)?;
                if map.level < position[1].wrapping_sub(500)
                    || actors.get(slave).attribute & ATTR_SUSPEND == 0
                {
                    item.mode = 0;
                    item.dog.remaining = 15;
                    return Ok(());
                }
                position[1] = position[1].min(map.level);
                ctx.effects
                    .smoke(position, [0, -50, 0], 10, 6, ctx.random)?;
                ctx.effects.sounds.push((position, 0x23));
                item.dog.slave = Some(slave);
                let owner = actors.get(item.owner.ok_or("Dog summon has no owner")?);
                let (rotation, target) = (owner.rotation, owner.root_model);
                let actor = actors.get_mut(slave);
                actor.status = 0;
                actor.health = actor.max_health;
                actor.position = position;
                actor.rotation = rotation;
                ctx.scene.set_translation(actor.root_model, position);
                actor.equip(ctx, false);
                actor.enter(ctx, MOT_STATE_SHEATHE, true);
                actor.attribute = 0;
                actor.target = Some(target);
                actor.motion.count = 0;
                actor.play_pose(ctx)?;
                ctx.scene.models[actor.parts[0]].attribute |= COLLIDE;
                actor.hide_body(ctx.scene, false);
                actor.velocity[1] = 0;
                item.mode = 2;
                item.dog.remaining = 1800;
            }
            2 => {
                let slave = item.dog.slave.ok_or("Active dog summon has no actor")?;
                if !actors.roster.order.contains(&slave) {
                    self.dispose(id, actors, ctx)?;
                }
                let item = &mut self.slots[id];
                item.dog.remaining = item.dog.remaining.wrapping_sub(1);
                let actor = actors.get(slave);
                if item.dog.remaining as i16 <= 0
                    || actor.health <= 0
                    || actor.attribute & ATTR_SUSPEND != 0
                {
                    let position = ctx.scene.models[actor.root_model].local.translation;
                    ctx.effects
                        .smoke(position, [0, -50, 0], 10, 6, ctx.random)?;
                    ctx.effects.sounds.push((position, 0x23));
                    self.spill_inventory(slave, actors, ctx)?;
                    return self.dispose(id, actors, ctx);
                }
                if clock % 15 != 0 || matches!(actor.status, 7 | 8 | 16) {
                    return Ok(());
                }
                let owner_id = item.owner.ok_or("Dog summon has no owner")?;
                let attribute = actors.get(owner_id).attribute;
                actors.get_mut(owner_id).attribute = ATTR_SUSPEND;
                let nearest = actors.roster.nearest(actors.get(slave), 10000, ctx.math);
                actors.get_mut(owner_id).attribute = attribute;
                if let Some(nearest) = nearest {
                    let target = actors.get(nearest).root_model;
                    let actor = actors.get_mut(slave);
                    if actor.target != Some(target) {
                        actor.setup_think(0x5449);
                        actor.target = Some(target);
                        actor.attribute |= 2;
                        actor.equip(ctx, true);
                        actor.enter(ctx, MOT_STATE_DRAW, true);
                    }
                } else {
                    let target = actors.get(owner_id).root_model;
                    let actor = actors.get_mut(slave);
                    if actor.target != Some(item.model) {
                        actor.equip(ctx, false);
                        actor.enter(ctx, MOT_STATE_SHEATHE, true);
                        actor.attribute &= !3;
                        actor.setup_think(0);
                        actor.target = Some(target);
                    }
                }
            }
            mode => return Err(format!("Invalid dog summon mode {mode}")),
        }
        Ok(())
    }
}
