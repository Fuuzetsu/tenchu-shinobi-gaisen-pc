use super::*;
use crate::ids::{ATTR_SUSPEND, MOT_ACTION, MOT_DAMAGE_CHOKE};

impl Items {
    pub(super) fn roll_icon(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<Option<[i32; 3]>> {
        let item = &mut self.slots[id];
        let model = item.model;
        let mut position = ctx.scene.models[model].local.translation;
        item.rolling.step(&mut position, ctx)?;
        ctx.scene.set_translation(model, position);
        if item.rolling.status == 1 {
            self.dispose(id, actors, ctx)?;
            return Ok(None);
        }
        self.update_coordinate(id, ctx);
        self.draw_icon(id, ctx)?;
        Ok(Some(position))
    }

    pub(super) fn smoke(
        &mut self,
        id: usize,
        actors: &mut Actors,
        intelligence: &mut crate::ai::Intelligence,
        clock: i32,
        ctx: &mut Context,
    ) -> Result<()> {
        let Some(position) = self.roll_icon(id, actors, ctx)? else {
            return Ok(());
        };
        let item = &mut self.slots[id];
        item.count = item.count.wrapping_sub(1);
        match item.mode {
            0 => {
                if item.count == 0 {
                    ctx.effects.sounds.push((position, 0x23));
                    item.count = 120;
                    item.mode = 1;
                }
            }
            1 => {
                if item.count == 0 {
                    return self.dispose(id, actors, ctx);
                }
                if item.count & 1 == 0 {
                    ctx.effects
                        .smoke(position, [0, -250, 0], 1, 3, ctx.random)?;
                }
                if clock & 15 != 0 {
                    return Ok(());
                }
                for target in actors.roster.order.clone() {
                    let actor = actors.get_mut(target);
                    if actor.health <= 0
                        || actor.motion.action == MOT_ACTION
                        || actor.attribute & ATTR_SUSPEND != 0
                    {
                        continue;
                    }
                    if ctx.math.vector_distance(std::array::from_fn(|i| {
                        position[i].wrapping_sub(actor.position[i])
                    })) >= 2000
                    {
                        continue;
                    }
                    if Some(target) == item.owner || actor.motion.action == MOT_DAMAGE_CHOKE {
                        continue;
                    }
                    if intelligence.action_halt == 0 {
                        actor.clear_attack(ctx, 3)?;
                        Self::item_motion(actor, MOT_DAMAGE_CHOKE, ctx);
                    }
                    intelligence.voices.push((target, 6));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
