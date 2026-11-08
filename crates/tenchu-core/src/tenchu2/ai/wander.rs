use super::{Decision, ForcedInput, SteeringInput, steer};
use crate::{
    Result,
    math::{Math, Position},
    random::Random,
    tenchu2::input,
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::Edition;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wander {
    #[default]
    Begin,
    Walking,
    Waiting {
        remaining: u8,
        turning: u8,
    },
}

pub struct WanderInput {
    pub position: Position,
    pub home: Position,
    pub action: i16,
    pub health: i16,
    pub loops: i16,
    pub in_water: bool,
    pub steering: SteeringInput,
}

impl Wander {
    pub fn update(
        &mut self,
        edition: Edition,
        actor: &WanderInput,
        destination: &mut Position,
        math: &Math,
        random: &mut Random,
    ) -> Result<Decision> {
        let mut decision = Decision::default();
        if actor.steering.forced || (edition == Edition::Japan && actor.action == 0x707) {
            return Ok(decision);
        }
        match *self {
            Self::Begin => {
                destination.x = actor.home.x.wrapping_add(random.next15() % 8000 - 4000);
                destination.z = actor.home.z.wrapping_add(random.next15() % 8000 - 4000);
                destination.y = actor.home.y;
                *self = Self::Walking;
            }
            Self::Walking => {
                let x = destination.x.wrapping_sub(actor.position.x);
                let z = destination.z.wrapping_sub(actor.position.z);
                if x.wrapping_abs() < 1000 && z.wrapping_abs() < 1000 {
                    *self = Self::Waiting {
                        remaining: 120,
                        turning: 0,
                    };
                    return Ok(decision);
                }
                if actor.steering.blocked {
                    decision.forced = Some(random_turn(actor.steering.turn, random)?);
                    *self = Self::Waiting {
                        remaining: 120,
                        turning: 0,
                    };
                    return Ok(decision);
                }
                decision = steer(x, z, &actor.steering, math)?;
                let far = (decision.held & input::UP != 0 && x.wrapping_abs() > 4000)
                    || (z.wrapping_abs() > 4000 && random.next15() & 1 != 0);
                if far {
                    if !actor.in_water
                        && actor
                            .steering
                            .ahead
                            .is_some_and(|probe| probe.material & 0xf00 != 0 || probe.height > 4000)
                    {
                        decision.forced = Some(random_turn(actor.steering.turn, random)?);
                        decision.held = 0;
                    }
                } else if actor.loops > 3 {
                    *self = Self::Waiting {
                        remaining: 15,
                        turning: 0,
                    };
                }
            }
            Self::Waiting { remaining, turning } => {
                let remaining = remaining.wrapping_sub(1);
                decision.held = if turning == 0 { 0 } else { input::RIGHT };
                *self = if remaining == 0 {
                    Self::Begin
                } else {
                    Self::Waiting {
                        remaining,
                        turning: turning.saturating_sub(1),
                    }
                };
                if edition == Edition::Japan
                    && remaining == 62
                    && actor.health > 0
                    && random.next15() & 31 == 0
                {
                    decision.action = Some(0x707);
                }
            }
        }
        Ok(decision)
    }
}

fn random_turn(speed: i16, random: &mut Random) -> Result<ForcedInput> {
    if speed == 0 {
        return Err("Tenchu 2 AI cannot turn at zero speed".into());
    }
    let held = if random.next15() & 1 == 0 {
        input::LEFT
    } else {
        input::RIGHT
    };
    ForcedInput::turn(held, 700, speed, 0)
}
