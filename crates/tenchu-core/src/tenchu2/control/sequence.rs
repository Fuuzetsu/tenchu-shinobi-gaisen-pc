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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Projectile {
    Arrow,
    Gunshot,
    Kunai,
    KunaiFan,
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
        let combo_at = resources
            .weapons
            .attacks
            .get(self.attack.index.unwrap_or(usize::MAX))
            .map_or(0, |attack| attack.combo);
        let combo = (i32::from(frame) - i32::from(combo_at)).abs() < 4
            && input & !self.pad.previous & ATTACK != 0;
        if let Some(attack) = self.imported_attack().cloned() {
            if let Some([start, end]) = attack.holster {
                self.attack_grip(request, frame, start, end, 0, &mut cue)?;
            }
            if combo {
                let direction = if input & RIGHT != 0 {
                    1
                } else if input & LEFT != 0 {
                    2
                } else {
                    0
                };
                if let Some(action) = attack.combo[direction] {
                    request.action = action;
                    request.velocity = Velocity::Motion;
                }
            }
            return Ok(ControlFlow::Continue(()));
        }
        let mut next = None;
        match self.motion.action {
            0x900 => {
                let shot = match clip {
                    0x205 => Some((9, 49)),
                    700 => Some((6, 40)),
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
                if clip == 0x418 && frame == 22 {
                    cue(self, request, AttackCue::Fire(Projectile::Gunshot))?;
                    return Ok(ControlFlow::Continue(()));
                }
                if clip == 0x10c {
                    self.attack_grip(request, frame, 9, 27, 0, &mut cue)?;
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
                    self.attack_grip(request, frame, 4, 7, 0, &mut cue)?;
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
                    0x4e2 => self.attack_grip(request, frame, 9, 12, 0, &mut cue)?,
                    0x10d => self.attack_grip(request, frame, 5, 34, 0, &mut cue)?,
                    0x510 if frame == 15 => cue(self, request, AttackCue::Fire(Projectile::Kunai))?,
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
                    0x10e => self.attack_grip(request, frame, 12, 37, 0, &mut cue)?,
                    0xbc => self.attack_grip(request, frame, 5, 31, 0, &mut cue)?,
                    0x4e3 => self.attack_grip(request, frame, 10, 14, 0, &mut cue)?,
                    0x176 if frame == 15 => cue(self, request, AttackCue::Fire(Projectile::Kunai))?,
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
                0x10f => self.attack_grip(request, frame, 10, 50, 0, &mut cue)?,
                0xbd => self.attack_grip(request, frame, 2, 30, 0, &mut cue)?,
                0x4e4 => self.attack_grip(request, frame, 11, 16, 0, &mut cue)?,
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
                } else if clip == 0x4c3 && frame == 18 {
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
                0x177 if frame == 16 => cue(self, request, AttackCue::Fire(Projectile::KunaiFan))?,
                0x110 => self.attack_grip(request, frame, 12, 47, 0, &mut cue)?,
                0x48f => {
                    if frame == 23 {
                        cue(self, request, AttackCue::Fire(Projectile::ForwardGunshot))?;
                    } else {
                        self.attack_grip(request, frame, 10, 40, 1, &mut cue)?;
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
