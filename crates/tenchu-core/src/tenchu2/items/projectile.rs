use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    scene::ModelId,
    tenchu2::collision::{BODY, Collision, WEAPON},
};
use serde::{Deserialize, Serialize};

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
        view: &Matrix,
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
        if self.flight.speed != 0 && self.trail {
            let sample = crate::tenchu2::effects::ribbon::Sample::projectile(
                view,
                self.flight.position,
                self.flight.velocity,
                if self.kind == Kind::Shuriken { 5 } else { 6 },
            );
            self.trail = sample.is_some();
            emit(Event::Ribbon { model, sample })?;
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
        self.phase = if self.flight.settle(
            self.phase == Phase::Falling,
            model,
            &mut self.transform,
            context,
        )? {
            Phase::Falling
        } else {
            Phase::Resting
        };
        Ok(true)
    }
}
