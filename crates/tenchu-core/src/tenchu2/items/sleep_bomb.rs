use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
};
use crate::{Result, math::Matrix, scene::ModelId, tenchu2::collision::Collision};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SleepBomb {
    pub owner: usize,
    pub flight: Flight,
    pub transform: Matrix,
    pub age: i32,
}

impl SleepBomb {
    pub fn thrown(owner: usize, mut flight: Flight) -> Self {
        flight.gravity = 8;
        Self {
            owner,
            transform: Matrix {
                translation: flight.position,
                ..Default::default()
            },
            flight,
            age: 0,
        }
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.flags = 0x1212;
        slot.size = [400; 3].into();
        slot.owner = Some(self.owner);
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        if self.age == 0 {
            let (contact, splash) = self.flight.advance(
                Motion::Ballistic,
                context.map,
                context.cursor,
                context.math,
            )?;
            if let Some(position) = splash {
                emit(Event::Splash(position))?;
            }
            if contact == Contact::Outside {
                return Ok(false);
            }
            if contact == Contact::Ground {
                if self.flight.speed == 0 {
                    if !self.flight.submerged {
                        context
                            .collision
                            .get_mut(model)
                            .ok_or("Missing Tenchu 2 sleep-bomb collider")?
                            .flags = 0x5012;
                    }
                    self.age = 1;
                } else if self.flight.submerged {
                    self.age = 1;
                } else {
                    self.flight.bounce();
                }
            }
            if context.collision.contacts(model).any(|hit| {
                context
                    .collision
                    .slot(hit.index)
                    .is_some_and(|s| s.flags & 0x200 == 0)
            }) {
                self.flight.velocity.x = 0;
                self.flight.velocity.z = 0;
                self.flight.speed = 0;
            }
        } else {
            if self.age > 95 {
                return Ok(false);
            }
            if !self.flight.submerged {
                if self.age & 15 == 1 {
                    emit(Event::SleepGas {
                        position: self.flight.position,
                        phase: self.age as i16,
                    })?;
                }
                let radius = if self.age <= 32 {
                    (self.age << 7) as i16
                } else {
                    4000
                };
                context
                    .collision
                    .get_mut(model)
                    .ok_or("Missing Tenchu 2 sleep-bomb collider")?
                    .size = [radius, 4000, radius].into();
            }
            self.age = self.age.wrapping_add(1);
        }
        self.transform.translation = self.flight.position;
        context.scene.update(model, self.transform);
        Ok(true)
    }
}
