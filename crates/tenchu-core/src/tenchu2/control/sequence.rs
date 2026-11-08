use super::Request;
use crate::{
    Result,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        input::{ATTACK, LEFT, RIGHT},
        physics::LANDED,
        rig::Grip,
    },
};
use std::ops::ControlFlow;
use tenchu_assets::tenchu2::Edition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Projectile {
    Arrow,
    Gunshot,
    Ice,
    IceFan,
    ForwardGunshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackCue {
    Grip {
        slot: usize,
        spare: usize,
        grip: Grip,
    },
    NockArrow,
    Fire(Projectile),
    Land,
    FallDead,
}

impl Actor {
    pub fn attack_sequence(
        &mut self,
        request: &mut Request,
        input: u16,
        resources: &Resources,
        mut cue: impl FnMut(&mut Actor, &mut Request, AttackCue) -> Result<()>,
    ) -> Result<ControlFlow<()>> {
        let clip = self.motion.clip.ok_or("No Tenchu 2 attack motion")?;
        let frame = self.motion.frame;
        let timing = |eu, jp| {
            if resources.registry.edition == Edition::Europe {
                eu
            } else {
                jp
            }
        };
        let combo_at = resources
            .weapons
            .attacks
            .get(self.attack.index.unwrap_or(usize::MAX))
            .map_or(0, |attack| attack.combo);
        let combo = (i32::from(frame) - i32::from(combo_at)).abs() < 4
            && input & !self.pad.previous & ATTACK != 0;
        let mut next = None;
        match self.motion.action {
            0x900 => {
                let shot = match clip {
                    0x205 => Some((timing(7, 9), timing(40, 49))),
                    700 => Some((timing(5, 6), timing(33, 40))),
                    _ => None,
                };
                if let Some((nock, fire)) = shot {
                    if frame == nock {
                        cue(self, request, AttackCue::NockArrow)?;
                        return Ok(ControlFlow::Continue(()));
                    }
                    if frame == fire {
                        cue(self, request, AttackCue::Fire(Projectile::Arrow))?;
                        return Ok(ControlFlow::Continue(()));
                    }
                }
                if clip == 0x418 && frame == timing(18, 22) {
                    cue(self, request, AttackCue::Fire(Projectile::Gunshot))?;
                    return Ok(ControlFlow::Continue(()));
                }
                if clip == 0x10c {
                    self.attack_grip(request, frame, timing(7, 9), timing(22, 27), 0, &mut cue)?;
                }
                if combo {
                    next = Some(if input & RIGHT != 0 {
                        0x902
                    } else if input & LEFT != 0 {
                        0x903
                    } else {
                        0x901
                    });
                }
            }
            0x901..=0x903 => {
                if self.motion.action == 0x901 && clip == 0x4e1 {
                    self.attack_grip(request, frame, timing(3, 4), timing(5, 7), 0, &mut cue)?;
                }
                if combo {
                    next = Some(0x904);
                }
            }
            0x904 | 0x905 => {
                if combo {
                    next = Some(self.motion.action + 1);
                }
            }
            0x907 => {
                match clip {
                    0x4e2 => {
                        self.attack_grip(request, frame, timing(7, 9), timing(10, 12), 0, &mut cue)?
                    }
                    0x10d => {
                        self.attack_grip(request, frame, timing(4, 5), timing(28, 34), 0, &mut cue)?
                    }
                    0x510 if frame == timing(12, 15) => {
                        cue(self, request, AttackCue::Fire(Projectile::Ice))?
                    }
                    _ => {}
                }
                if combo {
                    next = if input & RIGHT != 0 {
                        Some(0x908)
                    } else if input & LEFT != 0 {
                        Some(0x909)
                    } else {
                        None
                    };
                }
            }
            0x90a => {
                match clip {
                    0x10e => self.attack_grip(
                        request,
                        frame,
                        timing(10, 12),
                        timing(30, 37),
                        0,
                        &mut cue,
                    )?,
                    0xbc => {
                        self.attack_grip(request, frame, timing(4, 5), timing(25, 31), 0, &mut cue)?
                    }
                    0x4e3 => self.attack_grip(
                        request,
                        frame,
                        timing(8, 10),
                        timing(11, 14),
                        0,
                        &mut cue,
                    )?,
                    0x176 if frame == timing(12, 15) => {
                        cue(self, request, AttackCue::Fire(Projectile::Ice))?
                    }
                    _ => {}
                }
                if combo {
                    next = if input & LEFT != 0 {
                        Some(0x90b)
                    } else if input & RIGHT != 0 {
                        Some(0x90c)
                    } else {
                        None
                    };
                }
            }
            0x910 => match clip {
                0x10f => {
                    self.attack_grip(request, frame, timing(8, 10), timing(41, 50), 0, &mut cue)?
                }
                0xbd => {
                    self.attack_grip(request, frame, timing(1, 2), timing(25, 30), 0, &mut cue)?
                }
                0x4e4 => {
                    self.attack_grip(request, frame, timing(9, 11), timing(13, 16), 0, &mut cue)?
                }
                _ => {}
            },
            0x911 => {
                if self.flags & LANDED == 0 && self.terrain.height > i32::from(self.height) {
                    if self.motion.loops < 1 {
                        if frame == 1 && clip == 0x4c2 {
                            cue(
                                self,
                                request,
                                AttackCue::Grip {
                                    slot: 0,
                                    spare: 3,
                                    grip: Grip::Hold,
                                },
                            )?;
                        }
                    } else {
                        self.motion.loops = -1;
                    }
                } else if self.health == 0 {
                    self.motion.action = 0x702;
                    cue(self, request, AttackCue::FallDead)?;
                    return Ok(ControlFlow::Break(()));
                } else {
                    next = Some(0x912);
                    self.motion.loops = 0;
                }
            }
            0x912 => {
                if frame == 1 {
                    cue(self, request, AttackCue::Land)?;
                } else if clip == 0x4c3 && frame == timing(15, 18) {
                    cue(
                        self,
                        request,
                        AttackCue::Grip {
                            slot: 0,
                            spare: 3,
                            grip: Grip::Release,
                        },
                    )?;
                }
            }
            0x913 if self.motion.loops > 0 => {
                self.rotation.y = self.rotation.y.wrapping_add(2048);
                if let Some(rotation) = self.motion.pose.rotation.first_mut() {
                    rotation[1] = rotation[1].wrapping_add(2048);
                }
            }
            0x915 => match clip {
                0x177 if frame == timing(13, 16) => {
                    cue(self, request, AttackCue::Fire(Projectile::IceFan))?
                }
                0x110 => {
                    self.attack_grip(request, frame, timing(10, 12), timing(39, 47), 0, &mut cue)?
                }
                0x48f => {
                    if frame == timing(19, 23) {
                        cue(self, request, AttackCue::Fire(Projectile::ForwardGunshot))?;
                    } else {
                        self.attack_grip(
                            request,
                            frame,
                            timing(8, 10),
                            timing(33, 40),
                            1,
                            &mut cue,
                        )?;
                    }
                }
                _ => {}
            },
            0x916 => {
                self.velocity.x = self.velocity.x.wrapping_sub(self.velocity.x >> 4);
                self.velocity.z = self.velocity.z.wrapping_sub(self.velocity.z >> 4);
            }
            _ => {}
        }
        if let Some(action) = next {
            request.action = action;
            request.velocity = Velocity::Motion;
        }
        Ok(ControlFlow::Continue(()))
    }

    fn attack_grip(
        &mut self,
        request: &mut Request,
        frame: i16,
        hold: i16,
        release: i16,
        slot: usize,
        cue: &mut impl FnMut(&mut Actor, &mut Request, AttackCue) -> Result<()>,
    ) -> Result<()> {
        if frame == hold || frame == release {
            cue(
                self,
                request,
                AttackCue::Grip {
                    slot,
                    spare: slot + 3,
                    grip: if frame == hold {
                        Grip::Hold
                    } else {
                        Grip::Release
                    },
                },
            )?;
        }
        Ok(())
    }
}
