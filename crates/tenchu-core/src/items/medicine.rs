use super::*;
use crate::ids::MOT_ITEM_DRINK;
use crate::math::Position;

impl Items {
    fn recovery_particles(owner: &Actor, color: [u8; 3], ctx: &mut Context) {
        for _ in 0..20 {
            let base = ctx.scene.models[owner.root_model].local.translation;
            let position = [
                base[0].wrapping_add(ctx.random.next15() % 1000 - 500),
                base[1].wrapping_add(ctx.random.next15() % 1000 - 1200),
                base[2].wrapping_add(ctx.random.next15() % 1000 - 500),
            ];
            let velocity = [0, (ctx.random.next15() % 10 - 30) as i16, 0];
            let time = (ctx.random.next15() % 16 + 15) as u8;
            ctx.effects.bleed(position.into(), velocity, time, color);
        }
    }

    pub(super) fn resurrection(
        &mut self,
        id: usize,
        used: Used<state::Resurrection>,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        match used.phase {
            state::Resurrection::Begin => {
                item.count = 0;
                item.behavior =
                    Behavior::Resurrection(used.with_phase(state::Resurrection::Particles));
            }
            state::Resurrection::Particles => {
                Self::recovery_particles(actors.get(used.owner), [100, 200, 220], ctx);
                item.count = item.count.wrapping_add(1);
                if item.count >= 31 {
                    item.behavior =
                        Behavior::Resurrection(used.with_phase(state::Resurrection::Finished));
                }
            }
            state::Resurrection::Finished => self.dispose(id, actors, ctx)?,
        }
        Ok(())
    }

    pub(super) fn medicine(
        &mut self,
        id: usize,
        used: Used<state::Medicine>,
        actors: &mut Actors,
        halt: crate::ai::Halt,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = used.owner;
        let owner = actors.get_mut(owner_id);
        match used.phase {
            state::Medicine::Begin => {
                if halt == crate::ai::Halt::Running && owner.health > 0 {
                    owner.clear_attack(ctx, 3)?;
                    Self::item_motion(owner, MOT_ITEM_DRINK, ctx);
                }
                let model = &mut ctx.scene.models[item.model];
                model.parent = Some(owner.parts[if owner.parts.len() > 14 { 14 } else { 2 }]);
                model.local.translation = Position::new(0, 50, 0);
                self.slots[id].behavior =
                    Behavior::Medicine(used.with_phase(state::Medicine::Drinking));
            }
            state::Medicine::Drinking => {
                if owner.motion.action != MOT_ITEM_DRINK {
                    return self.refund_cast(id, actors, ctx);
                }
                if owner.motion.count == 55 {
                    self.slots[id].behavior =
                        Behavior::Medicine(used.with_phase(state::Medicine::Recover));
                    return Ok(());
                }
                if owner.motion.count < 4 {
                    return Ok(());
                }
                let kind = item.kind;
                self.update_coordinate(id, ctx);
                self.icon_scale[kind.index()] = 8192;
                self.draw_icon(id, ctx)?;
            }
            state::Medicine::Recover => {
                owner.health = owner.max_health;
                Self::recovery_particles(owner, [255, 255, 126], ctx);
                ctx.effects.sounds.push((owner.position, 0x24));
                self.dispose(id, actors, ctx)?;
            }
        }
        Ok(())
    }
}
