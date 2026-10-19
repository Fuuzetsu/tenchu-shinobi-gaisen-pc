use super::*;
use crate::ids::MOT_ITEM_KAENGEKI;

impl Items {
    pub(super) fn charm(
        &mut self,
        id: usize,
        actors: &mut Actors,
        player: usize,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = item.owner.ok_or("Protection charm has no owner")?;
        let owner = actors.get_mut(owner_id);
        match item.mode {
            0 => {
                owner.enter(ctx, MOT_ITEM_KAENGEKI, true);
                ctx.effects.sounds.push((owner.position, 0x4c));
                self.slots[id].mode = 1;
            }
            1 => {
                if owner.motion.action != MOT_ITEM_KAENGEKI {
                    let position = ctx.scene.world(item.model)?.translation;
                    let kind = item.kind;
                    let velocity = [
                        ctx.random.next15() % 200 - 100,
                        ctx.random.next15() % 100 - 200,
                        ctx.random.next15() % 200 - 100,
                    ];
                    self.drop(kind, Some(owner_id), position, velocity, actors, ctx)?;
                    return self.dispose(id, actors, ctx);
                }
                if owner.motion.count != 0 || owner.motion.loops == 0 {
                    return Ok(());
                }
                let action = owner.return_normal(ctx, player)?;
                owner.enter(ctx, action, true);
                let position = ctx.scene.world(owner.parts[1])?.translation;
                ctx.effects
                    .scatter_bleeds(position, 600, 100, 20, 15, [180, 140, 30], ctx.random);
                owner.active_item = item.kind as i16;
                self.slots[id].charm_remaining = 450;
                self.slots[id].mode = 2;
            }
            2 => {
                let remaining = item.charm_remaining.wrapping_sub(1);
                self.slots[id].charm_remaining = remaining;
                if remaining == 0 {
                    return self.dispose(id, actors, ctx);
                }
                if remaining & 63 == 0 {
                    let rotation = (ctx.random.next15() % 360) as i16;
                    ctx.effects.impact_ex(effects::Impact {
                        parent: Some(owner.root_model),
                        position: [0, -1200, 400],
                        size: [4096, 24576],
                        color: [[128; 3], [0; 3]],
                        rotation,
                        speed: 2,
                        time: 120,
                        count: 0,
                        sprite: 4,
                    });
                }
            }
            mode => return Err(format!("Invalid protection charm mode {mode}")),
        }
        Ok(())
    }
}
