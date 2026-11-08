use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    scene::ModelId,
    tenchu2::collision::{BODY, Collision, WEAPON},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Flying,
    Flaming,
    Embedded { age: i32, parent: Option<ModelId> },
    Exploding(i32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arrow {
    pub owner: Option<usize>,
    pub flight: Flight,
    pub transform: Matrix,
    pub rotation: [i16; 3],
    pub phase: Phase,
    pub fire_model: bool,
    pub flame: bool,
}

#[derive(Clone, Copy)]
pub enum FireAim {
    Player,
    Guard,
    Blocked,
}

impl Arrow {
    pub fn launch(
        origin: Position,
        direction: Position,
        distance: i32,
        math: &Math,
        random: &mut Random,
    ) -> Self {
        let velocity = launch_velocity(direction, distance, true, math, random);
        let backward = math.normalize_vector(velocity.map(i32::wrapping_neg));
        Self {
            owner: None,
            flight: Flight {
                position: origin,
                velocity,
                speed: 400,
                gravity: 4,
                ..Default::default()
            },
            transform: Matrix {
                rotation: orientation(backward, velocity, math),
                translation: origin,
            },
            rotation: [0; 3],
            phase: if distance == i32::MIN {
                Phase::Flaming
            } else {
                Phase::Flying
            },
            fire_model: distance == i32::MIN,
            flame: false,
        }
    }

    pub fn fire(
        owner: usize,
        origin: Position,
        direction: Position,
        distance: i32,
        aim: FireAim,
        math: &Math,
        random: &mut Random,
    ) -> Self {
        let mut arrow = Self {
            owner: Some(owner),
            flight: Flight {
                position: origin,
                gravity: 4,
                ..Default::default()
            },
            transform: Matrix {
                translation: origin,
                ..Default::default()
            },
            rotation: [0; 3],
            phase: Phase::Flaming,
            fire_model: true,
            flame: false,
        };
        if !matches!(aim, FireAim::Blocked) {
            let backward = math.normalize_vector(direction).map(i32::wrapping_neg);
            arrow.flight.velocity = launch_velocity(
                direction,
                distance,
                matches!(aim, FireAim::Guard),
                math,
                random,
            );
            arrow.flight.speed = 400;
            arrow.transform.rotation = orientation(backward, arrow.flight.velocity, math);
        }
        arrow
    }

    pub fn model(&self) -> u8 {
        if self.fire_model { 9 } else { 22 }
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(400, 200, 400);
        slot.owner = self.owner;
        slot.flags = if self.fire_model { 0x5049 } else { 0x5056 };
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        tick: u32,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        self.flame = false;
        match &mut self.phase {
            Phase::Embedded { age, .. } => {
                let previous = *age;
                *age = age.wrapping_add(1);
                context.scene.update(model, self.transform);
                return Ok(previous <= 150);
            }
            Phase::Exploding(age) => {
                let slot = context
                    .collision
                    .get_mut(model)
                    .ok_or("Missing Tenchu 2 fire-arrow collider")?;
                slot.size = Vector3::from([(*age as i16).wrapping_shl(9); 3]);
                *age = age.wrapping_add(1);
                return Ok(*age < 5);
            }
            _ => {}
        }
        let (contact, splash) =
            self.flight
                .advance(Motion::Ballistic, context.map, context.cursor, context.math)?;
        if let Some(position) = splash {
            emit(Event::Splash(position))?;
        }
        if contact == Contact::Outside {
            return Ok(false);
        }
        if self.phase == Phase::Flaming {
            return self.burning(model, tick, contact, context, &mut emit);
        }
        if contact == Contact::Ground || self.flight.speed == 0 {
            emit(Event::Sound {
                position: self.flight.position,
                id: 48,
            })?;
            self.phase = Phase::Embedded {
                age: 0,
                parent: None,
            };
            let slot = context
                .collision
                .get_mut(model)
                .ok_or("Missing Tenchu 2 arrow collider")?;
            slot.flags &= 0xbfbf;
            context.collision.remove(model);
            self.publish(model, context);
            return Ok(true);
        }
        self.orient(context.math);
        self.publish(model, context);
        let own_index = context
            .collision
            .index(model)
            .ok_or("Missing Tenchu 2 arrow collider")?;
        let flight_rotation = self.transform.rotation;
        let contacts: Vec<_> = context.collision.contacts(model).collect();
        for hit in contacts {
            let slot = context
                .collision
                .slot(hit.index)
                .ok_or("Missing Tenchu 2 arrow contact")?;
            let flags = slot.flags;
            if flags & BODY != 0 {
                emit(Event::Sound {
                    position: self.flight.position,
                    id: 50,
                })?;
                let parent = slot.model;
                // The hit bone has already been transformed by collision; use that frame's cache.
                let world = context.scene.models[parent].cached;
                let inverse = Matrix {
                    rotation: std::array::from_fn(|i| {
                        std::array::from_fn(|j| world.rotation[j][i])
                    }),
                    ..Default::default()
                };
                for (row, flight) in self.transform.rotation.iter_mut().zip(flight_rotation) {
                    row[0] = flight[0].wrapping_neg();
                    row[2] = flight[2].wrapping_neg();
                }
                self.transform.rotation = inverse.compose(&self.transform).rotation;
                self.flight.position = Position::new(
                    0,
                    self.flight
                        .position
                        .y
                        .wrapping_sub(world.translation.y)
                        .clamp(-512, 512),
                    -200,
                );
                self.transform.translation = self.flight.position;
                context.scene.models[model].parent = Some(parent);
                self.phase = Phase::Embedded {
                    age: 0,
                    parent: Some(parent),
                };
                self.rotation[1] = 2048;
                context
                    .collision
                    .slot_mut(own_index)
                    .ok_or("Missing Tenchu 2 arrow collider")?
                    .flags &= 0xbfff;
                context.collision.remove(model);
            } else if flags & WEAPON != 0 {
                if (flags & 0x40 != 0 && own_index < hit.index) || flags & 0x1000 == 0 {
                    emit(Event::Impact {
                        position: self.flight.position,
                        direction: self.flight.velocity,
                    })?;
                    emit(Event::Sound {
                        position: self.flight.position,
                        id: 31,
                    })?;
                }
                return Ok(false);
            } else if flags & 0x200 == 0 {
                self.flight.speed = 0;
            }
        }
        Ok(true)
    }

    fn orient(&mut self, math: &Math) {
        let normal = math.normalize_vector(self.flight.velocity);
        self.transform.rotation = orientation(
            normal.map(i32::wrapping_neg),
            normal.map(i32::wrapping_neg),
            math,
        );
    }

    fn publish(&mut self, model: ModelId, context: &mut Context<'_>) {
        self.transform.translation = self.flight.position;
        context.scene.update(model, self.transform);
    }

    fn burning(
        &mut self,
        model: ModelId,
        tick: u32,
        contact: Contact,
        context: &mut Context<'_>,
        emit: &mut impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        let own_index = context
            .collision
            .index(model)
            .ok_or("Missing Tenchu 2 fire-arrow collider")?;
        let contacts: Vec<_> = context.collision.contacts(model).collect();
        if self.flight.submerged {
            self.phase = Phase::Flying;
        }
        let mut alive = true;
        if contact == Contact::Ground || self.flight.speed == 0 {
            if self.flight.submerged {
                context.collision.remove(model);
                alive = false;
            } else {
                self.flight.submerged = true;
                self.explode(own_index, false, context, emit)?;
            }
        }
        self.orient(context.math);
        self.publish(model, context);
        self.flame = true;
        if tick & 3 == 0 {
            emit(Event::Smoke {
                position: self.flight.position,
                kind: 3,
            })?;
        }
        for hit in contacts {
            if context
                .collision
                .slot(hit.index)
                .is_some_and(|s| s.flags & 0x200 == 0)
            {
                self.explode(own_index, true, context, emit)?;
            }
        }
        Ok(alive)
    }

    fn explode(
        &mut self,
        index: usize,
        contact: bool,
        context: &mut Context<'_>,
        emit: &mut impl FnMut(Event) -> Result<()>,
    ) -> Result<()> {
        context
            .collision
            .slot_mut(index)
            .ok_or("Missing Tenchu 2 fire-arrow collider")?
            .flags |= 0x100;
        if self.phase != Phase::Flying {
            self.phase = Phase::Exploding(1);
        }
        emit(Event::Explosion {
            kind: 1,
            position: self.flight.position,
            contact,
        })
    }
}

fn launch_velocity(
    direction: Position,
    distance: i32,
    scattered: bool,
    math: &Math,
    random: &mut Random,
) -> Position {
    let mut velocity = math
        .normalize_vector(direction)
        .map(|v| v.wrapping_mul(400) >> 12);
    if distance < 0 {
        velocity.y = velocity.y.wrapping_sub(100);
    } else if !scattered || distance > 2048 {
        let distance = if scattered {
            distance
                .wrapping_sub(random.next15() & 8191)
                .wrapping_sub(2000)
        } else {
            distance
        };
        let speed = math.square_root(
            velocity
                .x
                .wrapping_mul(velocity.x)
                .wrapping_add(velocity.z.wrapping_mul(velocity.z)),
        );
        if speed != 0 {
            velocity.y = velocity
                .y
                .wrapping_sub((distance.wrapping_shl(4) >> 3).wrapping_div(speed));
        }
    }
    velocity
}

fn orientation(backward: Position, velocity: Position, math: &Math) -> [[i16; 3]; 3] {
    let up = if velocity.x == 0 && velocity.z == 0 {
        Position::new(0, 0, velocity.y.wrapping_neg())
    } else {
        Position::new(0, -4096, 0)
    };
    super::basis(backward, up, math)
}
