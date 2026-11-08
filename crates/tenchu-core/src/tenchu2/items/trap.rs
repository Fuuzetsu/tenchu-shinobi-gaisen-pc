use super::{
    Context, Event,
    flight::{Flight, Motion},
};
use crate::{
    Result,
    math::{Math, Matrix, Position},
    scene::ModelId,
    tenchu2::collision::{BODY, Collision},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spear {
    pub flight: Flight,
    pub transform: Matrix,
    pub age: i32,
}

impl Spear {
    pub fn launch(position: Position, direction: Position, math: &Math) -> Self {
        let velocity = math
            .normalize_vector(direction)
            .map(|v| v.wrapping_mul(400) >> 12);
        let backward = math.normalize_vector(velocity.map(i32::wrapping_neg));
        Self {
            flight: Flight {
                position,
                velocity,
                speed: 400,
                ..Default::default()
            },
            transform: Matrix {
                rotation: super::arrow::orientation(backward, velocity, math),
                translation: position,
            },
            age: 0,
        }
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.flags = 0x5016;
        slot.size = [400, 200, 400].into();
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        let alive = previous <= 4 || self.age <= 39;
        if self.age == 35 {
            self.flight.velocity = self.flight.velocity.map(i32::wrapping_neg);
        } else if previous <= 4 || (36..=39).contains(&self.age) {
            let (_, splash) = self.flight.advance(
                Motion::Ballistic,
                context.map,
                context.cursor,
                context.math,
            )?;
            if let Some(position) = splash {
                emit(Event::Splash(position))?;
            }
        }
        self.transform.translation = self.flight.position;
        context.scene.update(model, self.transform);
        let hits = context
            .collision
            .contacts(model)
            .filter(|contact| {
                context
                    .collision
                    .slot(contact.index)
                    .is_some_and(|slot| slot.flags & BODY != 0)
            })
            .count();
        if hits != 0 {
            context.collision.remove(model);
            for _ in 0..hits {
                emit(Event::Sound {
                    position: self.flight.position,
                    id: 50,
                })?;
            }
        }
        Ok(alive)
    }
}
