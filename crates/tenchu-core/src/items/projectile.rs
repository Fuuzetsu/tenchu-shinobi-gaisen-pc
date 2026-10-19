use super::*;
use crate::{actor::trail::Trail, ai::Intelligence, collision, scene::CONFLICT};

pub(super) fn target(
    owner: &Actor,
    rotation: [i16; 3],
    start: [i32; 3],
    roster: &Roster,
    ctx: &mut Context,
) -> Result<(Option<usize>, [i32; 3])> {
    let ray = Matrix {
        rotation: ctx.math.rotate_yxz(rotation),
        ..Matrix::default()
    }
    .apply_large([0, 0, -17000]);
    let end = std::array::from_fn(|i| start[i].wrapping_add(ray[i]));
    let (_, mut position) =
        terrain::ground_ray(&ctx.mission.terrain, ctx.cursor, ctx.math, start, end, 0)?;
    let mut distance = ctx
        .math
        .vector_distance(std::array::from_fn(|i| position[i].wrapping_sub(start[i])));
    let inverse = Matrix {
        rotation: ctx.math.rotate_xyz(rotation.map(i16::wrapping_neg)),
        ..Matrix::default()
    };
    let mut found = None;
    for &id in &roster.order {
        let actor = if id == owner.id {
            owner
        } else {
            roster.get(id)
        };
        let world = ctx.scene.world(actor.parts[0])?.translation;
        if actor.health > 0 && id != owner.id {
            let local =
                inverse.apply_large(std::array::from_fn(|i| world[i].wrapping_sub(start[i])));
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
    pub(crate) fn projectile(
        &mut self,
        kind: usize,
        owner: &mut Actor,
        start: [i32; 3],
        end: [i32; 3],
        roster: &Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        if owner.inventory[kind] != 0 && owner.inventory[kind] != 255 {
            owner.inventory[kind] -= 1;
        }
        let (processor, end) = match kind {
            1 => (
                Processor::Shuriken,
                target(owner, owner.rotation, start, roster, ctx)?.1,
            ),
            20 => (Processor::Gun, end),
            21 => {
                let rotation = ctx.math.vector_rotation(start, end)?;
                (
                    Processor::Arrow,
                    target(owner, rotation, start, roster, ctx)?.1,
                )
            }
            _ => return Err(format!("Projectile item {kind} is not implemented yet")),
        };
        let id = self.allocate(ctx)?;
        self.initialize(id, processor, kind, Some(owner.id), start, ctx)?;
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
                    owner.inventory[25] = 0;
                }
            }
            Processor::Gun => self.slots[id].flight.end = end,
            _ => unreachable!(),
        }
        Ok(())
    }

    fn fly(&mut self, id: usize, ctx: &mut Context) -> Result<()> {
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

    fn hit(&mut self, id: usize, ctx: &mut Context, roster: &Roster) -> Option<(usize, usize)> {
        let model = self.slots[id].model;
        if ctx.scene.models[model].attribute & CONFLICT == 0 {
            return None;
        }
        let slot = ctx.collision.result(ctx.scene, model, None)?;
        let owner = ctx.collision.slots[slot].owner?;
        roster.order.contains(&owner).then_some((slot, owner))
    }

    pub(super) fn shuriken(
        &mut self,
        id: usize,
        ctx: &mut Context,
        roster: &Roster,
        intelligence: &mut Intelligence,
        clock: i32,
    ) -> Result<()> {
        self.fly(id, ctx)?;
        let model = self.slots[id].model;
        ctx.scene.models[model].local.rotation =
            ctx.math
                .rotate_yxz([0, clock.wrapping_mul(4096 / 6) as i16, 0]);
        ctx.scene.models[model].flags = 0;
        self.draw_model(id, 0, ctx)?;
        if let Some(trail) = &mut self.slots[id].trail {
            ctx.drawing.bytes += trail.draw(ctx.scene, ctx.camera.view, true)?;
            self.trails.push(trail.clone());
        }
        let position = ctx.scene.models[model].local.translation;
        if self.hit(id, ctx, roster).is_some() {
            ctx.effects.impact(position, 16384, 2, ctx.random);
            ctx.effects.sounds.push((position, 0x30));
            self.dispose(id, ctx)?;
        } else if self.slots[id].flight.rolling {
            match self.slots[id].rolling.status {
                3 => {
                    ctx.effects
                        .scatter_bleeds(position, 0, 25, 10, 10, [255, 255, 0], ctx.random);
                    ctx.effects.sounds.push((position, 0x31));
                    intelligence.reset_alert();
                }
                2 | 4 => {
                    let kind = self.slots[id].kind;
                    let owner = self.slots[id].owner;
                    self.dispose(id, ctx)?;
                    self.drop(kind, owner, position, [0; 3], ctx)?;
                }
                1 => self.dispose(id, ctx)?,
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn arrow(
        &mut self,
        id: usize,
        ctx: &mut Context,
        roster: &Roster,
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
                            .filter(|id| roster.order.contains(id))
                        {
                            if ctx.collision.slots[slot].class & collision::HIT != 0 {
                                self.dispose(id, ctx)?;
                                return Ok(());
                            }
                            let actor = roster.get(actor_id);
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
                            attached.local.translation = [0; 3];
                            self.slots[id].count = 120;
                            self.slots[id].mode = 1;
                            ctx.collision.remove(ctx.scene, model);
                            self.draw_model(id, 1, ctx)?;
                            return Ok(());
                        }
                    } else if self.arrow_ground(id, ctx)? {
                        return Ok(());
                    }
                } else if self.arrow_ground(id, ctx)? {
                    return Ok(());
                }
                let position = ctx.scene.models[model].local.translation;
                let mut rotation = ctx.math.vector_rotation(previous, position)?;
                rotation[2] = clock.wrapping_mul(256) as i16;
                ctx.scene.models[model].local.rotation = ctx.math.rotate_yxz(rotation);
                ctx.scene.models[model].flags = 0;
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
                    self.dispose(id, ctx)?;
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

    fn arrow_ground(&mut self, id: usize, ctx: &mut Context) -> Result<bool> {
        let item = &mut self.slots[id];
        if !item.flight.rolling || item.rolling.status == 0 {
            return Ok(false);
        }
        if item.rolling.status == 1 {
            self.dispose(id, ctx)?;
        } else {
            let position = ctx.scene.models[item.model].local.translation;
            ctx.effects.sounds.push((position, 0x31));
            ctx.effects
                .scatter_bleeds(position, 0, 25, 30, 30, [255, 255, 0], ctx.random);
            item.count = 30;
            item.mode = 1;
            ctx.collision.remove(ctx.scene, item.model);
        }
        Ok(true)
    }

    pub(super) fn gun(&mut self, id: usize, ctx: &mut Context, roster: &Roster) -> Result<()> {
        let item = &self.slots[id];
        let owner = roster.get(item.owner.ok_or("Gun has no owner")?);
        let model = item.model;
        let position = ctx.scene.models[model].local.translation;
        match item.mode {
            0 => {
                ctx.effects.impact(position, 8192, 0, ctx.random);
                ctx.effects
                    .scatter_bleeds(position, 100, 10, 10, 10, [127; 3], ctx.random);
                self.slots[id].mode = 1;
            }
            1 => {
                let rotation = ctx.math.vector_rotation(position, item.flight.end)?;
                let (hit, position) = target(owner, rotation, position, roster, ctx)?;
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
                    velocity,
                    100,
                    15,
                    10,
                    if hit.is_some() {
                        [255, 0, 0]
                    } else {
                        [255, 255, 0]
                    },
                    ctx.random,
                );
                ctx.effects
                    .sounds
                    .push((position, if hit.is_some() { 0x34 } else { 0x35 }));
                self.slots[id].mode = 2;
            }
            2 => self.dispose(id, ctx)?,
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
        for k in 0..3 {
            m[i][k] = (i32::from(m[i][k]) * 4096 / diagonal) as i16;
        }
        m[i][i] = (4096 * 4096 / diagonal) as i16;
        for j in 0..3 {
            if j == i {
                continue;
            }
            let value = i32::from(m[j][i]);
            for k in 0..3 {
                m[j][k] = if k == i {
                    (-value * 4096 / diagonal) as i16
                } else {
                    m[j][k].wrapping_sub((i32::from(m[i][k]) * value / 4096) as i16)
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
