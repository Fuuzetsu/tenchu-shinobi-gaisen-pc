use super::*;
use crate::camera::{Environment, Mode};

pub(super) fn camera(mode: Mode, owner: &Actor, ctx: &mut Context) -> Result<()> {
    ctx.camera.set_mode(
        mode,
        owner,
        &mut Environment {
            terrain: &ctx.mission.terrain,
            cursor: ctx.cursor,
            math: ctx.math,
            body: ctx.body,
        },
        ctx.random,
    )
}

impl Items {
    pub(crate) fn request_default(
        &mut self,
        kind: usize,
        owner: &mut Actor,
        player: usize,
        roster: &mut Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        let start = [
            owner.position[0],
            owner.position[1].wrapping_sub(1200),
            owner.position[2],
        ];
        let rotation = if owner.id == player && ctx.camera.mode == Mode::Direction {
            ctx.math
                .vector_rotation(ctx.camera.position, ctx.camera.reference)?
        } else {
            owner.rotation
        };
        let delta = Matrix {
            rotation: ctx.math.rotate_yxz(rotation),
            ..Matrix::default()
        }
        .apply_large([0, 0, -100]);
        let end = std::array::from_fn(|i| start[i].wrapping_add(delta[i]));
        self.request(kind, owner, start, end, player, roster, ctx)
    }

    pub(crate) fn request(
        &mut self,
        kind: usize,
        owner: &mut Actor,
        start: [i32; 3],
        end: [i32; 3],
        player: usize,
        roster: &mut Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        let count = owner
            .inventory
            .get_mut(kind)
            .ok_or("Invalid item request")?;
        if *count != 0 && *count != 255 {
            *count -= 1;
        }
        let owner_id = owner.id;
        let actors = &mut Actors {
            roster,
            active: Some(owner),
        };
        match kind {
            1 if owner_id == player => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, Processor::SightShot, kind, Some(owner_id), start, ctx)?;
                self.slots[id].collision.size = 0;
                self.slots[id].count = 5;
                actors.get_mut(owner_id).inventory[25] = 1;
            }
            1 => {
                let end = projectile::target(
                    owner_id,
                    actors.get(owner_id).rotation,
                    start,
                    actors,
                    ctx,
                )?
                .1;
                self.launch(kind, owner_id, start, end, actors, ctx)?;
                actors.get_mut(owner_id).inventory[25] = 0;
            }
            2 => {
                let rotation = if owner_id == player && ctx.camera.mode == Mode::Direction {
                    ctx.math
                        .vector_rotation(ctx.camera.position, ctx.camera.reference)?
                } else {
                    actors.get(owner_id).rotation
                };
                let velocity = Matrix {
                    rotation: ctx.math.rotate_yxz(rotation),
                    ..Matrix::default()
                }
                .apply_large([0, 0, 100]);
                for _ in 0..5 {
                    let velocity = [
                        velocity[0].wrapping_add(ctx.random.next15() % 100 - 50),
                        velocity[1].wrapping_sub(ctx.random.next15() % 50 + 50),
                        velocity[2].wrapping_add(ctx.random.next15() % 100 - 50),
                    ];
                    let id = self.allocate(actors, ctx)?;
                    self.initialize(id, Processor::Caltrops, kind, Some(owner_id), start, ctx)?;
                    self.slots[id].collision.size = 0;
                    self.slots[id].rolling = Rolling {
                        velocity: velocity.map(|v| v as i16),
                        ..Rolling::default()
                    };
                    ctx.effects.sounds.push((start, 0x22));
                }
            }
            3 => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, Processor::Medicine, kind, Some(owner_id), start, ctx)?;
                self.slots[id].collision.size = 0;
            }
            20 => self.launch(kind, owner_id, start, end, actors, ctx)?,
            21 => {
                let rotation = ctx.math.vector_rotation(start, end)?;
                let end = projectile::target(owner_id, rotation, start, actors, ctx)?.1;
                self.launch(kind, owner_id, start, end, actors, ctx)?;
            }
            19 => {}
            _ => return Err(format!("Item {kind} use is not implemented yet")),
        }
        Ok(())
    }
}
