use super::Request;
use crate::{
    Result,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        input::CROUCH,
        physics::{Depth, FALLING, HIT, IN_WATER, Water},
    },
};

pub struct ControlInput {
    pub held: u16,
    pub player: bool,
    pub camera_actor: bool,
    pub camera: i16,
    pub script: u16,
    pub tick: i32,
    pub water: Water,
    pub model_height: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlCue {
    Damage,
    Ledge,
    Recover,
    Splash(u8),
    Voice(u8),
    DrowningRumble(i16),
    SelectItem { previous: bool },
    ItemSound,
}

pub trait ControlDriver {
    fn motion(
        &mut self,
        actor: &mut Actor,
        request: &Request,
        resources: &Resources,
    ) -> Result<()> {
        actor
            .change_motion(request.action, request.velocity, resources)
            .map(|_| ())
    }
    fn cue(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        cue: ControlCue,
    ) -> Result<()>;
    fn state(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
    ) -> Result<()>;
}

impl Actor {
    pub fn update_controls(
        &mut self,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
        driver: &mut impl ControlDriver,
    ) -> Result<()> {
        input.held = self.pad.held;
        *request = Request {
            action: self.motion.action,
            velocity: Velocity::Keep,
        };
        if self.flags & HIT != 0 {
            driver.cue(self, request, input, ControlCue::Damage)?;
        } else if self.flags & FALLING != 0 {
            self.falling_control(request, input, resources, driver)?;
        } else if self.flags & IN_WATER != 0
            && self.water_control(request, input, resources, driver)?
        {
            return Ok(());
        }
        self.breathing_control(request, input, resources, driver)?;
        if input.camera_actor {
            if input.script == 3 {
                input.held = 0;
            }
            if matches!(input.camera, 6 | 20) {
                input.held &= !0xf0;
            }
            if input.held & !self.pad.previous & 0x300 != 0
                && !matches!(self.state, State::Dead | State::Hurt | State::Item)
                && self.motion.action != 0x514
            {
                driver.cue(
                    self,
                    request,
                    input,
                    ControlCue::SelectItem {
                        previous: input.held & 0x100 == 0,
                    },
                )?;
                driver.cue(self, request, input, ControlCue::ItemSound)?;
            }
        }
        driver.state(self, request, input)?;
        if request.action != self.motion.action || request.velocity != Velocity::Keep {
            driver.motion(self, request, resources)?;
        }
        Ok(())
    }

    fn falling_control(
        &mut self,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
        driver: &mut impl ControlDriver,
    ) -> Result<()> {
        let state = self.state;
        if state == State::Ledge {
            return Ok(());
        }
        if state == State::Crouched && self.motion.loops == 0 {
            return driver.cue(self, request, input, ControlCue::Ledge);
        }
        if state == State::Jumping {
            return if request.action != 0x607 {
                driver.cue(self, request, input, ControlCue::Ledge)
            } else {
                Ok(())
            };
        }
        if request.action == 0x702 {
            return driver.cue(self, request, input, ControlCue::Ledge);
        }
        if matches!(request.action, 0x911 | 0x804 | 0xa21 | 0xa22) {
            return Ok(());
        }
        if self.terrain.height > i32::from(self.height) * 2 {
            if self.health == 0 && self.terrain.height < 101 {
                self.velocity.y = 0;
            } else {
                request.action = 0x702;
                request.velocity = Velocity::Keep;
                driver.motion(self, request, resources)?;
                if state == State::Wall {
                    self.motion.mask = u32::MAX;
                    self.motion.frame = -5;
                }
            }
        }
        Ok(())
    }

