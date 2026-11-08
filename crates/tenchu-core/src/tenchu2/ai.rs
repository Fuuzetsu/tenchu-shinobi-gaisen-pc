mod patrol;
pub use patrol::{Patrol, PatrolInput};
mod sentry;
pub use sentry::{Sentry, SentryInput};
mod sleep;
pub use sleep::{Sleep, SleepInput, SleepOutput};
mod corpses;
pub use corpses::{CorpseContext, CorpseSearch};
pub mod alert;
pub mod combat;
pub mod search;
pub mod sight;
mod steering;
pub use steering::{ForcedInput, Probe, SteeringInput, direction, steer};
mod wander;
pub use wander::{Wander, WanderInput};

use crate::{Result, random::Random};
use serde::{Deserialize, Serialize};

fn local_alert_only(kind: i16) -> bool {
    matches!(kind, 0x6a..=0x6e) || kind & 0xf0 == 0x10
}

#[derive(Default)]
pub struct Decision {
    pub held: u16,
    pub action: Option<i16>,
    pub forced: Option<ForcedInput>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Program {
    #[default]
    Disabled,
    PlayerOne,
    PlayerTwo,
    LookAround,
    Wander,
    Stationary,
    Sleep,
    Patrol,
    Watch,
    Investigate,
    Combat,
    Approach,
    GuardHome,
    Flank,
    Flee,
    WaitForTarget,
    Pursue,
}

impl Program {
    pub fn from_code(code: u16) -> Result<Self> {
        Ok(match code {
            0 => Self::Disabled,
            1 => Self::PlayerOne,
            2 => Self::PlayerTwo,
            3 => Self::LookAround,
            4 => Self::Wander,
            5 => Self::Stationary,
            6 => Self::Sleep,
            8 => Self::Patrol,
            0x10 => Self::Watch,
            0x20 => Self::Investigate,
            0x100 => Self::Combat,
            0x200 => Self::Approach,
            0x300 => Self::GuardHome,
            0x400 => Self::Flank,
            0x500 => Self::Flee,
            0x1000 => Self::WaitForTarget,
            0x2000 => Self::Pursue,
            _ => return Err(format!("Unknown Tenchu 2 AI program {code:#x}")),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Programs {
    pub idle: Program,
    pub suspicious: Program,
    pub combat: Program,
    pub searching: Program,
}

impl Programs {
    /// Returns whether the source enables AI perception for this assignment.
    pub fn assign(
        &mut self,
        code: u16,
        map: usize,
        difficulty: u8,
        rng: &mut Random,
    ) -> Result<bool> {
        if code < 3 {
            self.idle = Program::from_code(code)?;
            return Ok(false);
        }
        let mut next = Self {
            idle: Program::from_code(code & 0xf)?,
            suspicious: Program::from_code(code & 0xf0)?,
            combat: Program::from_code(code & 0xf00)?,
            searching: Program::from_code(code & 0xf000)?,
        };
        if matches!(map, 2 | 3 | 9 | 10 | 11 | 12 | 16 | 18 | 19) {
            match (difficulty, next.idle) {
                (3, Program::LookAround) if rng.next15() & 1 != 0 => next.idle = Program::Sleep,
                (1, Program::Sleep) => next.idle = Program::LookAround,
                _ => {}
            }
        }
        *self = next;
        Ok(true)
    }
}
