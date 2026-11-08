use super::{local_alert_only, sight::Visibility};
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::actor::{Actor, State},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::combat::Rules;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ki {
    Clear,
    Spotted,
    Searching,
    Suspicious,
    Nearby(u8),
}

impl Ki {
    pub fn from_distance(distance: i32) -> Self {
        match distance {
            0 => Self::Spotted,
            ..-24000 => Self::Searching,
            -24000..0 => Self::Suspicious,
            1..24000 => {
                let value = (24000 - distance) * 100 / 24000;
                if value == 0 {
                    Self::Clear
                } else {
                    Self::Nearby(value as u8)
                }
            }
            _ => Self::Clear,
        }
    }
}

#[derive(Clone, Copy)]
pub enum AlertCause {
    Suspicion,
    Corpse,
    Combat,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Alert {
    pub remaining: i16,
    pub position: Option<Position>,
}

impl Alert {
    pub fn raise(&mut self, cause: AlertCause, difficulty: u8, rules: &Rules) -> Result<()> {
        let duration = *rules
            .alert_duration
            .get(usize::from(difficulty.wrapping_sub(1)))
            .ok_or("Invalid Tenchu 2 alert difficulty")?;
        self.remaining = match cause {
            AlertCause::Suspicion => duration >> 1,
            AlertCause::Corpse => ((i32::from(duration) * 3) >> 2) as i16,
            AlertCause::Combat => duration,
        };
        Ok(())
    }

    pub fn reset(&mut self, difficulty: u8, rules: &Rules) -> Result<()> {
        self.remaining = *rules
            .alert_reset_duration
            .get(usize::from(difficulty.wrapping_sub(1)))
            .ok_or("Invalid Tenchu 2 alert difficulty")?;
        Ok(())
    }

    pub fn advance(&mut self) {
        if self.remaining != 0 {
            self.remaining = self.remaining.wrapping_sub(1);
            if self.remaining < 0 {
                self.remaining = 0;
                self.position = None;
            }
        }
    }

    pub fn visibility(
        &self,
        actor: &Actor,
        flags: u32,
        script: u16,
        observed: Visibility,
    ) -> Visibility {
        if local_alert_only(actor.kind) {
            return observed;
        }
        if script >= 3 || (script >= 2 && actor.kind & 0xe0 != 0xe0) {
            return Visibility::Hidden;
        }
        match self.remaining {
            0 => observed,
            1 => Visibility::Unnoticed,
            _ if flags & 3 == 0 => Visibility::Suspicious,
            _ if observed == Visibility::Suspicious => Visibility::Recognized,
            _ => observed,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum Victory {
    #[default]
    Begin,
    Waiting(u8),
    Taunt,
    Sheathe,
    Done,
}

impl Victory {
    pub fn update(
        &mut self,
        actor: &mut Actor,
        random: &mut Random,
        mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<()> {
        if actor.health <= 0 {
            return Ok(());
        }
        match self {
            Self::Begin => *self = Self::Waiting((random.next15() % 50 + 10) as u8),
            Self::Waiting(remaining) => {
                *remaining = remaining.wrapping_sub(1);
                if *remaining == 0 {
                    *self = Self::Taunt;
                }
            }
            Self::Taunt => {
                if actor.flags & 3 > 1 {
                    apply(actor, 0x70d)?;
                }
                *self = Self::Sheathe;
            }
            Self::Sheathe if actor.state != State::Special && actor.weapon_drawn() => {
                apply(actor, 0x701)?;
                *self = Self::Done;
            }
            Self::Sheathe | Self::Done => {}
        }
        Ok(())
    }
}
