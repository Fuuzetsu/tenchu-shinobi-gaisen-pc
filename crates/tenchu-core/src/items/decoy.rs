use super::*;
use crate::collision;

#[derive(Clone, Debug, Default)]
pub struct Decoy {
    pub health: u8,
    pub player: usize,
}

impl Items {
    pub(super) fn finish_decoy(&mut self, id: usize, actors: &mut Actors) {
        let item = &self.slots[id];
        if item.decoy.health != 99 {
            let target = actors.get(item.decoy.player).root_model;
            for actor_id in actors.roster.order.clone() {
                let actor = actors.get_mut(actor_id);
                if actor.target == Some(item.model) {
                    actor.target = Some(target);
                }
            }
            self.decoy_count = self.decoy_count.wrapping_sub(1);
        }
    }

    pub(super) fn decoy(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        let model = item.model;
        let mut position = ctx.scene.models[model].local.translation;
        item.rolling.step(&mut position, ctx)?;
        ctx.scene.set_translation(model, position);
        if item.rolling.status == 1 {
            return self.dispose(id, actors, ctx);
        }
        match item.mode {
            0 => {
                item.count = item.count.wrapping_sub(1);
                if item.count == 0 {
                    item.mode = 1;
                    ctx.effects
                        .smoke(position, [0, -25, 0], 10, 6, ctx.random)?;
                    ctx.effects.sounds.push((position, 0x23));
                    if self.decoy_count < 3 {
                        item.decoy.health = 3;
                        self.decoy_count += 1;
                    } else {
                        let owner = item.owner;
                        let kind = item.kind;
                        let velocity = [
                            ctx.random.next15() % 200 - 100,
                            ctx.random.next15() % 100 - 200,
                            ctx.random.next15() % 200 - 100,
                        ];
                        self.drop(kind, owner, position, velocity, actors, ctx)?;
                        return self.dispose(id, actors, ctx);
                    }
                }
                self.update_coordinate(id, ctx);
                self.draw_icon(id, ctx)?;
            }
            1 => {
                item.count = item.count.wrapping_add(1);
                let scale = i32::from(item.count) << 8;
                let coordinate = &mut ctx.scene.models[model];
                coordinate.local.rotation = ctx
                    .math
                    .rotate_yxz(item.rotation)
                    .map(|row| row.map(|v| ((i32::from(v) * scale) >> 12) as i16));
                coordinate.flags = 0;
                let grown = item.count >= 16;
                self.draw_model(id, 2, ctx)?;
                if grown {
                    self.contact(id, 500, collision::STAND | collision::SOFT, ctx)?;
                    let item = &mut self.slots[id];
                    item.collision.offset_y = -250;
                    ctx.collision.slots[ctx.scene.models[model].conflict.unwrap()].offset[1] = -250;
                    item.count = 3;
                    item.mode = 2;
                }
            }
            2 => {
                let contact = self.first_contact(id, ctx);
                let item = &mut self.slots[id];
                item.count = item.count.wrapping_sub(1);
                if item.count == 0 {
                    for actor_id in actors.roster.order.clone() {
                        let actor = actors.get_mut(actor_id);
                        let distance = ctx.math.vector_distance(std::array::from_fn(|i| {
                            position[i].wrapping_sub(actor.position[i])
                        }));
                        if distance < 10000
                            && let Some(target) = actor.target
                        {
                            let target_position = ctx.scene.models[target].local.translation;
                            let old_distance = ctx.math.vector_distance(std::array::from_fn(|i| {
                                target_position[i].wrapping_sub(actor.position[i])
                            }));
                            if distance < old_distance && actor.kind & 0xf0 != 0x80 {
                                actor.target = Some(model);
                            }
                        }
                    }
                    item.count = 30;
                } else if let Some(contact) = contact {
                    let class = ctx.collision.slots[contact].class;
                    let [x, y, z] = ctx.collision.distance.map(i32::from);
                    if class == collision::HIT {
                        if item.decoy.health == 0 {
                            ctx.effects.scatter_bleeds(
                                position,
                                0,
                                30,
                                30,
                                30,
                                [255, 255, 0],
                                ctx.random,
                            );
                            ctx.effects.sounds.push((position, 0x23));
                            self.dispose(id, actors, ctx)?;
                            self.slots[id].mode = self.slots[id].mode.wrapping_add(1);
                        } else {
                            item.rolling = Rolling {
                                velocity: [(-x / 16) as i16, -100, (-z / 16) as i16],
                                ..Rolling::default()
                            };
                            item.decoy.health -= 1;
                            ctx.effects.sounds.push((position, 0x30));
                        }
                    } else if class != collision::SOFT {
                        let vx = (-x / 8 + ctx.random.next15() % 20 - 10) as i16;
                        let vz = (-z / 8 + ctx.random.next15() % 20 - 10) as i16;
                        item.rolling = Rolling {
                            velocity: [vx, if y >= -500 { -100 } else { 0 }, vz],
                            ..Rolling::default()
                        };
                    }
                }
                self.update_coordinate(id, ctx);
                self.draw_model(id, 2, ctx)?;
            }
            mode => return Err(format!("Invalid decoy mode {mode}")),
        }
        Ok(())
    }
}
