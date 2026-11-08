use super::Event;
use crate::{Result, math::Position, scene::ModelId, tenchu2::collision::Collision};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cloud {
    pub owner: Option<usize>,
    pub position: Position,
    pub direction: Position,
    pub age: i32,
}

impl Cloud {
    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.flags = if self.owner.is_some() { 0x5012 } else { 0x500a };
        slot.owner = self.owner;
        slot.size = [40, 20, 40].into();
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        collision: &mut Collision,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        self.age = self.age.wrapping_add(1);
        if self.age > 80 {
            return Ok(false);
        }
        let slot = collision
            .get_mut(model)
            .ok_or("Missing Tenchu 2 status-cloud collider")?;
        if self.age & 63 == 1 && self.age < 20 {
            emit(Event::DispersingSmoke {
                position: self.position,
                direction: self.direction,
                kind: match slot.flags & 63 {
                    10 => 2,
                    28 => 1,
                    _ => 0,
                },
            })?;
        }
        if (33..=64).contains(&self.age) {
            let width = (self.age as i16).wrapping_shl(6);
            slot.size = [width, 4000, width].into();
        }
        Ok(true)
    }
}
