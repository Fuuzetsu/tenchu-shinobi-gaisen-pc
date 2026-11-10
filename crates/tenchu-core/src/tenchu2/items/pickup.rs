use super::{Context, flight::Flight};
use crate::{
    Result,
    math::{Matrix, Position, Vector3},
    scene::ModelId,
    tenchu2::collision::Collision,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pickup {
    pub item: u8,
    pub flight: Flight,
    pub transform: Matrix,
    pub falling: bool,
}

impl Pickup {
    pub fn new(item: u8, position: Position) -> Self {
        Self {
            item,
            flight: Flight {
                position,
                floor: position.y,
                ..Default::default()
            },
            transform: Matrix {
                translation: position,
                ..Default::default()
            },
            falling: false,
        }
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(400, 400, 400);
        slot.owner = None;
        slot.flags = 0x1004;
        Ok(())
    }

    pub fn advance(&mut self, model: ModelId, context: &mut Context<'_>) -> Result<()> {
        self.falling = self
            .flight
            .settle(self.falling, model, &mut self.transform, context)?;
        Ok(())
    }
}
