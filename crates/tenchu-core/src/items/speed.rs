use super::*;
use crate::{camera::Mode, ids::MOT_ITEM_SHINSOKU, input::*};

impl Items {
    pub(super) fn speed(
        &mut self,
        id: usize,
        actors: &mut Actors,
        player: usize,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = item.owner.ok_or("Speed scroll has no owner")?;
        let owner = actors.get_mut(owner_id);
        match item.mode {
            0 => {
                owner.enter(ctx, MOT_ITEM_SHINSOKU, true);
                ctx.effects.sounds.push((owner.position, 0x4c));
                self.slots[id].mode = 1;
            }
            1 => {
                if owner.motion.action != MOT_ITEM_SHINSOKU {
                    return self.refund_cast(id, actors, ctx);
                }
                if owner.motion.count == 0 && owner.motion.loops != 0 {
                    ctx.effects
                        .smoke_burst(owner.position, 150, 12, 8, ctx.random);
                    self.slots[id].count = 75;
                    self.slots[id].mode = 2;
                }
            }
            2 => {
                if owner.motion.action != MOT_ITEM_SHINSOKU {
                    return self.dispose(id, actors, ctx);
                }
                let mut position = std::array::from_fn(|i| {
                    ctx.scene.models[owner.root_model].local.translation[i]
                        .wrapping_add(i32::from(item.rolling.velocity[i]))
                });
                let mut query = position;
                query[1] = query[1].wrapping_sub(2000);
                let map = terrain::vector(&ctx.mission.terrain, ctx.cursor, query.into(), 500, 0)?;
                if map.level >= position[1].wrapping_sub(500) {
                    position[1] = position[1].min(map.level);
                    owner.position = position.into();
                    // MODEL_POSITION aliases locate; the original leaves the world cache intact.
                    ctx.scene.set_translation(owner.root_model, position.into());
                }
                if item.count & 3 == 0 {
                    let mut position = ctx.scene.models[owner.root_model].local.translation;
                    position[1] = position[1].wrapping_sub(300);
                    ctx.effects.impact_ex(effects::Impact {
                        parent: None,
                        position,
                        size: [8192, 20480],
                        color: [[128; 3], [0; 3]],
                        rotation: 0,
                        speed: -30,
                        sprite: 3,
                        count: 0,
                        time: 16,
                    });
                }
                if owner_id == player {
                    use_item::camera(Mode::Crouch, owner, ctx)?;
                }
                let turn = if owner.pad.held & RIGHT != 0 {
                    64
                } else if owner.pad.held & LEFT != 0 {
                    -64
                } else {
                    0
                };
                let item = &mut self.slots[id];
                if turn != 0 {
                    owner.rotation[1] = owner.rotation[1].wrapping_add(turn);
                    item.rolling.velocity = Matrix {
                        rotation: ctx.math.rotate_yxz([0, turn, 0]),
                        ..Matrix::default()
                    }
                    .transform(item.rolling.velocity)
                    .map(|v| v.clamp(-32768, 32767) as i16)
                    .to_array();
                }
                item.count = item.count.wrapping_sub(1);
                if item.count == 0 || owner.pad.trigger & (SQUARE | CROSS | CIRCLE | TRIANGLE) != 0
                {
                    let action = owner.return_normal(ctx, player)?;
                    owner.enter(ctx, action, true);
                    ctx.camera.normal(actors.get(player), ctx.math)?;
                    self.dispose(id, actors, ctx)?;
                }
            }
            mode => return Err(format!("Invalid speed scroll mode {mode}")),
        }
        Ok(())
    }
}
