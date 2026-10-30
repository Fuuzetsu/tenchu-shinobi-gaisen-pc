use super::*;
use crate::ids::MOT_ITEM_KAENGEKI;
use crate::math::Position;

impl Items {
    pub(super) fn charm(
        &mut self,
        id: usize,
        used: Used<state::Charm>,
        actors: &mut Actors,
        player: usize,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = used.owner;
        let owner = actors.get_mut(owner_id);
        match used.phase {
            state::Charm::Begin => {
                owner.enter(ctx, MOT_ITEM_KAENGEKI, true);
                ctx.effects.sounds.push((owner.position, 0x4c));
                self.slots[id].behavior = Behavior::Charm(used.with_phase(state::Charm::Casting));
            }
            state::Charm::Casting => {
                if owner.motion.action != MOT_ITEM_KAENGEKI {
                    return self.refund_cast(id, actors, ctx);
                }
                if owner.motion.count != 0 || owner.motion.loops == 0 {
                    return Ok(());
                }
                let action = owner.return_normal(ctx, player)?;
                owner.enter(ctx, action, true);
                let position = ctx.scene.world(owner.parts[1])?.translation;
                ctx.effects.scatter_bleeds(
                    position,
                    100,
                    crate::effects::Burst {
                        spread: 600,
                        count: 20,
                        lifetime: 15,
                        color: [180, 140, 30],
                    },
                    ctx.random,
                );
                owner.active_item = item.kind as i16;
                self.slots[id].behavior =
                    Behavior::Charm(used.with_phase(state::Charm::Protected { remaining: 450 }));
            }
            state::Charm::Protected { remaining } => {
                let remaining = remaining.wrapping_sub(1);
                self.slots[id].behavior =
                    Behavior::Charm(used.with_phase(state::Charm::Protected { remaining }));
                if remaining == 0 {
                    return self.dispose(id, actors, ctx);
                }
                if remaining & 63 == 0 {
                    let rotation = (ctx.random.next15() % 360) as i16;
                    ctx.effects.impact_ex(effects::Impact {
                        parent: Some(owner.root_model),
                        position: Position::new(0, -1200, 400),
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
        }
        Ok(())
    }
}
