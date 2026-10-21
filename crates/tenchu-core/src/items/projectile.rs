use super::*;
use crate::math::Position;
use crate::{actor::trail::Trail, ai::Intelligence, collision, scene::CONFLICT};

pub(super) fn target(
    owner: usize,
    rotation: [i16; 3],
    start: Position,
    actors: &Actors,
    ctx: &mut Context,
) -> Result<(Option<usize>, Position)> {
    let ray = Matrix {
        rotation: ctx.math.rotate_yxz(rotation),
        ..Matrix::default()
    }
    .apply_large(Position::new(0, 0, -17000));
    let end = std::array::from_fn(|i| start[i].wrapping_add(ray[i]));
    let (_, mut position) = terrain::ground_ray(
        &ctx.mission.terrain,
        ctx.cursor,
        ctx.math,
        start,
        end.into(),
        0,
    )?;
    let mut distance = ctx.math.vector_distance(position.wrapping_sub(start));
    let inverse = Matrix {
        rotation: ctx.math.rotate_xyz(rotation.map(i16::wrapping_neg)),
        ..Matrix::default()
    };
    let mut found = None;
    for &id in &actors.roster.order {
        let actor = actors.get(id);
        let world = ctx.scene.world(actor.parts[0])?.translation;
        if actor.health > 0 && id != owner {
            let local = inverse.apply_large(world.wrapping_sub(start));
            let forward = local[2].wrapping_neg();
            if forward > 100
                && local[0].wrapping_abs() < 500
                && local[1].wrapping_abs() < 1000
                && forward < distance
            {
                found = Some(id);
                distance = forward;
                position = world;
            }
        }
    }
    Ok((found, position))
}

impl Items {
    pub(super) fn launch(
        &mut self,
        kind: usize,
        owner_id: usize,
        start: Position,
        end: Position,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let processor = match kind {
            1 => Processor::Shuriken,
            20 => Processor::Gun,
            21 => Processor::Arrow,
            _ => return Err(format!("Invalid projectile kind {kind}")),
        };
        let id = self.allocate(actors, ctx)?;
        self.initialize(id, processor, kind, Some(owner_id), start, ctx)?;
        self.slots[id].collision.size = 0;
        match processor {
            Processor::Shuriken | Processor::Arrow => {
                self.slots[id].flight = flight::Flight::new(
                    start,
                    end,
                    if kind == 1 { 1024 } else { 0 },
                    if kind == 1 { 1024 } else { 2048 },
                    300,
                    ctx,
                );
                self.slots[id].count = 5;
                if kind == 1 {
                    self.slots[id].trail =
                        Some(Trail::new(self.meshes[0], [[20, 0, 0], [-20, 0, 0]]));
                }
            }
            Processor::Gun => self.slots[id].flight.end = end,
            _ => unreachable!(),
        }
        Ok(())
    }

    pub(super) fn fly(&mut self, id: usize, ctx: &mut Context) -> Result<()> {
        let item = &mut self.slots[id];
        let mut position = ctx.scene.models[item.model].local.translation;
        item.flight.step(&mut position, &mut item.rolling, ctx)?;
        ctx.scene.set_translation(item.model, position);
        item.count = item.count.wrapping_sub(1);
        if item.count == 0 {
            self.contact(id, 300, collision::HIT, ctx)?;
        }
        Ok(())
    }

    pub(super) fn draw_trail(&mut self, id: usize, ctx: &mut Context) -> Result<()> {
        if let Some(trail) = &mut self.slots[id].trail {
            ctx.drawing.bytes += trail.draw(ctx.scene, ctx.camera.view, true)?;
            self.trails.push(trail.clone());
        }
        Ok(())
    }

    pub(super) fn first_contact(&mut self, id: usize, ctx: &mut Context) -> Option<usize> {
        let model = self.slots[id].model;
        if ctx.scene.models[model].attribute & CONFLICT == 0 {
            return None;
        }
        ctx.collision.result(ctx.scene, model, None)
    }

    pub(super) fn hit(
        &mut self,
        id: usize,
        ctx: &mut Context,
        actors: &mut Actors,
    ) -> Option<(usize, usize)> {
        let slot = self.first_contact(id, ctx)?;
        let owner = ctx.collision.slots[slot].owner?;
        actors
            .roster
            .order
            .contains(&owner)
            .then_some((slot, owner))
    }

