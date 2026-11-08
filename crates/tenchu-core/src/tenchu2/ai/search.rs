use super::{Decision, combat::Context, sight::Visibility};
use crate::{
    Result,
    math::Position,
    tenchu2::{actor::Actor, input, terrain},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{Edition, collision::Collision};

#[derive(Default)]
pub struct SearchResult {
    pub decision: Decision,
    pub finished: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Waiting {
    pub turning: u8,
    pub aligning: u8,
}

impl Waiting {
    pub fn update(
        &mut self,
        c: &mut Context<'_>,
        alert: &mut i16,
        align: bool,
        mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<SearchResult> {
        let flags = c.observed_flags as u16 & 0xff7c;
        if c.actor.kind & 0xe0 == 0xe0 {
            *alert = 10;
        }
        if *alert == 0 {
            let mut decision = Decision::default();
            if align {
                decision = c.steer()?;
                let elapsed = self.aligning;
                self.aligning = self.aligning.wrapping_add(1);
                if elapsed < 250 {
                    decision.held &= input::LEFT | input::RIGHT;
                    if decision.held != 0 {
                        return Ok(SearchResult {
                            decision,
                            finished: false,
                        });
                    }
                } else {
                    self.aligning = 250;
                }
            }
            decision.held = 0;
            let awareness = match *c.visibility {
                Visibility::Recognized => {
                    c.observed_flags = (flags | 2) as i16 as u32;
                    return Ok(SearchResult {
                        decision,
                        finished: false,
                    });
                }
                Visibility::Unchanged => {
                    return Ok(SearchResult {
                        decision,
                        finished: false,
                    });
                }
                Visibility::Hidden | Visibility::Unnoticed => 0,
                Visibility::Suspicious => 1,
            };
            c.observed_flags = (flags | awareness) as i16 as u32;
            if c.actor.weapon_drawn() {
                apply(c.actor, 0x701)?;
            }
            *self = Self::default();
            return Ok(SearchResult {
                decision,
                finished: true,
            });
        }
        self.aligning = 0;
        if *c.visibility == Visibility::Recognized {
            c.observed_flags = (flags | 2) as i16 as u32;
        }
        Ok(SearchResult {
            decision: Decision {
                held: look_around(c, &mut self.turning, 1024)?,
                ..Default::default()
            },
            finished: false,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PursuitPhase {
    Following { detour: bool, elapsed: i16 },
    Looking { elapsed: u8, turning: u8 },
    Waiting(Waiting),
}

impl Default for PursuitPhase {
    fn default() -> Self {
        Self::Following {
            detour: false,
            elapsed: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pursuit {
    pub phase: PursuitPhase,
    pub remembered: Position,
}

impl Pursuit {
    pub fn update(
        &mut self,
        c: &mut Context<'_>,
        alert: &mut i16,
        terrain: &Collision,
        cursor: &mut terrain::Cursor,
        apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<SearchResult> {
        if *c.visibility == Visibility::Recognized {
            c.observed_flags = c.observed_flags & !3 | 2;
            return Ok(SearchResult::default());
        }
        if *alert == 1 {
            self.start_looking();
        }
        let delta = c.sight.destination.wrapping_sub(c.actor.position);
        let distance = c.resources.math.length(Position::new(delta.x, 0, delta.z));
        if !matches!(self.phase, PursuitPhase::Waiting(_))
            && (delta.y < -5000 || (distance < 3500 && delta.y.wrapping_abs() > 2000))
        {
            self.start_looking();
        }
        let decision = match &mut self.phase {
            PursuitPhase::Following { detour, elapsed } => {
                let mut decision = c.steer_to(delta.x, delta.z)?;
                if c.observed_flags & 0x48000 != 0
                    && decision.held & (input::LEFT | input::RIGHT) != 0
                {
                    decision.held |= input::UP;
                }
                let reached = delta.x.wrapping_abs() < 500 && delta.z.wrapping_abs() < 500;
                if reached {
                    if !*detour && self.remembered.y.wrapping_sub(c.actor.position.y) >= -3000 {
                        let dx = self.remembered.x.wrapping_sub(c.actor.position.x) as i16;
                        let dz = self.remembered.z.wrapping_sub(c.actor.position.z) as i16;
                        for step in [[dx, 0, 0], [0, 0, dz]] {
                            if step == [0; 3] {
                                continue;
                            }
                            if terrain::traverse(
                                terrain,
                                cursor,
                                c.sight.destination,
                                step.into(),
                                0,
                                c.resources.registry.edition,
                                &c.resources.math,
                            )?
                            .is_none()
                            {
                                c.sight.destination =
                                    c.actor.position.wrapping_add(step.map(i32::from).into());
                                *detour = true;
                                return Ok(SearchResult {
                                    decision,
                                    finished: false,
                                });
                            }
                        }
                        c.sight.destination = c.actor.position;
                    }
                    *elapsed = 0;
                }
                let old_elapsed = *elapsed;
                *elapsed = elapsed.wrapping_add(1);
                let timeout = match c.resources.registry.edition {
                    Edition::Europe => 300,
                    Edition::Japan => 150,
                };
                if reached || old_elapsed > timeout {
                    self.start_looking();
                }
                decision
            }
            PursuitPhase::Looking { elapsed, turning } => {
                let old_elapsed = *elapsed;
                *elapsed = elapsed.wrapping_add(1);
                let finish = old_elapsed > 90;
                if finish {
                    *elapsed = 0;
                    *turning = 0;
                }
                let held = look_around(c, turning, 512)?;
                if finish {
                    self.phase = PursuitPhase::Waiting(Waiting {
                        turning: 0,
                        aligning: *turning,
                    });
                }
                Decision {
                    held,
                    ..Default::default()
                }
            }
            PursuitPhase::Waiting(waiting) => {
                let result = waiting.update(c, alert, false, apply)?;
                if result.finished {
                    *self = Self::default();
                }
                return Ok(result);
            }
        };
        Ok(SearchResult {
            decision,
            finished: false,
        })
    }

    fn start_looking(&mut self) {
        self.phase = match &self.phase {
            PursuitPhase::Following { .. } => PursuitPhase::Looking {
                elapsed: 0,
                turning: 0,
            },
            PursuitPhase::Looking { .. } => return,
            PursuitPhase::Waiting(waiting) => PursuitPhase::Looking {
                elapsed: waiting.turning,
                turning: waiting.aligning,
            },
        };
    }
}

fn look_around(c: &mut Context<'_>, remaining: &mut u8, far_turn: i32) -> Result<u16> {
    if *remaining != 0 {
        *remaining = remaining.wrapping_sub(1);
        return Ok(c.actor.pad.held);
    }
    if c.actor.pad.held != 0 {
        return Ok(0);
    }
    let near = c.sight.distance <= 4000;
    if near && c.actor.flags & 0x100000 == 0 {
        c.face_target();
        return Ok(0);
    }
    if c.actor.motion.loops < 1 {
        return Ok(0);
    }
    let (mut held, rotation) = if near {
        let angle = i32::from(c.sight.angle).abs();
        let rotation = if angle <= 200 || c.random.next15() & 1 == 0 {
            1024
        } else {
            angle
        };
        (
            if c.sight.angle > 0 {
                input::RIGHT
            } else {
                input::LEFT
            },
            rotation,
        )
    } else {
        (
            if c.random.next15() & 1 != 0 {
                input::RIGHT
            } else {
                input::LEFT
            },
            far_turn,
        )
    };
    *remaining = (rotation
        .checked_div(i32::from(c.actor.turn))
        .ok_or("Tenchu 2 searching guard cannot turn at zero speed")? as u8)
        .wrapping_add(1);
    if near && c.sight.distance < 2000 && c.random.next15() & 1 != 0 {
        held = input::UP;
        *remaining = 31;
    }
    Ok(held)
}