    fn water_control(
        &mut self,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
        driver: &mut impl ControlDriver,
    ) -> Result<bool> {
        if self.kind == 0x75 {
            return Ok(false);
        }
        if self.health == 0 {
            if matches!(request.action, 0xb08 | 0xb0a) {
                return Ok(false);
            }
            request.action = if self.kind & 0xf0 == 0x70 {
                0xb0a
            } else {
                0xb08
            };
            request.velocity = Velocity::Motion;
            driver.motion(self, request, resources)?;
            let clip = resources
                .clips
                .get(&self.motion.clip.ok_or("No Tenchu 2 drowning motion")?)
                .ok_or("Missing Tenchu 2 drowning motion")?;
            self.motion.frame = clip.duration.wrapping_sub(1);
            driver.cue(self, request, input, ControlCue::Splash(1))?;
            return Ok(true);
        }
        let mut restart = false;
        match input.water.depth {
            Some(Depth::Shallow) if self.state == State::Swimming => {
                driver.cue(self, request, input, ControlCue::Recover)?;
                self.position.y = input.water.bottom;
                restart = true;
            }
            Some(Depth::Wading) => match request.action {
                3 | 0x103 => request.action = 0x500,
                4 | 0x104 => request.action = 0x501,
                0x510..=0x512 | 0x530..=0x537 => {
                    if request.action >= 0x530 {
                        self.rotation.x = 0;
                    }
                    driver.cue(self, request, input, ControlCue::Recover)?;
                    self.position.y = input.water.bottom;
                    restart = true;
                }
                _ => {}
            },
            Some(Depth::Swimming) => {
                match request.action {
                    0x533 | 0x534 if input.held & CROUCH != 0 => {
                        request.action = 0x536;
                        self.rotation.x = 0;
                    }
                    0x534 => request.action = 0x513,
                    0x530..=0x533 | 0x535 | 0x537 => request.action = 0x510,
                    0x510..=0x515 | 0x520..=0x522 | 0x536 | 0x606 | 0x804 | 0xa16 => {}
                    _ if self.state == State::Swimming => {
                        request.action = 0x512;
                        restart = true;
                    }
                    _ if self.state == State::Crouched => {
                        driver.cue(self, request, input, ControlCue::Voice(26))?;
                        request.action = 0x536;
                    }
                    _ => {
                        self.position.x =
                            self.position.x.wrapping_add(i32::from(self.velocity.x) * 2);
                        self.position.z =
                            self.position.z.wrapping_add(i32::from(self.velocity.z) * 2);
                        request.action = 0x522;
                        if input.player {
                            input.camera = 0;
                        }
                    }
                }
                self.position.y = input.water.surface;
                self.velocity.y = 0;
            }
            Some(Depth::Submerged)
                if !matches!(request.action, 0x530..=0x535 | 0x537 | 0x521 | 0xa04) =>
            {
                request.action = 0x530;
            }
            _ => {}
        }
        if request.action == self.motion.action {
            return Ok(false);
        }
        self.height = input.model_height.wrapping_add(200);
        driver.motion(
            self,
            &Request {
                action: request.action,
                velocity: Velocity::Motion,
            },
            resources,
        )?;
        if restart {
            self.motion.frame = 0;
        }
        Ok(true)
    }

    fn breathing_control(
        &mut self,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
        driver: &mut impl ControlDriver,
    ) -> Result<()> {
        if !input.player || (self.state != State::Swimming && self.breath == 100) {
            return Ok(());
        }
        let action = self.motion.action;
        if matches!(action, 0xa04 | 0xa16) && self.breath != 0 {
            self.breath -= 1;
        }
        if matches!(action, 0x530..=0x535 | 0xa04 | 0xa16) {
            let period = if action == 0x534 { 6 } else { 10 };
            if self.breath != 0 && input.tick % period == 0 {
                self.breath -= 1;
            }
            if self.breath == 0 && input.tick % 3 == 0 {
                let strength = self.max_health.wrapping_sub(self.health).wrapping_mul(2);
                driver.cue(self, request, input, ControlCue::DrowningRumble(strength))?;
                self.health = self.health.wrapping_sub(1);
                if self.health <= 0 {
                    self.health = 0;
                    request.action = 0xb08;
                    request.velocity = Velocity::Motion;
                    self.height = input.model_height.wrapping_add(200);
                    driver.motion(self, request, resources)?;
                }
            }
        } else if action != 0xb08 && input.tick % 3 == 0 {
            self.breath = self.breath.wrapping_add(1).min(100);
        }
        Ok(())
    }
}
