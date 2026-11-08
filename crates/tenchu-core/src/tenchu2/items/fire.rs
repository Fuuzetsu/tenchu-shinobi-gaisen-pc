use super::Event;
use crate::{
    Result,
    math::{Math, Position},
    scene::ModelId,
    tenchu2::{collision::Collision, terrain::Cursor},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Breath {
    pub owner: usize,
    pub position: Position,
    pub floor: i32,
    pub age: i32,
}

impl Breath {
    pub fn begin(
        owner: usize,
        origin: Position,
        yaw: i16,
        map: &tenchu_assets::tenchu2::collision::Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<Self> {
        let position = super::surface_ahead(origin, yaw, map, cursor, math)?;
        Ok(Self {
            owner,
            position,
            floor: position.y,
            age: 0,
        })
    }

    pub fn advance(
        &mut self,
        yaw: i16,
        math: &Math,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        if previous >= 16 {
            return Ok(false);
        }
        if self.age & 3 == 3 {
            for offset in [0, -682, 682] {
                let [sin, cos] = math.pair(yaw.wrapping_add(offset));
                emit(Event::FireBlast {
                    owner: self.owner,
                    position: self.position.wrapping_add(Position::new(
                        -sin >> 1,
                        (self.age >> 2).wrapping_mul(-1024).wrapping_add(1024),
                        -cos >> 1,
                    )),
                })?;
            }
            if self.age >> 2 == 0 {
                emit(Event::Sound {
                    position: self.position,
                    id: 51,
                })?;
            }
        }
        Ok(true)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blast {
    pub owner: usize,
    pub position: Position,
    pub age: i32,
}

impl Blast {
    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = [400; 3].into();
        slot.owner = Some(self.owner);
        slot.flags = 0x5106;
        Ok(())
    }

    pub fn advance(&mut self, model: ModelId, collision: &mut Collision) -> Result<bool> {
        self.age = self.age.wrapping_add(1);
        collision
            .get_mut(model)
            .ok_or("Missing Tenchu 2 fire-blast collider")?
            .size = [(self.age as i16).wrapping_mul(512); 3].into();
        Ok(self.age <= 3)
    }
}
