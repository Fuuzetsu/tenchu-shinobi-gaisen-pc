use super::Context;
use crate::{
    Result,
    math::{Matrix, Position},
    scene::ModelId,
    tenchu2::{
        collision::Collision,
        terrain::{self, Query},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Marker {
    pub owner: usize,
    pub color: u8,
    pub position: Position,
    pub floor: Option<i32>,
}

impl Marker {
    pub fn place(
        owner: usize,
        color: u8,
        mut position: Position,
        yaw: i16,
        context: &mut Context<'_>,
    ) -> Result<Self> {
        let [sin, cos] = context.math.pair(yaw);
        position.x = position.x.wrapping_add(-sin >> 3);
        position.z = position.z.wrapping_add(-cos >> 3);
        let floor = terrain::level(context.map, context.cursor, position, Query::Uncached)?
            .filter(|_| !water(context));
        if let Some(floor) = floor {
            position.y = floor;
        }
        Ok(Self {
            owner,
            color,
            position,
            floor,
        })
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        if self.floor.is_some() {
            let slot = collision.insert(model)?;
            slot.size = [200, 400, 200].into();
            slot.flags = 0x120c;
        }
        Ok(())
    }

    pub fn advance(&mut self, model: ModelId, context: &mut Context<'_>) -> Result<bool> {
        let mut probe = self.position;
        probe.y = probe.y.wrapping_sub(300);
        let mut alive = true;
        if let Some(floor) = terrain::level(context.map, context.cursor, probe, Query::Uncached)? {
            self.floor = Some(floor);
            self.position.y = floor;
            alive = !water(context);
        }
        context.scene.update(
            model,
            Matrix {
                translation: self.position,
                ..Default::default()
            },
        );
        Ok(alive)
    }
}

fn water(context: &Context<'_>) -> bool {
    context
        .cursor
        .current
        .is_some_and(|id| context.map.cells[id.cell].nodes[id.node].attributes & 0x200 != 0)
}
