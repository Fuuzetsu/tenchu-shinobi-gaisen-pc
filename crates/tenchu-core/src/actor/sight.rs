use super::{Actor, Context};
use crate::{Result, terrain};

#[cfg(test)]
#[path = "sight_tests.rs"]
mod tests;

struct Query {
    position: [i32; 3],
    delta: [i32; 3],
    height: i16,
    player_height: i16,
    player_status: i16,
    clock: i32,
    slot: i32,
    alert: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i16)]
pub enum Detection {
    Gone = -2,
    Unseen = -1,
    None = 0,
    Seen = 1,
    Glimpse = 2,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Sight {
    pub distance: i32,
    pub direction: i16,
}

impl Sight {
    pub fn search(
        &mut self,
        owner: &Actor,
        target: Option<[i32; 3]>,
        player: &Actor,
        clock: i32,
        alert: i32,
        ctx: &mut Context,
    ) -> Result<Detection> {
        let Some(target) = target else {
            return Ok(Detection::None);
        };
        let position = owner.position;
        let delta: [i32; 3] = std::array::from_fn(|i| target[i].wrapping_sub(position[i]));
        self.distance = ctx.math.square_root(
            delta
                .iter()
                .fold(0i32, |sum, &n| sum.wrapping_add(n.wrapping_mul(n))),
        );
        self.direction = ctx.math.direction(delta[0], delta[2], owner.rotation[1])?;
        let slot = ctx.scene.models[owner.parts[0]]
            .conflict
            .map_or(-1, |s| s as i32);
        self.evaluate(
            Query {
                position,
                delta,
                height: ctx.mission.characters[owner.definition].height,
                player_height: ctx.mission.characters[player.definition].height,
                player_status: player.status,
                clock,
                slot,
                alert,
            },
            |p, d, n| Ok(terrain::passage(&ctx.mission.terrain, ctx.cursor, p, d, n)?.is_some()),
        )
    }

    fn evaluate(
        &self,
        query: Query,
        mut blocked: impl FnMut([i32; 3], [i16; 3], i16) -> Result<bool>,
    ) -> Result<Detection> {
        let Query {
            mut position,
            mut delta,
            height,
            player_height,
            player_status,
            clock,
            slot,
            alert,
        } = query;
        if clock.wrapping_add(slot) & 31 != 0 {
            return Ok(Detection::None);
        }
        let sneaking = matches!(player_status, 11 | 12);
        if (player_status == 10 && (delta[1] >= 0 || (delta[1] < -3000 && self.distance < 4000)))
            || (delta[1].wrapping_abs() >= 3000 && (alert == 0 || self.distance < 4000))
        {
            return Ok(Detection::Gone);
        }
        let (sight, clear, far, limit) = if sneaking {
            (12000, 7000, 16000, 300)
        } else {
            (16000, 10000, 20000, 500)
        };
        if self.distance >= far {
            return Ok(Detection::Gone);
        }
        let degree = i32::from(self.direction).abs();
        if degree >= 900 || (sneaking && degree >= 450) || self.distance >= sight {
            return Ok(Detection::Unseen);
        }
        let height = i32::from(height);
        let player_height = i32::from(player_height);
        position[1] = position[1].wrapping_add(300 - height);
        delta[1] = delta[1]
            .wrapping_sub(300)
            .wrapping_add(height)
            .wrapping_sub(if player_status == 11 {
                player_height / 2
            } else {
                player_height
            });
        let mut count = 1i16;
        while delta.iter().any(|v| v.wrapping_abs() > limit) {
            count = count.wrapping_shl(1);
            delta = delta.map(|v| v >> 1);
        }
        if blocked(position, delta.map(|v| v as i16), count)? {
            return Ok(Detection::Gone);
        }
        Ok(if self.distance < clear {
            Detection::Seen
        } else {
            Detection::Glimpse
        })
    }
}
