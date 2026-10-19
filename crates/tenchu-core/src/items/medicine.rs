use super::*;
use crate::ids::MOT_ITEM_DRINK;

impl Items {
    pub(super) fn medicine(
        &mut self,
        id: usize,
        actors: &mut Actors,
        halt: i16,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = item.owner.ok_or("Medicine has no owner")?;
        let owner = actors.get_mut(owner_id);
        match item.mode {
            0 => {
                if halt == 0 && owner.health > 0 {
                    owner.clear_attack(ctx, 3)?;
                    owner.motion.change(
                        MOT_ITEM_DRINK,
                        &ctx.mission.characters[owner.definition].motions,
                        &ctx.mission.common_motions,
                        &ctx.mission.clips,
                    );
                    owner.status = 15;
                    owner.motion_velocity(ctx);
                }
                let model = &mut ctx.scene.models[item.model];
                model.parent = Some(owner.parts[if owner.parts.len() > 14 { 14 } else { 2 }]);
                model.local.translation = [0, 50, 0];
                self.slots[id].mode = 1;
            }
            1 => {
                if owner.motion.action != MOT_ITEM_DRINK {
                    let position = ctx.scene.world(item.model)?.translation;
                    let kind = item.kind;
                    let velocity = [
                        ctx.random.next15() % 200 - 100,
                        ctx.random.next15() % 100 - 200,
                        ctx.random.next15() % 200 - 100,
                    ];
                    self.drop(kind, Some(owner_id), position, velocity, actors, ctx)?;
                    self.dispose(id, actors, ctx)?;
                    return Ok(());
                }
                if owner.motion.count == 55 {
                    self.slots[id].mode = 2;
                    return Ok(());
                }
                if owner.motion.count < 4 {
                    return Ok(());
                }
                let kind = item.kind;
                self.update_coordinate(id, ctx);
                self.icon_scale[kind] = 8192;
                self.draw_icon(id, ctx)?;
            }
            2 => {
                owner.health = owner.max_health;
                for _ in 0..20 {
                    let base = ctx.scene.models[owner.root_model].local.translation;
                    let position = [
                        base[0].wrapping_add(ctx.random.next15() % 1000 - 500),
                        base[1].wrapping_add(ctx.random.next15() % 1000 - 1200),
                        base[2].wrapping_add(ctx.random.next15() % 1000 - 500),
                    ];
                    let velocity = [0, (ctx.random.next15() % 10 - 30) as i16, 0];
                    let time = (ctx.random.next15() % 16 + 15) as u8;
                    ctx.effects.bleed(position, velocity, time, [255, 255, 126]);
                }
                ctx.effects.sounds.push((owner.position, 0x24));
                self.dispose(id, actors, ctx)?;
            }
            _ => {}
        }
        Ok(())
    }
}
