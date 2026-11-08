use super::{ItemSound, Request};
use crate::{
    Result,
    math::Position,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        input::{ATTACK, CROUCH, DOWN, GUARD, ITEM, LEFT, RIGHT, UP},
        inventory::{Item, Unavailable},
        physics::Terrain,
    },
};

pub struct WallInput {
    pub held: u16,
    pub player: bool,
    pub allowed: bool,
    pub camera: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallCue {
    PublishRoot,
    Recover,
    Voice(u8),
    Sound(ItemSound),
    Throw { item: Item, direction: Position },
}

impl Actor {
    pub fn update_wall(
        &mut self,
        request: &mut Request,
        input: &mut WallInput,
        terrain: &mut Terrain<'_>,
        resources: &mut Resources,
        mut cue: impl FnMut(&mut Actor, &mut Request, u16, &Resources, WallCue) -> Result<()>,
    ) -> Result<()> {
        let held = input.held;
        match self.motion.action {
            0x300 => {
                if self.motion.frame < 0 {
                    if !self.enter_wall(request, held, input.allowed, terrain, resources)? {
                        request.action = 0x200;
                        request.velocity = Velocity::Motion;
                        self.motion.mask = u32::MAX;
                        return Ok(());
                    }
                    self.align_to_wall(terrain, resources)?;
                } else if self.motion.loops > 0 {
                    self.motion.loops = -1;
                }
                if let Some(action) = self.pad.consume(&resources.commands.crouched) {
                    self.leave_wall(request, action, Velocity::Motion, resources)?;
                    return Ok(());
                }
                if self.motion.loops == -1 {
                    if held & (UP | RIGHT | DOWN | LEFT) != 0 {
                        if held & GUARD != 0 {
                            let action = if held & UP != 0 {
                                0x210
                            } else if held & DOWN != 0 {
                                0x211
                            } else if held & RIGHT != 0 {
                                0x212
                            } else {
                                0x213
                            };
                            self.leave_wall(request, action, Velocity::Motion, resources)?;
                            return Ok(());
                        }
                        let direction = wall_direction(held);
                        let facing = (self.motion.pose.rotation[0][1] as u16 >> 10) & 3;
                        if facing != (direction + 2) & 3 {
                            request.action = if facing == (direction + 1) & 3 {
                                0x301
                            } else {
                                0x302
                            };
                            self.motion.select(
                                request.action,
                                self.kind,
                                &resources.registry,
                                &resources.clips,
                            )?;
                            self.state = State::Wall;
                            self.velocity.x = 0;
                            self.velocity.z = 0;
                            self.motion.mask = u32::MAX - 1;
                            self.motion.pose.rotation[0][0] = -105;
                            cue(self, request, held, resources, WallCue::PublishRoot)?;
                        }
                    } else if held & !self.pad.previous & ITEM != 0 {
                        let facing = (self.motion.pose.rotation[0][1] as u16 >> 10) & 3;
                        let (required, action) = match input.camera {
                            9 => (2, 0x310),
                            10 => (2, 0x311),
                            11 => (3, 0x310),
                            12 => (1, 0x311),
                            _ => return Ok(()),
                        };
                        match self.inventory.take() {
                            Ok(item) => {
                                if facing == required
                                    && matches!(u8::from(item), 5 | 6 | 10 | 11 | 18)
                                {
                                    request.action = action;
                                    request.velocity = Velocity::Motion;
                                    self.motion.mask = u32::MAX - 1;
                                } else {
                                    cue(
                                        self,
                                        request,
                                        held,
                                        resources,
                                        WallCue::Sound(ItemSound::EmptyInventory),
                                    )?;
                                    self.inventory.refund();
                                }
                            }
                            Err(Unavailable::Empty) if input.player => {
                                cue(
                                    self,
                                    request,
                                    held,
                                    resources,
                                    WallCue::Sound(ItemSound::EmptyInventory),
                                )?;
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
            0x301 | 0x302 => {
                if let Some(action) = self.pad.consume(&resources.commands.crouched) {
                    self.leave_wall(request, action, Velocity::Motion, resources)?;
                    return Ok(());
                }
                if self.motion.frame >= 0 {
                    if held & (UP | RIGHT | DOWN | LEFT) == 0 {
                        request.action = 0x300;
                        request.velocity = Velocity::Motion;
                        self.motion.mask = u32::MAX;
                    } else {
                        let facing = (self.motion.pose.rotation[0][1] as u16 >> 10) & 3;
                        let direction = wall_direction(held);
                        if facing != (direction + 2) & 3 {
                            let action = if facing == (direction + 1) & 3 {
                                0x301
                            } else {
                                0x302
                            };
                            if request.action != action {
                                request.action = action;
                                self.motion.select(
                                    action,
                                    self.kind,
                                    &resources.registry,
                                    &resources.clips,
                                )?;
                            }
                            let (forward, side) = if held & UP != 0 {
                                (30, 0)
                            } else if held & DOWN != 0 {
                                (-30, 0)
                            } else if held & LEFT != 0 {
                                (0, 30)
                            } else {
                                (0, -30)
                            };
                            self.set_horizontal_velocity(forward, side, &resources.math);
                            let root_yaw = self.motion.pose.rotation[0][1];
                            let yaw = self.rotation.y;
                            self.position.x =
                                self.position.x.wrapping_add(i32::from(self.velocity.x));
                            self.position.z =
                                self.position.z.wrapping_add(i32::from(self.velocity.z));
                            let mut valid =
                                self.enter_wall(request, held, input.allowed, terrain, resources)?;
                            if valid
                                && root_yaw.wrapping_add(yaw) & 4095
                                    != terrain.rules.wall_angles
                                        [usize::from(terrain.scratch.blocked)]
                                    .ok_or("Missing Tenchu 2 wall angle")?
                            {
                                if facing == direction {
                                    input.held = 0;
                                } else {
                                    valid = false;
                                }
                            }
                            if !valid {
                                request.action = 0x300;
                                self.position.x =
                                    self.position.x.wrapping_sub(i32::from(self.velocity.x));
                                self.position.z =
                                    self.position.z.wrapping_sub(i32::from(self.velocity.z));
                                self.motion.select(
                                    0x300,
                                    self.kind,
                                    &resources.registry,
                                    &resources.clips,
                                )?;
                                self.motion.loops = -1;
                                self.motion.mask = u32::MAX;
                            }
                            self.velocity.x = 0;
                            self.velocity.z = 0;
                            if self.motion.frame == 1 {
                                cue(self, request, input.held, resources, WallCue::Voice(17))?;
                            }
                        }
                    }
                }
            }
            0x310 | 0x311 => {
                if self.motion.loops > 0 {
                    request.action = 0x300;
                    request.velocity = Velocity::Motion;
                    self.motion.mask = u32::MAX;
                    return Ok(());
                }
                let clip = resources
                    .clips
                    .get(&self.motion.clip.ok_or("No Tenchu 2 wall motion")?)
                    .ok_or("Missing Tenchu 2 wall motion")?;
                if i32::from(self.motion.frame) == i32::from(clip.duration) * 2 / 3
                    && matches!(u8::from(self.inventory.active), 5 | 6 | 10 | 11 | 18)
                {
                    let [sin, cos] = resources.math.pair(self.rotation.y);
                    cue(
                        self,
                        request,
                        held,
                        resources,
                        WallCue::Throw {
                            item: self.inventory.active,
                            direction: Position::new(-sin, 0, -cos),
                        },
                    )?;
                }
                return Ok(());
            }
            _ => {}
        }
        let held = input.held;
        if held & CROUCH != 0 {
            if held & !self.pad.previous & ATTACK != 0 && !self.weapon_drawn() {
                request.action = 0x700;
                request.velocity = Velocity::Motion;
                if self
                    .change_motion(request.action, request.velocity, resources)?
                    .is_some()
                {
                    self.motion.mask = u32::MAX;
                    self.motion.frame = -10;
                }
            }
            return Ok(());
        }
        let old = self.motion.action;
        self.update_velocity(Velocity::Stop, resources)?;
        cue(self, request, held, resources, WallCue::Recover)?;
        self.motion.mask = u32::MAX;
        self.motion.frame = -5;
        if held & UP == 0 {
            return Ok(());
        }
        let speed = 50;
        let action = match (old, input.camera) {
            (0x301, 12) => {
                self.motion.pose.rotation[0][1] =
                    self.motion.pose.rotation[0][1].wrapping_add(1024);
                self.set_horizontal_velocity(speed, 0, &resources.math);
                self.rotation.y = self.rotation.y.wrapping_sub(1024);
                0x113
            }
            (0x302, 11) => {
                self.motion.pose.rotation[0][1] =
                    self.motion.pose.rotation[0][1].wrapping_sub(1024);
                self.set_horizontal_velocity(speed, 0, &resources.math);
                self.rotation.y = self.rotation.y.wrapping_add(1024);
                0x114
            }
            (0x300, 10) => {
                self.set_horizontal_velocity(0, -speed, &resources.math);
                0x113
            }
            (0x300, 9) => {
                self.set_horizontal_velocity(0, speed, &resources.math);
                0x114
            }
            _ => return Ok(()),
        };
        request.action = action;
        request.velocity = Velocity::Keep;
        self.change_motion(action, request.velocity, resources)?;
        self.motion.frame = -10;
        self.motion.pose.translation[0].x = 0;
        self.motion.pose.translation[0].z = 0;
        Ok(())
    }

    fn leave_wall(
        &mut self,
        request: &mut Request,
        action: i16,
        velocity: Velocity,
        resources: &Resources,
    ) -> Result<()> {
        request.action = action;
        request.velocity = velocity;
        self.motion.mask = u32::MAX;
        self.change_motion(action, velocity, resources)?;
        self.motion.frame = -5;
        Ok(())
    }

    fn align_to_wall(&mut self, terrain: &Terrain<'_>, resources: &mut Resources) -> Result<()> {
        let mut rounded = self.rotation.y & 0xc00;
        if self.rotation.y & 0x200 != 0 {
            rounded += 1024;
        }
        let blocked = terrain.scratch.blocked;
        let mut delta = if blocked == 12 && rounded == 4096 {
            0
        } else {
            terrain.rules.wall_angles[usize::from(blocked)]
                .ok_or("Missing Tenchu 2 wall angle")?
                .wrapping_sub(rounded)
        };
        if delta == 0 {
            self.rotation.y = self.rotation.y.wrapping_add(2048);
        }
        if i32::from(delta).abs() > 2048 {
            delta = if delta > 0 {
                delta.wrapping_sub(4096)
            } else {
                delta.wrapping_add(4096)
            };
        }
        if delta & 0xf00 == 0x800 {
            delta = 2048;
        }
        let turn =
            (i32::from(rounded) - i32::from(self.rotation.y)) / -i32::from(self.motion.frame);
        self.rotation.y = self.rotation.y.wrapping_add(turn as i16);
        let [x, _, z] = resources.math.velocity(delta, -300, 0);
        // Retail changes shared keys here; other actors using this clip see the same edits.
        let clip = resources
            .clips
            .get_mut(&self.motion.clip.ok_or("No Tenchu 2 wall motion")?)
            .ok_or("Missing Tenchu 2 wall motion")?;
        clip.bones
            .first_mut()
            .and_then(|keys| keys.first_mut())
            .ok_or("Missing Tenchu 2 wall root track")?
            .value[1] = delta;
        clip.bones
            .get_mut(2)
            .and_then(|keys| keys.first_mut())
            .ok_or("Missing Tenchu 2 wall torso track")?
            .value[1] = if delta & 0x400 != 0 {
            delta.wrapping_neg()
        } else {
            0
        };
        let root = clip.root.first_mut().ok_or("Missing Tenchu 2 wall root")?;
        root.value[0] = x;
        root.value[2] = z;
        Ok(())
    }
}

fn wall_direction(held: u16) -> u16 {
    ((held >> 4) & 15).trailing_zeros() as u16
}
