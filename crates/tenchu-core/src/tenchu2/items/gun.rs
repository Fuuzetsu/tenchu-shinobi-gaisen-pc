use super::{Context, Event, aim};
use crate::{
    Result,
    math::{Math, Position, Vector3},
    scene::ModelId,
    tenchu2::{
        collision::{BODY, Collision},
        terrain::{self, Cursor, Query},
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision as Map;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gunshot {
    pub owner: usize,
    pub position: Position,
    pub direction: Position,
    pub distance: i32,
    pub age: i32,
}

impl Gunshot {
    pub fn launch(
        owner: usize,
        origin: Position,
        direction: Position,
        distance: Option<i32>,
        map: &Map,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<Self> {
        let direction = math.normalize_vector(direction);
        let step = direction.map(|v| (v >> 4) as i16);
        let mut distance = distance.map(|d| d.wrapping_sub(600).max(0));
        let hit = if let Some(distance) = distance {
            aim::wall(
                map,
                cursor,
                origin,
                step,
                (distance.wrapping_shl(8) >> 16) as i16,
                math,
            )?
        } else {
            terrain::traverse(map, cursor, origin, step, -1, math)?
        };
        let position = if let Some(hit) = hit {
            distance = Some(0);
            hit
        } else {
            origin.wrapping_add(direction.map(|v| v.wrapping_mul(distance.unwrap_or(16000)) >> 12))
        };
        Ok(Self {
            owner,
            position,
            direction,
            distance: distance.unwrap_or(16000),
            age: 0,
        })
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(800, 1200, 800);
        slot.owner = Some(self.owner);
        slot.flags = 0x501d;
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
        if previous <= 1 {
            return Ok(true);
        }
        let mut contacts = context.collision.contacts(model).peekable();
        if contacts.peek().is_none() {
            if self.distance != 0
                && let Some(floor) =
                    terrain::level(context.map, context.cursor, self.position, Query::Surface)?
            {
                self.position.y = floor;
            }
            emit(Event::Impact {
                position: self.position,
                direction: Position::new(
                    self.direction.x.wrapping_neg(),
                    self.direction.y,
                    self.direction.z.wrapping_neg(),
                ),
            })?;
            emit(Event::Sound {
                position: self.position,
                id: 59,
            })?;
            emit(Event::Alert(self.position))?;
        } else {
            for hit in contacts {
                if context
                    .collision
                    .slot(hit.index)
                    .is_some_and(|s| s.flags & BODY != 0)
                {
                    emit(Event::Sound {
                        position: self.position,
                        id: 50,
                    })?;
                }
            }
        }
        Ok(false)
    }
}