    pub(super) fn shuriken(
        &mut self,
        id: usize,
        ctx: &mut Context,
        actors: &mut Actors,
        intelligence: &mut Intelligence,
        clock: i32,
    ) -> Result<()> {
        self.fly(id, ctx)?;
        let model = self.slots[id].model;
        self.slots[id].rotation = [0, clock.wrapping_mul(4096 / 6) as i16, 0];
        self.update_coordinate(id, ctx);
        self.draw_model(id, 0, ctx)?;
        self.draw_trail(id, ctx)?;
        let position = ctx.scene.models[model].local.translation;
        if self.hit(id, ctx, actors).is_some() {
            ctx.effects.impact(position, 16384, 2, ctx.random);
            ctx.effects.sounds.push((position, 0x30));
            self.dispose(id, actors, ctx)?;
        } else if self.slots[id].flight.rolling {
            match self.slots[id].rolling.status {
                3 => {
                    ctx.effects.scatter_bleeds(
                        position,
                        25,
                        crate::effects::Burst {
                            spread: 0,
                            count: 10,
                            lifetime: 10,
                            color: [255, 255, 0],
                        },
                        ctx.random,
                    );
                    ctx.effects.sounds.push((position, 0x31));
                    intelligence.reset_alert();
                }
                2 | 4 => {
                    let kind = self.slots[id].kind;
                    let owner = self.slots[id].owner;
                    self.dispose(id, actors, ctx)?;
                    self.drop(kind, owner, position, Position::ZERO, actors, ctx)?;
                }
                1 => self.dispose(id, actors, ctx)?,
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn arrow(
        &mut self,
        id: usize,
        ctx: &mut Context,
        actors: &mut Actors,
        clock: i32,
    ) -> Result<()> {
        let model = self.slots[id].model;
        match self.slots[id].mode {
            0 => {
                let previous = ctx.scene.models[model].local.translation;
                self.fly(id, ctx)?;
                if ctx.scene.models[model].attribute & CONFLICT != 0 {
                    if let Some(slot) = ctx.collision.result(ctx.scene, model, None) {
                        if let Some(actor_id) = ctx.collision.slots[slot]
                            .owner
                            .filter(|id| actors.roster.order.contains(id))
                        {
                            if ctx.collision.slots[slot].class & collision::HIT != 0 {
                                self.dispose(id, actors, ctx)?;
                                return Ok(());
                            }
                            let actor = actors.get(actor_id);
                            let part =
                                actor.parts[ctx.random.next15() as usize % actor.parts.len()];
                            let position = ctx.scene.world(part)?.translation;
                            ctx.effects.impact(position, 24576, 2, ctx.random);
                            let position = ctx.scene.world(part)?.translation;
                            ctx.effects.sounds.push((position, 0x30));
                            let world = ctx.scene.world(part)?;
                            arrange_local(&mut ctx.scene.models[model].local, world);
                            let attached = &mut ctx.scene.models[model];
                            attached.flags = 0;
                            attached.parent = Some(part);
                            attached.local.translation = Position::ZERO;
                            self.slots[id].count = 120;
                            self.slots[id].mode = 1;
                            ctx.collision.remove(ctx.scene, model);
                            self.draw_model(id, 1, ctx)?;
                            return Ok(());
                        }
                    } else if self.arrow_ground(id, actors, ctx)? {
                        return Ok(());
                    }
                } else if self.arrow_ground(id, actors, ctx)? {
                    return Ok(());
                }
                let position = ctx.scene.models[model].local.translation;
                let mut rotation = ctx.math.vector_rotation(previous, position)?;
                rotation[2] = clock.wrapping_mul(256) as i16;
                self.slots[id].rotation = rotation;
                self.update_coordinate(id, ctx);
            }
            1 => {
                self.slots[id].count = self.slots[id].count.wrapping_sub(1);
                if self.slots[id].count == 0 {
                    self.slots[id].count = 15;
                    self.slots[id].mode = 2;
                }
            }
            2 => {
                self.slots[id].count = self.slots[id].count.wrapping_sub(1);
                if self.slots[id].count == 0 {
                    self.dispose(id, actors, ctx)?;
                    return Ok(());
                }
                if self.slots[id].count & 1 != 0 {
                    return Ok(());
                }
            }
            _ => {}
        }
        self.draw_model(id, 1, ctx)
    }

    fn arrow_ground(&mut self, id: usize, actors: &mut Actors, ctx: &mut Context) -> Result<bool> {
        let item = &mut self.slots[id];
        if !item.flight.rolling || item.rolling.status == 0 {
            return Ok(false);
        }
        if item.rolling.status == 1 {
            self.dispose(id, actors, ctx)?;
        } else {
            let position = ctx.scene.models[item.model].local.translation;
            ctx.effects.sounds.push((position, 0x31));
            ctx.effects.scatter_bleeds(
                position,
                25,
                crate::effects::Burst {
                    spread: 0,
                    count: 30,
                    lifetime: 30,
                    color: [255, 255, 0],
                },
                ctx.random,
            );
            item.count = 30;
            item.mode = 1;
            ctx.collision.remove(ctx.scene, item.model);
        }
        Ok(true)
    }

    pub(super) fn gun(&mut self, id: usize, ctx: &mut Context, actors: &mut Actors) -> Result<()> {
        let item = &self.slots[id];
        let owner = actors.get(item.owner.ok_or("Gun has no owner")?);
        let model = item.model;
        let position = ctx.scene.models[model].local.translation;
        match item.mode {
            0 => {
                ctx.effects.impact(position, 8192, 0, ctx.random);
                ctx.effects.scatter_bleeds(
                    position,
                    10,
                    crate::effects::Burst {
                        spread: 100,
                        count: 10,
                        lifetime: 10,
                        color: [127; 3],
                    },
                    ctx.random,
                );
                self.slots[id].mode = 1;
            }
            1 => {
                let rotation = ctx.math.vector_rotation(position, item.flight.end)?;
                let (hit, position) = target(owner.id, rotation, position, actors, ctx)?;
                ctx.scene.set_translation(model, position);
                self.contact(id, 100, collision::HIT, ctx)?;
                let matrix = Matrix {
                    rotation: ctx
                        .math
                        .rotate_yxz([owner.rotation[0], owner.rotation[1], 0]),
                    ..Matrix::default()
                };
                let velocity = matrix
                    .transform([0, 0, 150])
                    .map(|v| v.clamp(-32768, 32767) as i16);
                ctx.effects.impact(
                    position,
                    if hit.is_some() { 24576 } else { 16384 },
                    0,
                    ctx.random,
                );
                ctx.effects.bleeds(
                    position,
                    velocity.to_array(),
                    crate::effects::Burst {
                        spread: 100,
                        count: 15,
                        lifetime: 10,
                        color: if hit.is_some() {
                            [255, 0, 0]
                        } else {
                            [255, 255, 0]
                        },
                    },
                    ctx.random,
                );
                ctx.effects
                    .sounds
                    .push((position, if hit.is_some() { 0x34 } else { 0x35 }));
                self.slots[id].mode = 2;
            }
            2 => self.dispose(id, actors, ctx)?,
            _ => {}
        }
        Ok(())
    }
}

fn arrange_local(local: &mut Matrix, parent: Matrix) {
    let mut m = parent.rotation;
    let mut determinant = 4096i32;
    for i in 0..3 {
        let diagonal = if m[i][i] == 0 {
            4096
        } else {
            i32::from(m[i][i])
        };
        determinant = determinant.wrapping_mul(diagonal) / 4096;
        for value in &mut m[i] {
            *value = (i32::from(*value) * 4096 / diagonal) as i16;
        }
        m[i][i] = (4096 * 4096 / diagonal) as i16;
        let pivot = m[i];
        for (j, row) in m.iter_mut().enumerate() {
            if j == i {
                continue;
            }
            let value = i32::from(row[i]);
            for (k, cell) in row.iter_mut().enumerate() {
                *cell = if k == i {
                    (-value * 4096 / diagonal) as i16
                } else {
                    cell.wrapping_sub((i32::from(pivot[k]) * value / 4096) as i16)
                };
            }
        }
    }
    if (2048..=4096).contains(&determinant) {
        local.rotation = Matrix {
            rotation: m,
            ..Matrix::default()
        }
        .compose(local)
        .rotation;
    }
}
