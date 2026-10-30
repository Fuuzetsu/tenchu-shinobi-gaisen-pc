use super::*;
use crate::actor::State as ActorState;
use crate::camera::{Environment, Mode};
use crate::math::Position;

pub(super) fn consume(owner: &mut Actor, kind: Kind) {
    let count = &mut owner.inventory[kind.index()];
    if *count != 0 && *count != 255 {
        *count -= 1;
    }
}

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

fn throw_vector(owner: &Actor, player: usize, vector: Position, ctx: &Context) -> Result<Position> {
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
    pub(super) fn refund_cast(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let position = ctx.scene.world(item.model)?.translation;
        let (kind, owner) = (item.kind, item.owner());
        let velocity = [
            ctx.random.next15() % 200 - 100,
            ctx.random.next15() % 100 - 200,
            ctx.random.next15() % 200 - 100,
        ];
        self.drop(kind, owner, position, velocity.into(), actors, ctx)?;
        self.dispose(id, actors, ctx)
    }

    fn throw_rolling(
        &mut self,
        kind: Kind,
        owner: usize,
        start: Position,
        velocity: Position,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let behavior = match kind {
            Kind::Caltrops => Behavior::Caltrops(Used::new(owner, state::Caltrops::Falling)),
            Kind::Grenade => Behavior::Fire(Used::new(owner, state::Fire::Fuse)),
            Kind::SmokeBomb => Behavior::Smoke(Used::new(owner, state::Smoke::Fuse)),
            Kind::Mine => Behavior::Mine(Used::new(owner, state::Mine::Place)),
            Kind::PoisonedFood => Behavior::Poison(Used::new(owner, state::Poison::Delay)),
            _ => return Err(format!("Rolling item {kind:?} is not implemented yet")),
        };
        let id = self.allocate(actors, ctx)?;
        self.initialize(id, behavior, kind, start, ctx);
        let item = &mut self.slots[id];
        item.collision.size = 0;
        item.rolling = Rolling {
            velocity: velocity.map(|v| v as i16).to_array(),
            ..Rolling::default()
        };
        match kind {
            Kind::Caltrops => ctx.effects.sounds.push((start, 0x22)),
            Kind::Grenade => item.count = 150,
            Kind::SmokeBomb => item.count = 10,
            Kind::PoisonedFood => {
                item.poison.remaining = 10;
                item.poison.eater = None;
                actors
                    .get_mut(owner)
                    .enter(ctx, crate::ids::MOT_ITEM_THROW, true);
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn wall_throw(
        &mut self,
        kind: Kind,
        owner: &mut Actor,
        start: Position,
        mut angle: i16,
        roster: &mut Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        let id = owner.id;
        let actors = &mut Actors {
            roster,
            active: Some(owner),
        };
        if kind == Kind::Caltrops {
            for _ in 0..5 {
                angle = angle
                    .wrapping_sub(10)
                    .wrapping_add((ctx.random.next15() % 20) as i16);
                let [sin, cos] = ctx.math.pair(angle);
                let velocity = [
                    (sin * (-30 - ctx.random.next15() % 200)) >> 12,
                    -(ctx.random.next15() % 30),
                    (cos * (-30 - ctx.random.next15() % 200)) >> 12,
                ];
                self.throw_rolling(kind, id, start, velocity.into(), actors, ctx)?;
            }
        } else {
            let [sin, cos] = ctx.math.pair(angle);
            self.throw_rolling(
                kind,
                id,
                start,
                Position::new((sin * -120) >> 12, 0, (cos * -120) >> 12),
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
        actor.status = ActorState::from_motion(action);
        actor.motion_velocity(ctx);
    }

    pub(crate) fn request_default(
        &mut self,
        kind: Kind,
        owner: &mut Actor,
        player: usize,
        roster: &mut Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        let start = [
            owner.position.x,
            owner.position.y.wrapping_sub(1200),
            owner.position.z,
        ];
        let delta = throw_vector(owner, player, Position::new(0, 0, -100), ctx)?;
        let end = std::array::from_fn(|i| start[i].wrapping_add(delta[i]));
        self.request(
            crate::items::Request {
                kind,
                start: start.into(),
                end: end.into(),
            },
            owner,
            player,
            roster,
            ctx,
        )
    }

    pub(crate) fn request(
        &mut self,
        request: Request,
        owner: &mut Actor,
        player: usize,
        roster: &mut Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        let Request { kind, start, end } = request;
        consume(owner, kind);
        let owner_id = owner.id;
        let actors = &mut Actors {
            roster,
            active: Some(owner),
        };
        match kind {
            Kind::Grapple => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, Behavior::Grapple { owner: owner_id }, kind, start, ctx);
                self.slots[id].collision.size = 0;
                actors.get_mut(owner_id).aiming = true;
                camera(Mode::Sight, actors.get(player), ctx)?;
                ctx.camera.direction = [-341, 0];
            }
            Kind::Shuriken if owner_id == player => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(
                    id,
                    Behavior::SightShot { owner: owner_id },
                    kind,
                    start,
                    ctx,
                );
                self.slots[id].collision.size = 0;
                self.slots[id].count = 5;
                actors.get_mut(owner_id).aiming = true;
            }
            Kind::Shuriken => {
                let end = projectile::target(
                    owner_id,
                    actors.get(owner_id).rotation,
                    start,
                    actors,
                    ctx,
                )?
                .1;
                self.launch(kind, owner_id, start, end, actors, ctx)?;
                actors.get_mut(owner_id).aiming = false;
            }
            Kind::Caltrops => {
                let velocity =
                    throw_vector(actors.get(owner_id), player, Position::new(0, 0, 100), ctx)?;
                for _ in 0..5 {
                    let velocity = [
                        velocity[0].wrapping_add(ctx.random.next15() % 100 - 50),
                        velocity[1].wrapping_sub(ctx.random.next15() % 50 + 50),
                        velocity[2].wrapping_add(ctx.random.next15() % 100 - 50),
                    ];
                    self.throw_rolling(kind, owner_id, start, velocity.into(), actors, ctx)?;
                }
            }
            Kind::Medicine
            | Kind::Resurrection
            | Kind::Disguise
            | Kind::ProtectionCharm
            | Kind::Flute => {
                let behavior = match kind {
                    Kind::Medicine => {
                        Behavior::Medicine(Used::new(owner_id, state::Medicine::Begin))
                    }
                    Kind::Resurrection => {
                        Behavior::Resurrection(Used::new(owner_id, state::Resurrection::Begin))
                    }
                    Kind::Disguise => {
                        Behavior::Disguise(Used::new(owner_id, state::Disguise::Begin))
                    }
                    Kind::ProtectionCharm => {
                        Behavior::Charm(Used::new(owner_id, state::Charm::Begin))
                    }
                    _ => Behavior::Flute(Used::new(owner_id, state::Flute::Begin)),
                };
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, behavior, kind, start, ctx);
                self.slots[id].collision.size = 0;
            }
            Kind::Grenade | Kind::SmokeBomb | Kind::Mine | Kind::PoisonedFood => {
                let vector = match kind {
                    Kind::Grenade => [0, -120, -120],
                    Kind::SmokeBomb => [0, 0, -60],
                    Kind::PoisonedFood => [0, -120, -240],
                    _ => [0, 0, -1000],
                };
                let velocity = throw_vector(actors.get(owner_id), player, vector.into(), ctx)?;
                let position = if kind == Kind::Mine {
                    start.wrapping_add(velocity)
                } else {
                    start
                };
                self.throw_rolling(kind, owner_id, position, velocity, actors, ctx)?;
            }
            Kind::Gun => self.launch(kind, owner_id, start, end, actors, ctx)?,
            Kind::Lightning => {
                let end = if owner_id == player {
                    let vector = throw_vector(
                        actors.get(owner_id),
                        player,
                        Position::new(0, 0, -4096),
                        ctx,
                    )?;
                    start.wrapping_add(vector)
                } else {
                    end
                };
                let id = self.allocate(actors, ctx)?;
                self.initialize(
                    id,
                    Behavior::Lightning(Used::new(owner_id, state::Lightning::Begin)),
                    kind,
                    start,
                    ctx,
                );
                self.slots[id].collision.size = 0;
                self.slots[id].lightning = lightning::Lightning {
                    start,
                    rotation: ctx.math.vector_rotation(start, end)?,
                };
            }
            Kind::Teleport => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, Behavior::Teleport { owner: owner_id }, kind, start, ctx);
                self.slots[id].collision.size = 0;
                ctx.camera.mode = Mode::Sight;
            }
            Kind::Arrow => {
                let rotation = ctx.math.vector_rotation(start, end)?;
                let end = projectile::target(owner_id, rotation, start, actors, ctx)?.1;
                self.launch(kind, owner_id, start, end, actors, ctx)?;
            }
            Kind::Armor => {}
            Kind::Darts => self.request_darts(owner_id, start, actors, ctx)?,
            Kind::Dog => {
                let velocity = throw_vector(
                    actors.get(owner_id),
                    player,
                    Position::new(0, -120, -120),
                    ctx,
                )?;
                let id = self.allocate(actors, ctx)?;
                self.initialize(
                    id,
                    Behavior::Dog(Used::new(
                        owner_id,
                        state::Dog::Thrown {
                            remaining: 15,
                            player,
                        },
                    )),
                    kind,
                    start,
                    ctx,
                );
                let item = &mut self.slots[id];
                item.collision.size = 0;
                item.rolling = Rolling {
                    velocity: [velocity[0] as i16, -250, velocity[2] as i16],
                    ..Rolling::default()
                };
                actors
                    .get_mut(owner_id)
                    .enter(ctx, crate::ids::MOT_ITEM_THROW, true);
            }
            Kind::Decoy => {
                let velocity = throw_vector(
                    actors.get(owner_id),
                    player,
                    Position::new(0, -120, -120),
                    ctx,
                )?;
                let id = self.allocate(actors, ctx)?;
                self.initialize(
                    id,
                    Behavior::Decoy(Used::new(owner_id, state::Decoy::Fuse)),
                    kind,
                    start,
                    ctx,
                );
                let item = &mut self.slots[id];
                item.collision.size = 0;
                item.rolling = Rolling {
                    velocity: velocity.map(|v| v as i16).to_array(),
                    ..Rolling::default()
                };
                item.count = 90;
                item.rotation = [
                    0,
                    (ctx.random.next15() % 4096) as i16,
                    (ctx.random.next15() % 68) as i16,
                ];
                item.decoy = decoy::Decoy { health: 99, player };
                actors
                    .get_mut(owner_id)
                    .enter(ctx, crate::ids::MOT_ITEM_THROW, true);
            }
            Kind::ColoredRice | Kind::SpeedScroll => {
                let (behavior, speed) = if kind == Kind::ColoredRice {
                    (
                        Behavior::Rice(Used::new(owner_id, state::Rice::Begin)),
                        -100,
                    )
                } else {
                    (
                        Behavior::Speed(Used::new(owner_id, state::Speed::Begin)),
                        -500,
                    )
                };
                let velocity = throw_vector(
                    actors.get(owner_id),
                    player,
                    Position::new(0, 0, speed),
                    ctx,
                )?;
                let id = self.allocate(actors, ctx)?;
                self.initialize(id, behavior, kind, start, ctx);
                self.slots[id].collision.size = 0;
                self.slots[id].rolling.velocity = velocity.map(|v| v as i16).to_array();
            }
            Kind::SleepingGas => {
                let velocity =
                    throw_vector(actors.get(owner_id), player, Position::new(0, 0, -120), ctx)?;
                let id = self.allocate(actors, ctx)?;
                self.initialize(
                    id,
                    Behavior::Sleep(Used::new(owner_id, state::Sleep::Begin)),
                    kind,
                    start,
                    ctx,
                );
                self.slots[id].collision.size = 0;
                self.slots[id].napalm.velocity = velocity.map(|v| v as i16).to_array();
            }
            Kind::FireBreath => {
                let id = self.allocate(actors, ctx)?;
                self.initialize(
                    id,
                    Behavior::Breath(Used::new(owner_id, state::Breath::Begin)),
                    kind,
                    start,
                    ctx,
                );
                self.slots[id].collision.size = 0;
                self.slots[id].breath = breath::Breath {
                    origin: start,
                    player,
                    ..breath::Breath::default()
                };
            }
            _ => return Err(format!("Item {kind:?} use is not implemented yet")),
        }
        Ok(())
    }
}
