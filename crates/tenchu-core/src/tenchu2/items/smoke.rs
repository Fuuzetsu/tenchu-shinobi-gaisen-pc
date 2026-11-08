use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    scene::ModelId,
    tenchu2::{
        collision::Collision,
        terrain::{self, Cursor, Query},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Smoke {
    pub owner: usize,
    pub flight: Flight,
    pub transform: Matrix,
    pub age: i32,
}

impl Smoke {
    pub fn thrown(owner: usize, flight: Flight) -> Self {
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

    pub fn place(
        owner: usize,
        yaw: i16,
        origin: Position,
        map: &tenchu_assets::tenchu2::collision::Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<Option<Self>> {
        let [sin, cos] = math.pair(yaw);
        let mut position = origin.wrapping_add(Position::new((-sin * 3) >> 3, 0, (-cos * 3) >> 3));
        let floor = match terrain::level(map, cursor, position, Query::Surface)? {
            Some(floor) => Some(floor),
            None => {
                position = origin;
                terrain::level(map, cursor, position, Query::Surface)?
            }
        };
        let Some(floor) = floor else {
            return Ok(None);
        };
        position.y = floor;
        let submerged = cursor.current.is_some_and(|s| {
            matches!(
                map.cells[s.cell].nodes[s.node].attributes & 0xf00,
                0x200 | 0x300
            )
        });
        let mut smoke = Self::thrown(
            owner,
            Flight {
                position,
                floor,
                gravity: 16,
                submerged,
                ..Default::default()
            },
        );
        smoke.age = 1;
        Ok(Some(smoke))
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(40, 20, 40);
        slot.owner = Some(self.owner);
        slot.flags = if self.age != 0 && !self.flight.submerged {
            0x500a
        } else {
            0x100a
        };
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
                            .ok_or("Missing Tenchu 2 smoke collider")?
                            .flags = 0x500a;
                    }
                    self.age = 1;
                } else if self.flight.submerged {
                    self.age = 1;
                } else {
                    self.flight.bounce();
                }
            }
            self.transform.translation = self.flight.position;
            context.scene.update(model, self.transform);
            return Ok(true);
        }
        context.scene.update(model, self.transform);
        self.age = self.age.wrapping_add(1);
        if self.age > 80 {
            return Ok(false);
        }
        if self.flight.submerged {
            return Ok(true);
        }
        if self.age & 31 == 8 && self.age < 50 {
            emit(Event::Smoke {
                position: self.flight.position.wrapping_sub(Position::new(0, 600, 0)),
                kind: 2,
            })?;
        } else if self.age & 63 == 16 && self.age < 20 {
            emit(Event::Smoke {
                position: self.flight.position.wrapping_sub(Position::new(0, 1500, 0)),
                kind: 1,
            })?;
        }
        if self.age <= 64 {
            if self.age > 32 {
                let size = self.age.wrapping_mul(64).wrapping_sub(800) as i16;
                context
                    .collision
                    .get_mut(model)
                    .ok_or("Missing Tenchu 2 smoke collider")?
                    .size = Vector3::new(size, 4000, size);
            } else if self.age == 6 {
                emit(Event::Sound {
                    position: self.flight.position,
                    id: 52,
                })?;
            }
        }
        Ok(true)
    }
}
