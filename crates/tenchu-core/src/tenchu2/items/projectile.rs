use super::flight::{Contact, Flight, Motion};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    scene::{ModelId, Scene},
    tenchu2::{
        collision::{BODY, Collision, WEAPON},
        terrain::{self, Cursor, Query},
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision as Map;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Shuriken,
    Kunai,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Flying,
    Resting,
    Falling,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Projectile {
    pub kind: Kind,
    pub owner: usize,
    pub flight: Flight,
    pub transform: Matrix,
    pub rotation: [i16; 3],
    pub phase: Phase,
    pub trail: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Splash(Position),
    Impact {
        position: Position,
        direction: Position,
    },
    Sound {
        position: Position,
        id: u8,
    },
    Alert(Position),
}

pub struct Context<'a> {
    pub map: &'a Map,
    pub cursor: &'a mut Cursor,
    pub math: &'a Math,
    pub scene: &'a mut Scene,
    pub collision: &'a mut Collision,
    pub player: usize,
}

impl Projectile {
    pub fn launch(
        kind: Kind,
        owner: usize,
        origin: Position,
        direction: Position,
        math: &Math,
    ) -> Self {
        let velocity = math.normalize_vector(direction);
        Self {
            kind,
            owner,
            flight: Flight {
                position: origin,
                velocity,
                speed: if kind == Kind::Shuriken { 500 } else { 300 },
                ..Default::default()
            },
            transform: Matrix {
                rotation: super::facing(
                    velocity,
                    if kind == Kind::Shuriken { 4096 } else { -4096 },
                    math,
                ),
                translation: origin,
            },
            rotation: [0; 3],
            phase: Phase::Flying,
            trail: true,
        }
    }

    pub fn model(&self) -> u8 {
        match self.kind {
            Kind::Shuriken => 4,
            Kind::Kunai => 30,
        }
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(400, 100, 400);
        slot.owner = Some(self.owner);
        slot.flags = 0x5044;
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        if self.phase != Phase::Flying {
            return self.settled(model, context);
        }
        if self.kind == Kind::Shuriken {
            self.rotation[1] = self.rotation[1].wrapping_add(227);
        }
        let before = self.flight.speed;
        let direction = self.flight.velocity.map(i32::wrapping_neg);
        let (contact, splash) =
            self.flight
                .advance(Motion::Directed, context.map, context.cursor, context.math)?;
        if let Some(position) = splash {
            emit(Event::Splash(position))?;
        }
        if contact == Contact::Outside {
            return Ok(false);
        }
        let limit = if self.kind == Kind::Shuriken {
            250
        } else {
            150
        };
        if self.flight.speed <= limit {
            self.flight.gravity =
                self.flight
                    .gravity
                    .wrapping_add(if self.flight.speed == 0 { 16 } else { 2 });
        }
        if self.flight.speed != 0 {
            self.flight.speed = self.flight.speed.wrapping_sub(1);
        }
        if self.flight.speed == 0 && before != 0 {
            emit(Event::Impact {
                position: self.flight.position,
                direction,
            })?;
            emit(Event::Sound {
                position: self.flight.position,
                id: 48,
            })?;
            if self.owner == context.player {
                emit(Event::Alert(self.flight.position))?;
            }
            self.trail = false;
        }
        if contact == Contact::Ground {
            if self.kind == Kind::Kunai {
                self.trail = false;
                return Ok(false);
            }
            self.phase = Phase::Resting;
            self.rotation[2] = 0;
            self.transform.rotation = super::facing(self.flight.velocity, 4096, context.math);
            let slot = context
                .collision
                .get_mut(model)
                .ok_or("Missing Tenchu 2 projectile collider")?;
            slot.flags &= 0xbfbf;
            slot.owner = None;
            self.trail = false;
        }
        self.transform.translation = self.flight.position;
        context.scene.update(model, self.transform);
        if contact == Contact::Ground {
            return Ok(true);
        }
        let own_index = context
            .collision
            .index(model)
            .ok_or("Missing Tenchu 2 projectile collider")?;
        for hit in context.collision.contacts(model) {
            let slot = context
                .collision
                .slot(hit.index)
                .ok_or("Missing Tenchu 2 projectile contact")?;
            if slot.flags & BODY != 0 {
                if slot.owner != Some(self.owner) {
                    emit(Event::Sound {
                        position: self.flight.position,
                        id: 50,
                    })?;
                    return Ok(false);
                }
            } else if slot.flags & WEAPON != 0 {
                if (slot.flags & 0x40 != 0 && own_index < hit.index) || slot.flags & 0x1000 == 0 {
                    let other = context.scene.world(slot.model)?.translation;
                    let position = self.flight.position.wrapping_add(other).map(|v| v >> 1);
                    self.impact(
                        position,
                        self.flight.velocity,
                        31,
                        context.player,
                        &mut emit,
                    )?;
                }
                return Ok(false);
            } else if slot.flags & 0x1000 != 0 && slot.flags & 0x200 == 0 {
                if self.flight.speed != 0 {
                    self.impact(
                        self.flight.position,
                        self.flight.velocity,
                        48,
                        context.player,
                        &mut emit,
                    )?;
                }
                self.flight.speed = 0;
            }
        }
        Ok(true)
    }

    fn impact(
        &self,
        position: Position,
        direction: Position,
        id: u8,
        player: usize,
        emit: &mut impl FnMut(Event) -> Result<()>,
    ) -> Result<()> {
        emit(Event::Impact {
            position,
            direction,
        })?;
        if self.owner == player {
            emit(Event::Alert(self.flight.position))?;
        }
        emit(Event::Sound {
            position: self.flight.position,
            id,
        })?;
        Ok(())
    }

    fn settled(&mut self, model: ModelId, context: &mut Context<'_>) -> Result<bool> {
        let phase = self.phase;
        if phase == Phase::Resting {
            let probe = self.flight.position.wrapping_sub(Position::new(0, 300, 0));
            if let Some(floor) =
                terrain::level(context.map, context.cursor, probe, Query::Uncached)?
            {
                self.flight.floor = floor;
                if floor != self.flight.position.y {
                    self.phase = Phase::Falling;
                }
            }
        }
        self.transform.translation = self.flight.position;
        context.scene.update(model, self.transform);
        if phase == Phase::Falling {
            let blocked = context.collision.contacts(model).any(|hit| {
                context
                    .collision
                    .slot(hit.index)
                    .is_some_and(|slot| slot.flags & BODY == 0)
            });
            if blocked {
                self.phase = Phase::Resting;
            } else {
                self.flight.position.y = self.flight.position.y.wrapping_add(180);
                if self.flight.floor < self.flight.position.y {
                    self.flight.position.y = self.flight.floor;
                    self.phase = Phase::Resting;
                }
            }
        }
        Ok(true)
    }
}
