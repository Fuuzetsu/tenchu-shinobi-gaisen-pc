use super::*;
use crate::{collision, ids::MOT_STATE_PICKUP, scene::CONFLICT};

impl Items {
    pub(crate) fn drop(
        &mut self,
        kind: usize,
        owner: Option<usize>,
        position: [i32; 3],
        velocity: [i32; 3],
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<bool> {
        let id = self.allocate(actors, ctx)?;
        if terrain::level(&ctx.mission.terrain, ctx.cursor, position, 0)? < position[1] {
            return Ok(false);
        }
        self.initialize(id, Processor::Pickup, kind, owner, position, ctx)?;
        let item = &mut self.slots[id];
        item.collision.size = 0;
        item.rolling = Rolling {
            hint: None,
            velocity: velocity.map(|v| v as i16),
            status: 0,
        };
        Ok(true)
    }

    pub(crate) fn stay(
        &mut self,
        kind: usize,
        mut position: [i32; 3],
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        position[1] = terrain::level(&ctx.mission.terrain, ctx.cursor, position, 0)?;
        self.drop(kind, None, position, [0; 3], actors, ctx)?;
        Ok(())
    }

    pub(super) fn pickup(
        &mut self,
        id: usize,
        ctx: &mut Context,
        actors: &mut Actors,
        halt: i16,
    ) -> Result<()> {
        self.draw_icon(id, ctx)?;
        let item = &mut self.slots[id];
        match item.mode {
            0 => {
                let mut position = ctx.scene.models[item.model].local.translation;
                item.rolling.step(&mut position, ctx)?;
                ctx.scene.set_translation(item.model, position);
                match item.rolling.status {
                    1 => self.dispose(id, actors, ctx)?,
                    2 | 4 => {
                        self.contact(id, 180, collision::SOFT, ctx)?;
                        self.slots[id].mode = 1;
                    }
                    _ => {}
                }
            }
            1 => {
                if ctx.scene.models[item.model].attribute & CONFLICT == 0 {
                    return Ok(());
                }
                let Some(slot) = ctx.collision.result(ctx.scene, item.model, None) else {
                    return Ok(());
                };
                let Some(actor_id) = ctx.collision.slots[slot]
                    .owner
                    .filter(|id| actors.roster.order.contains(id))
                else {
                    return Ok(());
                };
                let actor = actors.get_mut(actor_id);
                if actor.motion.action == MOT_STATE_PICKUP || !matches!(actor.status, 2 | 6) {
                    return Ok(());
                }
                if halt == 0 && actor.health > 0 {
                    actor.clear_attack(ctx, 3)?;
                    actor.enter(ctx, MOT_STATE_PICKUP, true);
                }
                item.owner = Some(actor_id);
                item.mode = 2;
                item.count = 0;
            }
            2 => {
                let actor = actors.get_mut(item.owner.ok_or("Pickup transfer has no owner")?);
                if actor.motion.action != MOT_STATE_PICKUP {
                    item.rolling = Rolling {
                        velocity: [
                            (ctx.random.next15() % 200 - 100) as i16,
                            (ctx.random.next15() % 100 - 200) as i16,
                            (ctx.random.next15() % 200 - 100) as i16,
                        ],
                        ..Rolling::default()
                    };
                    item.mode = 0;
                }
                item.count = item.count.wrapping_add(1);
                if item.count == 10 {
                    ctx.effects.sounds.push((actor.position, 0x0d));
                    if actor.inventory[item.kind] != 255 {
                        actor.inventory[item.kind] = actor.inventory[item.kind].wrapping_add(1);
                    }
                    self.dispose(id, actors, ctx)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
