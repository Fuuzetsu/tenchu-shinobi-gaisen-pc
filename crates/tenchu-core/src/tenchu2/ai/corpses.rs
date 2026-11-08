use super::{direction, local_alert_only};
use crate::{
    Result,
    math::Math,
    tenchu2::{
        actor::{Actor, State},
        terrain::{self, Cursor},
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{Edition, collision::Collision};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CorpseSearch {
    pub observer: usize,
}

pub struct CorpseContext<'a> {
    pub edition: Edition,
    pub player: usize,
    pub difficulty: u8,
    pub distance: i32,
    pub terrain: &'a Collision,
    pub cursor: &'a mut Cursor,
    pub math: &'a Math,
}

impl CorpseSearch {
    pub fn observe<'a>(
        &mut self,
        corpse: usize,
        count: usize,
        actor_at: impl Fn(usize) -> Option<&'a Actor>,
        world: &mut CorpseContext<'_>,
    ) -> Result<Option<usize>> {
        let body = actor_at(corpse).ok_or("Missing Tenchu 2 corpse")?;
        if body.motion.loops != i16::MIN || local_alert_only(body.kind) || body.flags & 0x80 != 0 {
            return Ok(None);
        }
        self.observer = (self.observer + 1) % count;
        let observer = actor_at(self.observer).ok_or("Missing Tenchu 2 corpse observer")?;
        if observer.flags & 0xb != 0
            || matches!(observer.state, State::Hurt | State::Dead)
            || observer.motion.action == 0x70c
            || self.observer == corpse
            || self.observer == world.player
            || local_alert_only(observer.kind)
            || world.difficulty == 3
        {
            return Ok(None);
        }
        let delta = body.position.wrapping_sub(observer.position);
        let angle = direction(world.math, delta.x, delta.z, observer.rotation.y)?;
        let cone = if world.edition == Edition::Europe {
            800
        } else {
            700
        };
        if i32::from(angle).abs() >= cone {
            return Ok(None);
        }
        let distance = if delta.x.wrapping_abs() < 32767 && delta.z.wrapping_abs() < 32767 {
            world.math.length(delta)
        } else {
            i32::MAX
        };
        if distance > world.distance || delta.y.wrapping_abs() >= distance.wrapping_mul(3) / 5 {
            return Ok(None);
        }
        let mut eye = observer.position;
        eye.y = eye.y.wrapping_sub(i32::from(observer.height) >> 1);
        world.cursor.current = None;
        Ok(terrain::segment(
            world.terrain,
            world.cursor,
            eye,
            body.position.wrapping_sub(eye),
            world.edition,
            world.math,
        )?
        .is_none()
        .then_some(self.observer))
    }
}
