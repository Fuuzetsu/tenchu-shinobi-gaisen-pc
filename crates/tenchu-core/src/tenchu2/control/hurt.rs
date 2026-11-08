use super::Request;
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        input::{LEFT, RIGHT},
        physics::LANDED,
    },
};

pub struct HurtInput {
    pub held: u16,
    pub player: bool,
    pub difficulty: i16,
    pub map: i16,
    pub water_surface: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HurtCue {
    Recover,
    HideHeldItem,
    Voice(u8),
    Bubbles,
    Blood {
        joint: usize,
        // A random angle is drawn after the joint query, using the shared RNG.
        angle: Option<i16>,
    },
    WaterRipple {
        position: Position,
        size: i16,
    },
    Splash(u8),
    LandingRumble,
    Alert,
    Dust,
    RemoveWeapons,
    AlignToFloor,
    Dead,
}

impl Actor {
    pub fn update_hurt(
        &mut self,
        request: &mut Request,
        input: &HurtInput,
        resources: &Resources,
        random: &mut Random,
        mut cue: impl FnMut(&mut Actor, &mut Request, &mut Random, HurtCue) -> Result<()>,
    ) -> Result<()> {
        if self.motion.frame == 1 && input.player {
            cue(self, request, random, HurtCue::HideHeldItem)?;
        }
        match self.motion.action {
            0xa00..=0xa03 => {
                for speed in [&mut self.velocity.x, &mut self.velocity.z] {
                    if *speed != 0 {
                        *speed = speed.wrapping_add(if *speed > 0 { -4 } else { 4 });
                    }
                }
                if input.held & (LEFT | RIGHT) != 0 {
                    let turn = self.turn / 2;
                    self.rotation.y = self.rotation.y.wrapping_add(if input.held & RIGHT != 0 {
                        turn
                    } else {
                        turn.wrapping_neg()
                    });
                    self.update_velocity(Velocity::Motion, resources)?;
                }
            }
            0xa04 => {
                if self.motion.frame % 5 == 0 && random.next15() & 1 != 0 {
                    cue(self, request, random, HurtCue::Voice(27))?;
                    cue(self, request, random, HurtCue::Bubbles)?;
                }
                if self.motion.loops > 0 {
                    request.action = 0x530;
                    request.velocity = Velocity::Motion;
                }
                return Ok(());
            }
            0xa10 if self.motion.frame == 1 => {
                cue(self, request, random, HurtCue::Voice(13))?;
                self.flags &= !0x200;
            }
            0xa11 if self.motion.frame == 1 => {
                cue(self, request, random, HurtCue::Voice(14))?;
                cue(
                    self,
                    request,
                    random,
                    HurtCue::Blood {
                        joint: 2,
                        angle: Some(self.rotation.y),
                    },
                )?;
            }
            0xa13 => {
                if self.motion.frame == 1 {
                    cue(self, request, random, HurtCue::Voice(13))?;
                } else if self.motion.loops > 0 {
                    self.update_velocity(Velocity::Stop, resources)?;
                    self.motion.loops = -1;
                } else if self.motion.loops < 0 {
                    self.motion.loops = self.motion.loops.wrapping_sub(1);
                    if self.motion.loops <= -201 {
                        request.action = 0xa14;
                        request.velocity = Velocity::Motion;
                        self.flags &= !0x800;
                    }
                    return Ok(());
                }
            }
            0xa15 if self.motion.frame == 1 => {
                let voice = if random.next15() & 1 != 0 { 13 } else { 14 };
                cue(self, request, random, HurtCue::Voice(voice))?;
            }
            0xa16 => {
                if self.motion.frame % 3 == 0 && random.next15() & 1 != 0 {
                    let position = Position::new(
                        self.position.x.wrapping_add(random.next15() % 1000 - 500),
                        input.water_surface,
                        self.position.z.wrapping_add(random.next15() % 1000 - 500),
                    );
                    cue(
                        self,
                        request,
                        random,
                        HurtCue::WaterRipple {
                            position,
                            size: 512,
                        },
                    )?;
                    if random.next15() & 1 != 0 {
                        cue(self, request, random, HurtCue::Splash(4))?;
                    }
                }
                if self.motion.loops > 0 {
                    request.action = 0x510;
                    request.velocity = Velocity::Motion;
                }
                return Ok(());
            }
            0xa21 | 0xa22 => {
                if self.motion.frame == 1 {
                    let speed = 100;
                    self.set_horizontal_velocity(
                        if request.action == 0xa21 {
                            -speed
                        } else {
                            speed
                        },
                        0,
                        &resources.math,
                    );
                }
                if self.motion.loops > 0 {
                    if self.terrain.height < 20001 {
                        self.motion.loops = -1;
                    } else {
                        request.action = 0x702;
                        request.velocity = Velocity::Keep;
                    }
                } else if self.motion.loops == 0 {
                    let clip = resources
                        .clips
                        .get(&self.motion.clip.ok_or("No Tenchu 2 hurt motion")?)
                        .ok_or("Missing Tenchu 2 hurt motion")?;
                    self.velocity.y = self
                        .motion
                        .frame
                        .wrapping_sub(clip.duration >> 1)
                        .wrapping_mul(10);
                }
                if self.flags & LANDED != 0 || self.terrain.height < i32::from(self.height) {
                    request.action = if request.action == 0xa21 {
                        0xa23
                    } else {
                        0xa24
                    };
                    request.velocity = Velocity::Motion;
                    self.change_motion(request.action, request.velocity, resources)?;
                }
                if input.map != 12 && self.motion.frame % 5 == 0 && random.next15() % 5 == 0 {
                    cue(
                        self,
                        request,
                        random,
                        HurtCue::Blood {
                            joint: 0,
                            angle: None,
                        },
                    )?;
                }
                return Ok(());
            }
            0xa23 | 0xa24 => {
                if self.motion.frame == 1 {
                    if self.health != 0 {
                        cue(self, request, random, HurtCue::LandingRumble)?;
                    }
                    if input.difficulty == 1 {
                        cue(self, request, random, HurtCue::Alert)?;
                    }
                    match self.terrain.material & 0xf00 {
                        0x200 => {
                            cue(
                                self,
                                request,
                                random,
                                HurtCue::WaterRipple {
                                    position: self.position,
                                    size: 2048,
                                },
                            )?;
                            cue(self, request, random, HurtCue::Voice(28))?;
                        }
                        0 => {
                            cue(self, request, random, HurtCue::Dust)?;
                            cue(self, request, random, HurtCue::Voice(24))?;
                        }
                        _ => {}
                    }
                } else if self.motion.loops > 0 {
                    request.action = 0xa25;
                    request.velocity = Velocity::Motion;
                }
                return Ok(());
            }
            0xa25 => {
                if self.motion.frame < 0 {
                    return Ok(());
                }
                let delay = if self.kind as u8 & 0xf0 >= 0xe0 {
                    0
                } else if self.flags & 0x800 != 0 {
                    -60
                } else {
                    (-(i32::from(self.max_health) - i32::from(self.health)) / 2) as i16
                };
                if self.health == 0 {
                    if input.player {
                        cue(self, request, random, HurtCue::HideHeldItem)?;
                    }
                    cue(self, request, random, HurtCue::RemoveWeapons)?;
                    cue(self, request, random, HurtCue::AlignToFloor)?;
                    self.state = State::Dead;
                    self.motion.loops = 1;
                    cue(self, request, random, HurtCue::Dead)?;
                } else {
                    self.motion.loops = self.motion.loops.wrapping_sub(1);
                    if self.motion.loops < delay {
                        request.action = 0xa26;
                        request.velocity = Velocity::Motion;
                        self.flags &= !0x800;
                    }
                }
                return Ok(());
            }
            _ => {}
        }
        if self.motion.loops > 0 {
            cue(self, request, random, HurtCue::Recover)?;
        }
        Ok(())
    }
}
