use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
};
use crate::{
    Result,
    math::{Math, Matrix, Position},
    scene::ModelId,
    tenchu2::{collision::Collision, effects::ribbon::Sample, visibility::View},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dart {
    pub owner: usize,
    pub flight: Flight,
    pub distance: i32,
    pub trail: bool,
}

impl Dart {
    pub fn launch(owner: usize, origin: Position, direction: Position, math: &Math) -> Self {
        Self {
            owner,
            flight: Flight {
                position: origin,
                velocity: math
                    .normalize_vector(direction)
                    .map(|v| v.wrapping_mul(700) >> 12),
                speed: 700,
                ..Default::default()
            },
            distance: 0,
            trail: true,
        }
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.flags = 0x5008;
        slot.owner = Some(self.owner);
        slot.size = [400; 3].into();
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        view: View,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        let (contact, splash) =
            self.flight
                .advance(Motion::Ballistic, context.map, context.cursor, context.math)?;
        if let Some(position) = splash {
            emit(Event::Splash(position))?;
        }
        if contact == Contact::Outside {
            return Ok(false);
        }
        self.distance = self.distance.wrapping_add(700);
        if contact != Contact::Air || self.flight.speed == 0 || self.distance > view.distance {
            return Ok(false);
        }
        context.scene.update(
            model,
            Matrix {
                translation: self.flight.position,
                ..Default::default()
            },
        );
        if self.trail {
            let sample =
                Sample::projectile(&view.matrix, self.flight.position, self.flight.velocity, 4);
            self.trail = sample.is_some();
            emit(Event::Ribbon { model, sample })?;
        }
        for hit in context.collision.contacts(model) {
            let slot = context
                .collision
                .slot(hit.index)
                .ok_or("Missing Tenchu 2 dart contact")?;
            if slot.owner != Some(self.owner) && slot.flags & 0x200 == 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
