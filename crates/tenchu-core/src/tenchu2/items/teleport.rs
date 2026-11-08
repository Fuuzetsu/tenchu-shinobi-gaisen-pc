use crate::{
    Result,
    math::{Math, Position},
    tenchu2::{
        actor::{Actor, State},
        terrain::{self, Query},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Teleport {
    pub owner: usize,
    pub target: Option<usize>,
    pub age: i32,
}

pub fn nearest<'a>(
    owner_index: usize,
    owner: &Actor,
    actors: impl Iterator<Item = (usize, &'a Actor)>,
    player: usize,
    range: i32,
    math: &Math,
) -> Option<usize> {
    let mut target = None;
    let mut distance = i32::MAX;
    for (index, actor) in actors {
        if index == player
            || index == owner_index
            || actor.flags & 0x100008 != 0x100000
            || actor.state == State::Dead
            || matches!(actor.kind & 0xf0, 0 | 0x10 | 0x60)
        {
            continue;
        }
        let delta = actor.position.wrapping_sub(owner.position);
        if delta.y.wrapping_abs() > 4000 {
            continue;
        }
        let current = math.length(delta);
        if current < distance {
            distance = current;
            target = Some(index);
        }
    }
    (distance <= range.wrapping_abs())
        .then_some(target)
        .flatten()
}

impl Teleport {
    pub fn advance(
        &mut self,
        owner: &mut Actor,
        target: Option<(Position, i16)>,
        camera: &mut i16,
        context: &mut super::Context<'_>,
    ) -> Result<bool> {
        if let Some((position, yaw)) = target
            && owner.flags & 0x80000 == 0
        {
            self.age = self.age.wrapping_add(1);
            if self.age != 96 {
                return Ok(true);
            }
            let [sin, cos] = context.math.pair(yaw);
            let position = position.wrapping_sub(Position::new(-sin >> 2, 0, -cos >> 2));
            if terrain::level(context.map, context.cursor, position, Query::Surface)?.is_some() {
                owner.position = position;
                owner.rotation.y = yaw;
                *camera = 21;
            }
        }
        Ok(false)
    }
}
