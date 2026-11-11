use super::{Request, StatusResult};
use crate::{
    Result,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        input::{ATTACK, CROUCH, DOWN, GUARD, ITEM, JUMP, LEFT, RIGHT, UP},
        physics::BLOCKED,
    },
};

pub struct StandingInput {
    pub held: u16,
    pub unarmed: bool,
    pub player: bool,
    pub controls_enabled: bool,
    pub water: bool,
    pub map: i16,
    pub sequence: i16,
    pub story_triggered: bool,
    pub carried_action: Option<i16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandingCue {
    Status(StatusResult),
    Voice(u8),
    Splash(u8),
    Dust,
    Footstep { check_surface: bool },
    Jump,
    Attack,
    Item,
    Ledge,
    Recover,
    HiddenMission,
    HiddenCarry,
}

impl Actor {
    /// The ledge callback returns whether it took control; other cues ignore the return value.
    pub fn update_standing(
        &mut self,
        request: &mut Request,
        input: &StandingInput,
        resources: &Resources,
        random: &mut Random,
        mut cue: impl FnMut(&mut Actor, &mut Request, &mut Random, StandingCue) -> Result<bool>,
    ) -> Result<()> {
        let status = self.update_status(request, random, input.player, input.controls_enabled);
        if status != StatusResult::default() {
            cue(self, request, random, StandingCue::Status(status))?;
        }
        if status.effect.is_some() {
            return Ok(());
        }
        if self.previous.y.wrapping_sub(self.position.y) > 1000 {
            request.action = 0x703;
            request.velocity = Velocity::Keep;
            return Ok(());
        }
        let ready = self.state == State::Ready;
        let base = if ready { 0x100 } else { 0 };
        let held = input.held;
        let pressed = held & !self.pad.previous;
        let forward = held & UP != 0;
        let turning = held & (LEFT | RIGHT) != 0;
        let right = held & RIGHT != 0;
        match self.motion.action.wrapping_sub(base) {
            0 => {
                let command = self.pad.consume(&resources.commands.standing).or_else(|| {
                    if ready {
                        self.pad
                            .consume(&resources.commands.jumping)
                            .filter(|&action| action == 0x910)
                    } else {
                        None
                    }
                });
                if let Some(action) = command {
                    request.action = action;
                    request.velocity = Velocity::Motion;
                    return Ok(());
                }
                if turning {
                    request.action = base + if right { 1 } else { 2 };
                    request.velocity = Velocity::Stop;
                }
                if !ready
                    && !input.story_triggered
                    && input.map == 8
                    && self.position.x / 1000 == 34
                    && self.position.y / 1000 == -15
                    && self.position.z / 1000 == 23
                    && self.motion.loops == 5
                {
                    cue(self, request, random, StandingCue::HiddenMission)?;
                }
            }
            action @ (1 | 2) => {
                if held & if action == 1 { RIGHT } else { LEFT } != 0 {
                    self.turn_standing(self.turn, action == 1);
                } else {
                    request.action = base;
                    request.velocity = Velocity::Motion;
                }
                if self.motion.frame == 1 {
                    cue(self, request, random, StandingCue::Voice(16))?;
                }
            }
            3 => {
                if !forward {
                    request.action = base;
                    request.velocity = Velocity::Motion;
                } else if turning {
                    self.turn_standing(
                        if ready || input.player {
                            self.turn / 2
                        } else {
                            self.turn
                        },
                        right,
                    );
                    self.update_velocity(Velocity::Motion, resources)?;
                }
                if self.flags & BLOCKED != 0 {
                    let (y, height) = (self.position.y, self.terrain.height);
                    self.position.y = y.wrapping_sub(1);
                    self.terrain.height = i32::from(self.height);
                    if cue(self, request, random, StandingCue::Ledge)? {
                        return Ok(());
                    }
                    self.position.y = y;
                    self.terrain.height = height;
                }
                if ready && self.motion.frame == 0 && !input.player && self.flags & 3 == 0 {
                    request.action = 0x701;
                    request.velocity = Velocity::Motion;
                }
            }
            4 => {
                if held & DOWN == 0 {
                    request.action = base;
                    request.velocity = Velocity::Motion;
                } else if turning {
                    self.turn_standing(self.turn.wrapping_mul(2), right);
                    self.update_velocity(Velocity::Motion, resources)?;
                }
                if ready && self.motion.frame == 1 {
                    cue(self, request, random, StandingCue::Voice(19))?;
                }
            }
            0x10 if ready => {
                if held & GUARD == 0 {
                    cue(self, request, random, StandingCue::Recover)?;
                } else if let Some(action) = [
                    (CROUCH, 0x200),
                    (UP, 0x111),
                    (DOWN, 0x112),
                    (RIGHT, 0x113),
                    (LEFT, 0x114),
                ]
                .into_iter()
                .find_map(|(button, action)| (held & button != 0).then_some(action))
                {
                    request.action = action;
                    request.velocity = Velocity::Motion;
                }
                return Ok(());
            }
            action @ 0x11..=0x14 if ready => {
                if action <= 0x12 && pressed & JUMP != 0 {
                    cue(self, request, random, StandingCue::Jump)?;
                }
                if self.motion.loops > 0 {
                    if held & GUARD == 0 {
                        cue(self, request, random, StandingCue::Recover)?;
                    } else {
                        request.action = 0x110;
                        request.velocity = Velocity::Motion;
                    }
                }
                if self.motion.frame == 1 {
                    if input.water {
                        cue(self, request, random, StandingCue::Splash(2))?;
                        cue(self, request, random, StandingCue::Splash(4))?;
                    } else {
                        cue(self, request, random, StandingCue::Dust)?;
                        cue(self, request, random, StandingCue::Voice(19))?;
                    }
                }
                if pressed & ATTACK != 0 && (self.weapon_drawn() || input.unarmed) {
                    cue(self, request, random, StandingCue::Attack)?;
                }
                return Ok(());
            }
            0x20 if ready => {
                if self.motion.frame < 0 {
                    return Ok(());
                }
                if !forward && !turning {
                    if self.motion.loops >= 0 {
                        self.update_velocity(Velocity::Stop, resources)?;
                        self.motion.loops = -1;
                    }
                } else {
                    if self.motion.loops < 0 {
                        self.motion.loops = 0;
                    }
                    if turning {
                        self.turn_standing(if forward { self.turn / 2 } else { self.turn }, right);
                    }
                    self.update_velocity(Velocity::Motion, resources)?;
                }
                if held & GUARD != 0 && input.carried_action == Some(0xb09) {
                    return Ok(());
                }
                if !input.story_triggered
                    && input.map == 12
                    && self.position.x / 1000 == -16
                    && self.position.z / 1000 == 78
                {
                    cue(self, request, random, StandingCue::HiddenCarry)?;
                }
                cue(self, request, random, StandingCue::Recover)?;
                return Ok(());
            }
            _ => {}
        }
        let duration = resources
            .clips
            .get(&self.motion.clip.ok_or("No Tenchu 2 movement motion")?)
            .ok_or("Missing Tenchu 2 movement motion")?
            .duration;
        if self.terrain.height <= i32::from(self.height) + 300
            && matches!(self.motion.action.wrapping_sub(base), 3 | 4)
            && (self.motion.frame == 0 || self.motion.frame == duration / 2)
        {
            let mut material = self.terrain.material & 0xf00;
            if matches!(input.map, 6 | 8) {
                if random.next15() & 3 == 0 {
                    material = 0x200;
                } else {
                    cue(self, request, random, StandingCue::Voice(25))?;
                }
            }
            if material == 0x200 && self.terrain.height <= i32::from(self.height) {
                cue(self, request, random, StandingCue::Splash(4))?;
            } else if matches!(material, 0x200 | 0x400) {
                if input.player {
                    cue(
                        self,
                        request,
                        random,
                        StandingCue::Footstep {
                            check_surface: true,
                        },
                    )?;
                }
            } else if input.player {
                cue(
                    self,
                    request,
                    random,
                    StandingCue::Footstep {
                        check_surface: false,
                    },
                )?;
            } else {
                cue(
                    self,
                    request,
                    random,
                    StandingCue::Voice(if ready { 18 } else { 17 }),
                )?;
            }
        }
        if pressed & ATTACK != 0 {
            cue(self, request, random, StandingCue::Attack)?;
            if ready {
                return Ok(());
            }
        } else {
            let action = if held & GUARD != 0 {
                Some(
                    if ready && self.weapon_drawn() && self.pad.history[..2] == [GUARD, GUARD] {
                        0x701
                    } else {
                        0x110
                    },
                )
            } else if ready && held & ATTACK != 0 && self.motion.loops != 0 {
                Some(0x701)
            } else if held & CROUCH != 0 {
                Some(0x200)
            } else if pressed & JUMP != 0 {
                cue(self, request, random, StandingCue::Jump)?;
                None
            } else if held & ITEM != 0 {
                cue(self, request, random, StandingCue::Item)?;
                None
            } else if forward {
                Some(base + 3)
            } else if held & DOWN != 0 {
                Some(base + 4)
            } else {
                None
            };
            if let Some(action) = action {
                request.action = action;
                request.velocity = Velocity::Motion;
            }
        }
        if ready
            && request.action == 0x701
            && (self.kind == 2 || (self.kind == 0 && input.sequence > 93))
        {
            request.action = self.motion.action;
            request.velocity = Velocity::Keep;
        }
        if self.motion.frame == 0 && input.water {
            cue(self, request, random, StandingCue::Splash(3))?;
        }
        Ok(())
    }

    fn turn_standing(&mut self, turn: i16, right: bool) {
        self.rotation.y = if right {
            self.rotation.y.wrapping_add(turn)
        } else {
            self.rotation.y.wrapping_sub(turn)
        };
    }
}
