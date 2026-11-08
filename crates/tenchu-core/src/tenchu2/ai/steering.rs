use super::Decision;
use crate::{Result, math::Math, random::Random, tenchu2::input};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForcedInput {
    pub held: u16,
    pub remaining: i16,
}

impl ForcedInput {
    fn from_ticks(held: u16, ticks: i32) -> Self {
        // The source ORs a signed counter into the packed input word.
        let packed = (u32::from(held) << 16) | ticks as u32;
        Self {
            held: (packed >> 16) as u16,
            remaining: packed as i16,
        }
    }

    pub(super) fn turn(held: u16, angle: i32, speed: i16, extra: i32) -> Result<Self> {
        let ticks = angle
            .checked_div(i32::from(speed))
            .ok_or("Tenchu 2 AI cannot turn at zero speed")?;
        Ok(Self::from_ticks(held, ticks.wrapping_add(extra)))
    }

    pub fn advance(self, random: &mut Random) -> (u16, Option<Self>) {
        let remaining = self.remaining.wrapping_sub(1);
        let next = if remaining != 0 {
            Some(Self::from_ticks(self.held, i32::from(remaining)))
        } else if self.held & (input::LEFT | input::RIGHT) != 0 {
            Some(Self::from_ticks(input::UP, (random.next15() % 3 + 1) * 30))
        } else {
            None
        };
        (self.held, next)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Probe {
    pub height: i32,
    pub material: u16,
}

pub struct SteeringInput {
    pub yaw: i16,
    pub turn: i16,
    pub target_angle: i16,
    pub alerted: bool,
    pub blocked: bool,
    pub forced: bool,
    pub ahead: Option<Probe>,
    pub left: bool,
    pub right: bool,
}

pub fn steer(x: i32, z: i32, actor: &SteeringInput, math: &Math) -> Result<Decision> {
    let angle = i32::from(if x == 0 && z == 0 {
        actor.target_angle
    } else {
        direction(math, x, z, actor.yaw)?
    });
    let turn = i32::from(actor.turn);
    let held = if angle > turn * 2 {
        input::RIGHT
    } else if angle < -turn * 2 {
        input::LEFT
    } else {
        0
    };
    let mut decision = Decision {
        held,
        ..Default::default()
    };
    if angle.abs() < turn * 3 {
        decision.held |= input::UP;
    }
    if !actor.alerted && actor.blocked && actor.ahead.is_none() && !actor.forced {
        let held = if decision.held & input::RIGHT != 0 && actor.right {
            input::RIGHT
        } else if decision.held & input::LEFT != 0 && actor.left {
            input::LEFT
        } else {
            decision.held = 0;
            return Ok(decision);
        };
        decision.forced = Some(ForcedInput::turn(held, 600, actor.turn, 1)?);
    }
    Ok(decision)
}

pub fn direction(math: &Math, x: i32, z: i32, yaw: i16) -> Result<i16> {
    let angle = math.atan2(x, z)?.wrapping_add(2048).wrapping_sub(yaw);
    Ok(if angle > 2048 {
        angle.wrapping_sub(4096)
    } else if angle <= -2048 {
        angle.wrapping_add(4096)
    } else {
        angle
    })
}
