use crate::{
    Result,
    math::{Math, Position},
    scene::ModelId,
    tenchu2::{
        collision::Collision,
        terrain::{self, Cursor, Query},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Powder {
    pub owner: usize,
    pub position: Position,
    pub floor: i32,
    pub age: i32,
}

impl Powder {
    pub fn blow(
        owner: usize,
        mut origin: Position,
        yaw: i16,
        map: &tenchu_assets::tenchu2::collision::Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<Self> {
        let floor = terrain::level(map, cursor, origin, Query::Surface)?.unwrap_or(i32::MIN);
        let [sin, cos] = math.pair(yaw);
        origin.y = floor;
        origin.x = origin.x.wrapping_add(-sin >> 3);
        origin.z = origin.z.wrapping_add(-cos >> 3);
        Ok(Self {
            owner,
            position: origin,
            floor,
            age: 0,
        })
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.flags = 0x500e;
        slot.size = [1500, 4000, 1500].into();
        slot.owner = Some(self.owner);
        Ok(())
    }

    pub fn advance(&mut self) -> bool {
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        previous <= 30
    }
}
