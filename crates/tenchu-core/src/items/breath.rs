use super::*;
use crate::math::Position;
use crate::{camera::Mode, ids::MOT_ITEM_KAENGEKI, input};

#[derive(Clone, Debug, Default)]
pub struct Breath {
    pub origin: Position,
    pub remaining: u8,
    pub player: usize,
}

impl Items {
    pub(super) fn finish_breath(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner = actors.get_mut(item.owner.ok_or("Fire breath has no owner")?);
        if owner.motion.action == MOT_ITEM_KAENGEKI {
            let action = owner.return_normal(ctx, item.breath.player)?;
            owner.enter(ctx, action, true);
        }
        Ok(())
    }

    pub(super) fn breath(
        &mut self,
        id: usize,
        actors: &mut Actors,
        halt: i16,
        clock: i32,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = item.owner.ok_or("Fire breath has no owner")?;
        let owner = actors.get_mut(owner_id);
        match item.mode {
            0 => {
                if halt == 0 && owner.health > 0 {
                    owner.clear_attack(ctx, 3)?;
                    Self::item_motion(owner, MOT_ITEM_KAENGEKI, ctx);
                }
                ctx.effects.sounds.push((owner.position, 0x4c));
                self.slots[id].mode = 1;
            }
            1 => {
                if owner.motion.count == 0 && owner.motion.loops != 0 {
                    ctx.effects.sounds.push((owner.position, 0x28));
                    self.slots[id].mode = 2;
                    self.slots[id].breath.remaining = 40;
                }
                if owner.motion.action != MOT_ITEM_KAENGEKI {
                    let item = &self.slots[id];
                    let position = ctx.scene.world(item.model)?.translation;
                    let kind = item.kind;
                    let velocity = [
                        ctx.random.next15() % 200 - 100,
                        ctx.random.next15() % 100 - 200,
                        ctx.random.next15() % 200 - 100,
                    ];
                    self.drop(kind, Some(owner_id), position, velocity.into(), actors, ctx)?;
                    self.dispose(id, actors, ctx)?;
                }
            }
            2 => {
                if owner.motion.action != MOT_ITEM_KAENGEKI {
                    return self.dispose(id, actors, ctx);
                }
                let item = &mut self.slots[id];
                item.breath.remaining = item.breath.remaining.wrapping_sub(1);
                if item.breath.remaining == 0 {
                    return self.dispose(id, actors, ctx);
                }
                let turn = if owner.pad.held & input::RIGHT != 0 {
                    32
                } else if owner.pad.held & input::LEFT != 0 {
                    -32
                } else {
                    0
                };
                owner.rotation[1] = owner.rotation[1].wrapping_add(turn);
                let rotation =
                    if owner_id == item.breath.player && ctx.camera.mode == Mode::Direction {
                        ctx.math
                            .vector_rotation(ctx.camera.position, ctx.camera.reference)?
                    } else {
                        owner.rotation
                    };
                let direction = Matrix {
                    rotation: ctx.math.rotate_yxz(rotation),
                    ..Matrix::default()
                }
                .apply_large(Position::new(0, 0, -60));
                let start = std::array::from_fn(|i| {
                    item.breath.origin[i].wrapping_add(direction[i].wrapping_mul(12))
                });
                let end =
                    std::array::from_fn(|i| start[i].wrapping_add(direction[i].wrapping_mul(2)));
                self.request_napalm(owner_id, start.into(), end.into(), clock, actors, ctx)?;
            }
            mode => return Err(format!("Invalid fire-breath mode {mode}")),
        }
        Ok(())
    }
}
