use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
    projectile::Phase,
};
use crate::{
    Result,
    math::{Matrix, Vector3},
    random::Random,
    scene::ModelId,
    tenchu2::collision::{BODY, Collision},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grenade {
    pub owner: usize,
    pub flight: Flight,
    pub transform: Matrix,
    pub phase: Phase,
    pub age: i32,
    pub visible: bool,
}

impl Grenade {
    pub fn launch(owner: usize, flight: Flight) -> Self {
        Self {
            owner,
            transform: Matrix {
                translation: flight.position,
                ..Default::default()
            },
            flight,
            phase: Phase::Flying,
            age: 0,
            visible: true,
        }
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(400, 400, 400);
        slot.owner = Some(self.owner);
        slot.flags = 0x5006;
        Ok(())
    }

    pub fn ignite(&mut self, model: ModelId, collision: &mut Collision) -> Result<Event> {
        let slot = collision
            .get_mut(model)
            .ok_or("Missing Tenchu 2 grenade collider")?;
        slot.flags |= 0x4000;
        self.phase = Phase::Flying;
        self.explode(model, collision)
    }

    fn explode(&mut self, model: ModelId, collision: &mut Collision) -> Result<Event> {
        collision
            .get_mut(model)
            .ok_or("Missing Tenchu 2 grenade collider")?
            .flags |= 0x100;
        self.age = 256;
        Ok(Event::Explosion {
            position: self.flight.position,
            kind: 1,
            contact: true,
        })
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        context: &mut Context<'_>,
        random: &mut Random,
        mut emit: impl FnMut(Event, &mut Random) -> Result<()>,
    ) -> Result<bool> {
        if self.phase != Phase::Flying {
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
            return Ok(true);
        }
        let (contact, splash) =
            self.flight
                .advance(Motion::Ballistic, context.map, context.cursor, context.math)?;
        if let Some(position) = splash {
            emit(Event::Splash(position), random)?;
        }
        self.transform.translation = self.flight.position;
        if contact == Contact::Outside {
            self.visible = false;
            return Ok(false);
        }
        if self.age >= 256 {
            self.visible = false;
            let size = self.age.wrapping_sub(255).wrapping_mul(512) as i16;
            context
                .collision
                .get_mut(model)
                .ok_or("Missing Tenchu 2 grenade collider")?
                .size = Vector3::new(size, size, size);
            self.age = self.age.wrapping_add(1);
            return Ok(self.age < 260);
        }
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        if previous & 3 == 0 {
            emit(Event::Fuse(self.flight.position), random)?;
        }
        if contact == Contact::Ground {
            if self.flight.speed == 0 || self.flight.submerged {
                let slot = context
                    .collision
                    .get_mut(model)
                    .ok_or("Missing Tenchu 2 grenade collider")?;
                slot.owner = None;
                if !self.flight.submerged && random.next15() & 3 != 0 {
                    emit(self.explode(model, context.collision)?, random)?;
                } else {
                    self.phase = Phase::Resting;
                    self.age = 0;
                    slot.flags &= 0xbfff;
                }
            } else {
                self.flight.bounce();
            }
        }
        context.scene.update(model, self.transform);
        let contacts: Vec<_> = context.collision.contacts(model).collect();
        for hit in contacts {
            let slot = context
                .collision
                .slot(hit.index)
                .ok_or("Missing Tenchu 2 grenade contact")?;
            if slot.flags & (BODY | 0x100) != 0 {
                emit(self.explode(model, context.collision)?, random)?;
            } else if slot.flags & 0x200 == 0 {
                self.flight.speed = 0;
            }
        }
        Ok(true)
    }
}
