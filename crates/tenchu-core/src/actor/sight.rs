use super::{Actor, Context};
use crate::math::Position;
use crate::{Result, terrain};

#[cfg(test)]
#[path = "sight_tests.rs"]
mod tests;

struct Query {
    position: Position,
    delta: Position,
    height: i16,
    player_height: i16,
    player_status: i16,
    clock: i32,
    slot: i32,
    alert: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detection {
    Gone,
    Unseen,
    None,
    Seen,
    Glimpse,
}

#[cfg(test)]
impl Detection {
    pub(crate) fn source_value(self) -> i32 {
        match self {
            Self::Gone => -2,
            Self::Unseen => -1,
            Self::None => 0,
            Self::Seen => 1,
            Self::Glimpse => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Sight {
    pub distance: i32,
    pub direction: i16,
}

pub struct Search<'a> {
    pub owner: &'a Actor,
    pub target: Option<Position>,
    pub player: crate::ai::Player,
    pub clock: i32,
    pub alert: i32,
    pub range_percent: u16,
    pub all_active: bool,
}

pub struct Profile {
    pub sight: i32,
    pub clear: i32,
    pub far: i32,
    pub half_angle: i16,
}

pub fn profile(sneaking: bool, percent: u16) -> Profile {
    let (sight, clear, far, half_angle) = if sneaking {
        (12000, 7000, 16000, 450)
    } else {
        (16000, 10000, 20000, 900)
    };
    let scale = |distance| distance * i32::from(percent.min(400)) / 100;
    Profile {
        sight: scale(sight),
        clear: scale(clear),
        far: scale(far),
        half_angle,
    }
}

impl Sight {
    pub fn search(&mut self, search: Search<'_>, ctx: &mut Context) -> Result<Detection> {
        let Search {
            owner,
            target,
            player,
            clock,
            alert,
            range_percent,
            all_active,
        } = search;
        let Some(target) = target else {
            return Ok(Detection::None);
        };
        let position = owner.position;
        let delta: Position = target.wrapping_sub(position);
        self.distance = if all_active {
            ctx.math.length_wide(delta)
        } else {
            ctx.math.length(delta)
        };
        self.direction = ctx.math.direction(delta.x, delta.z, owner.rotation[1])?;
        let slot = ctx.scene.models[owner.parts[0]]
            .conflict
            .map_or(-1, |s| s as i32);
        self.evaluate_range(
            Query {
                position,
                delta,
                height: ctx.mission.characters[owner.definition].height,
                player_height: player.height,
                player_status: player.status,
                clock,
                slot,
                alert,
            },
            range_percent,
            |p, d, n| Ok(terrain::passage(&ctx.mission.terrain, ctx.cursor, p, d, n)?.is_some()),
        )
    }

    #[cfg(test)]
    fn evaluate(
        &self,
        query: Query,
        blocked: impl FnMut(Position, [i16; 3], i16) -> Result<bool>,
    ) -> Result<Detection> {
        self.evaluate_range(query, 100, blocked)
    }

    fn evaluate_range(
        &self,
        query: Query,
        range_percent: u16,
        mut blocked: impl FnMut(Position, [i16; 3], i16) -> Result<bool>,
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
        if (player_status == 10 && (delta.y >= 0 || (delta.y < -3000 && self.distance < 4000)))
            || (delta.y.wrapping_abs() >= 3000 && (alert == 0 || self.distance < 4000))
        {
            return Ok(Detection::Gone);
        }
        let Profile {
            sight, clear, far, ..
        } = profile(sneaking, range_percent);
        let limit = if sneaking { 300 } else { 500 };
        if self.distance >= far {
            return Ok(Detection::Gone);
        }
        let degree = i32::from(self.direction).abs();
        if degree >= 900 || (sneaking && degree >= 450) || self.distance >= sight {
            return Ok(Detection::Unseen);
        }
        let height = i32::from(height);
        let player_height = i32::from(player_height);
        position.y = position.y.wrapping_add(300 - height);
        delta.y =
            delta
                .y
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
        if blocked(position, delta.map(|v| v as i16).to_array(), count)? {
            return Ok(Detection::Gone);
        }
        Ok(if self.distance < clear {
            Detection::Seen
        } else {
            Detection::Glimpse
        })
    }
}
