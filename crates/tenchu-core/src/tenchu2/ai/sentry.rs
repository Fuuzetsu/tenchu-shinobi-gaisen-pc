use crate::{Result, random::Random, tenchu2::input};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::Edition;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum Sentry {
    #[default]
    Begin,
    Turning {
        remaining: u8,
        pause: u8,
    },
    Waiting {
        remaining: u8,
    },
}

pub struct SentryInput {
    pub action: i16,
    pub health: i16,
    pub turn: i16,
    pub held: u16,
}

impl Sentry {
    pub fn update(
        &mut self,
        edition: Edition,
        actor: SentryInput,
        random: &mut Random,
    ) -> Result<(u16, Option<i16>)> {
        if edition == Edition::Japan && actor.action == 0x707 {
            return Ok((0, None));
        }
        match *self {
            Self::Begin => {
                let held = if random.next15() & 1 == 0 {
                    input::LEFT
                } else {
                    input::RIGHT
                };
                let remaining = (1024i32
                    .checked_div(i32::from(actor.turn))
                    .ok_or("Tenchu 2 sentry cannot turn at zero speed")?
                    as u8)
                    .wrapping_add(1);
                let pause = (random.next15() % 21 + 180) as u8;
                *self = if remaining == 0 {
                    Self::Waiting { remaining: pause }
                } else {
                    Self::Turning { remaining, pause }
                };
                Ok((held, None))
            }
            Self::Turning { remaining, pause } => {
                let remaining = remaining.wrapping_sub(1);
                *self = if remaining == 0 {
                    Self::Waiting { remaining: pause }
                } else {
                    Self::Turning { remaining, pause }
                };
                Ok((actor.held, None))
            }
            Self::Waiting { remaining } => {
                let remaining = remaining.wrapping_sub(1);
                *self = if remaining == 0 {
                    Self::Begin
                } else {
                    Self::Waiting { remaining }
                };
                let action = (edition == Edition::Japan
                    && remaining == 92
                    && actor.health > 0
                    && random.next15() & 31 == 0)
                    .then_some(0x707);
                Ok((0, action))
            }
        }
    }
}
