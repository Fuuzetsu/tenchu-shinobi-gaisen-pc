use super::{Decision, ForcedInput, Probe, direction};
use crate::{
    Result,
    math::{Math, Position},
    random::Random,
    tenchu2::input,
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{Edition, layout};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Turn {
    None,
    Left,
    Right,
    Back,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Waypoint {
    x: i32,
    z: i32,
    turn: Turn,
    pause: i16,
}

pub struct PatrolInput {
    pub position: Position,
    pub yaw: i16,
    pub turn: i16,
    pub action: i16,
    pub health: i16,
    pub held: u16,
    pub blocked: bool,
    pub touching_player: bool,
    pub in_water: bool,
    pub ahead: Option<Probe>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patrol {
    route: Vec<Waypoint>,
    pub next: usize,
    pub wait: i16,
    pub turning: u8,
}

impl Patrol {
    pub fn new(route: &[layout::Waypoint], home: Position) -> Self {
        let mut points: Vec<_> = route
            .iter()
            .map(|point| Waypoint {
                x: point.x,
                z: point.z,
                turn: match point.action as i8 {
                    1 => Turn::Left,
                    2 => Turn::Right,
                    4 => Turn::Back,
                    _ => Turn::None,
                },
                pause: point.wait as i16,
            })
            .collect();
        if !points.is_empty() {
            points.push(Waypoint {
                x: home.x,
                z: home.z,
                turn: Turn::None,
                pause: -1,
            });
        }
        Self {
            route: points,
            next: 0,
            wait: 0,
            turning: 0,
        }
    }

    pub fn update(
        &mut self,
        edition: Edition,
        actor: &PatrolInput,
        math: &Math,
        random: &mut Random,
    ) -> Result<Decision> {
        let mut decision = Decision::default();
        if self.route.is_empty() {
            return Ok(decision);
        }
        if edition == Edition::Japan {
            if actor.action == 0x707 {
                return Ok(decision);
            }
            if random.next15() % 18000 == 0 && actor.health > 0 {
                decision.action = Some(0x707);
                return Ok(decision);
            }
        }
        let point = self
            .route
            .get(self.next)
            .ok_or("Invalid Tenchu 2 patrol waypoint")?;
        let x = point.x.wrapping_sub(actor.position.x);
        let z = point.z.wrapping_sub(actor.position.z);
        let distance = if x.wrapping_abs() < 32767 && z.wrapping_abs() < 32767 {
            math.square_root(x.wrapping_mul(x).wrapping_add(z.wrapping_mul(z)))
        } else {
            32767
        };
        if self.turning != 0 {
            self.turning -= 1;
            decision.held = actor.held;
            return Ok(decision);
        }
        let wait = self.wait;
        self.wait = self.wait.wrapping_sub(1);
        if wait > 0 {
            return Ok(decision);
        }
        if distance > 500 {
            let angle = direction(math, x, z, actor.yaw)?;
            decision.held = input::UP;
            if angle > actor.turn {
                decision.held |= input::RIGHT;
            } else if i32::from(angle) < -i32::from(actor.turn) {
                decision.held |= input::LEFT;
            }
            if i32::from(angle).abs() > 300 {
                decision.held &= input::LEFT | input::RIGHT;
            }
            if actor.blocked
                && decision.held & (input::LEFT | input::RIGHT) != 0
                && !actor.touching_player
            {
                if actor.turn == 0 {
                    return Err("Tenchu 2 patrol cannot turn at zero speed".into());
                }
                let held = if random.next15() & 1 == 0 {
                    input::LEFT
                } else {
                    input::RIGHT
                };
                decision.forced = Some(ForcedInput::turn(held, 600, actor.turn, 0)?);
                decision.held = 0;
            }
            if decision.held & input::UP != 0
                && distance >= 3001
                && !actor.in_water
                && actor
                    .ahead
                    .as_ref()
                    .is_some_and(|probe| probe.material & 0xf00 != 0 || probe.height >= 4001)
            {
                if decision.held & (input::LEFT | input::RIGHT) == 0 {
                    decision.held = if random.next15() & 1 == 0 {
                        input::LEFT
                    } else {
                        input::RIGHT
                    };
                }
                decision.forced = Some(ForcedInput::turn(
                    decision.held & (input::LEFT | input::RIGHT),
                    1100,
                    actor.turn,
                    0,
                )?);
            }
            return Ok(decision);
        }
        self.next += 1;
        if point.pause == 0 {
            return Ok(decision);
        }
        let turn = match point.turn {
            Turn::None => None,
            Turn::Left => Some((input::LEFT, 1024i32)),
            Turn::Right => Some((input::RIGHT, 1024)),
            Turn::Back => Some((input::RIGHT, 2048)),
        };
        self.turning = if let Some((held, angle)) = turn {
            decision.held = held;
            (angle
                .checked_div(i32::from(actor.turn))
                .ok_or("Tenchu 2 patrol cannot turn at zero speed")? as u8)
                .wrapping_add(1)
        } else {
            0
        };
        if point.pause == -1 {
            self.next = 0;
            self.wait = 0;
        } else {
            self.wait = point.pause;
        }
        Ok(decision)
    }
}
