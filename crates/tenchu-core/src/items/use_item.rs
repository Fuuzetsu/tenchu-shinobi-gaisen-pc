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

fn throw_vector(owner: &Actor, player: usize, vector: [i32; 3], ctx: &Context) -> Result<[i32; 3]> {
    let rotation = if owner.id == player && ctx.camera.mode == Mode::Direction {
        ctx.math
            .vector_rotation(ctx.camera.position, ctx.camera.reference)?
    } else {
        owner.rotation
    };
    Ok(Matrix {
        rotation: ctx.math.rotate_yxz(rotation),
        ..Matrix::default()
    }
    .apply_large(vector))
}

impl Items {
    fn throw_rolling(
        &mut self,
        kind: usize,
        owner: usize,
        start: [i32; 3],
        velocity: [i32; 3],
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let processor = match kind {
            2 => Processor::Caltrops,
            4 => Processor::Fire,
            5 => Processor::Smoke,
            6 => Processor::Mine,
            _ => return Err(format!("Rolling item {kind} is not implemented yet")),
        };
        let id = self.allocate(actors, ctx)?;
        self.initialize(id, processor, kind, Some(owner), start, ctx)?;
        let item = &mut self.slots[id];
        item.collision.size = 0;
        item.rolling = Rolling {
            velocity: velocity.map(|v| v as i16),
            ..Rolling::default()
        };
        match kind {
            2 => ctx.effects.sounds.push((start, 0x22)),
            4 => item.count = 150,
            5 => item.count = 10,
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn wall_throw(
        &mut self,
        kind: usize,
        owner: &mut Actor,
        start: [i32; 3],
        mut angle: i16,
        roster: &mut Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        let id = owner.id;
        let actors = &mut Actors {
            roster,
            active: Some(owner),
        };
        if kind == 2 {
            for _ in 0..5 {
                angle = angle
                    .wrapping_sub(10)
                    .wrapping_add((ctx.random.next15() % 20) as i16);
                let [sin, cos] = ctx.math.pair(angle);
                let velocity = [
                    sin * (-30 - ctx.random.next15() % 200) >> 12,
                    -(ctx.random.next15() % 30),
                    cos * (-30 - ctx.random.next15() % 200) >> 12,
                ];
                self.throw_rolling(kind, id, start, velocity, actors, ctx)?;
            }
        } else {
            let [sin, cos] = ctx.math.pair(angle);
            self.throw_rolling(
                kind,
                id,
                start,
                [sin * -120 >> 12, 0, cos * -120 >> 12],
                actors,
                ctx,
            )?;
        }
        Ok(())
    }

    pub(super) fn item_motion(actor: &mut Actor, action: i16, ctx: &Context) {
        actor.motion.change(
            action,
            &ctx.mission.characters[actor.definition].motions,
            &ctx.mission.common_motions,
            &ctx.mission.clips,
        );
        actor.status = (action >> 8) as i8 as i16;
        actor.motion_velocity(ctx);
    }

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
        let delta = throw_vector(owner, player, [0, 0, -100], ctx)?;
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
            0 => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, Processor::Grapple, kind, Some(owner_id), start, ctx)?;
                self.slots[id].collision.size = 0;
                actors.get_mut(owner_id).inventory[25] = 1;
                camera(Mode::Sight, actors.get(player), ctx)?;
                ctx.camera.direction = [-341, 0];
            }
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
                let velocity = throw_vector(actors.get(owner_id), player, [0, 0, 100], ctx)?;
                for _ in 0..5 {
                    let velocity = [
                        velocity[0].wrapping_add(ctx.random.next15() % 100 - 50),
                        velocity[1].wrapping_sub(ctx.random.next15() % 50 + 50),
                        velocity[2].wrapping_add(ctx.random.next15() % 100 - 50),
                    ];
                    self.throw_rolling(kind, owner_id, start, velocity, actors, ctx)?;
                }
            }
            3 => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, Processor::Medicine, kind, Some(owner_id), start, ctx)?;
                self.slots[id].collision.size = 0;
            }
            4 | 5 | 6 => {
                let vector = match kind {
                    4 => [0, -120, -120],
                    5 => [0, 0, -60],
                    _ => [0, 0, -1000],
                };
                let velocity = throw_vector(actors.get(owner_id), player, vector, ctx)?;
                let position = if kind == 6 {
                    std::array::from_fn(|i| start[i].wrapping_add(velocity[i]))
                } else {
                    start
                };
                self.throw_rolling(kind, owner_id, position, velocity, actors, ctx)?;
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
