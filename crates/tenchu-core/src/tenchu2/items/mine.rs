use super::{Context, Event};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    scene::ModelId,
    tenchu2::{
        collision::{BODY, Collision, WEAPON},
        terrain::{self, Cursor, Query},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mine {
    pub owner: usize,
    pub transform: Matrix,
    pub floor: i32,
    pub age: i32,
}

impl Mine {
    pub fn place(
        owner: usize,
        yaw: i16,
        mut origin: Position,
        map: &tenchu_assets::tenchu2::collision::Collision,
        cursor: &mut Cursor,
        math: &Math,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<Option<Self>> {
        let [sin, cos] = math.pair(yaw);
        origin.x = origin.x.wrapping_add(-sin >> 3);
        origin.z = origin.z.wrapping_add(-cos >> 3);
        let floor = terrain::level(
            map,
            cursor,
            origin.wrapping_sub(Position::new(0, 1000, 0)),
            Query::Uncached,
        )?;
        let wet = cursor.current.is_some_and(|surface| {
            map.cells[surface.cell].nodes[surface.node].attributes & 0x200 != 0
        });
        let Some(floor) = floor.filter(|_| !wet) else {
            emit(Event::Sound {
                position: origin,
                id: 6,
            })?;
            return Ok(None);
        };
        origin.y = floor;
        Ok(Some(Self {
            owner,
            transform: Matrix {
                translation: origin,
                ..Default::default()
            },
            floor,
            age: 0,
        }))
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(400, 400, 400);
        slot.owner = None;
        slot.flags = 0x5007;
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        let mut alive = true;
        if let Some(floor) = terrain::level(
            context.map,
            context.cursor,
            self.transform
                .translation
                .wrapping_sub(Position::new(0, 300, 0)),
            Query::Uncached,
        )? {
            self.floor = floor;
            self.transform.translation.y = floor;
            if context
                .cursor
                .current
                .is_some_and(|s| context.map.cells[s.cell].nodes[s.node].attributes & 0x200 != 0)
            {
                alive = false;
            }
        }
        context.scene.update(model, self.transform);
        if self.age == 0 {
            let triggered = context.collision.contacts(model).any(|hit| {
                context.collision.slot(hit.index).is_some_and(|slot| {
                    slot.flags & BODY != 0 || (slot.flags & WEAPON != 0 && slot.flags & 0x140 != 0)
                })
            });
            if triggered {
                emit(Event::Explosion {
                    position: self.transform.translation,
                    kind: 0,
                    contact: false,
                })?;
                context
                    .collision
                    .get_mut(model)
                    .ok_or("Missing Tenchu 2 mine collider")?
                    .flags |= 0x100;
                self.age = 1;
            }
        } else {
            let size = self.age.wrapping_mul(1000) as i16;
            context
                .collision
                .get_mut(model)
                .ok_or("Missing Tenchu 2 mine collider")?
                .size = Vector3::new(size, size, size);
            self.age = self.age.wrapping_add(1);
            alive &= self.age <= 4;
        }
        Ok(alive)
    }
}
