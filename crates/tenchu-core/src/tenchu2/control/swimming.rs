use super::{GrappleStatus, Request};
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        ai::direction,
        input::{ATTACK, CROUCH, DOWN, ITEM, JUMP, LEFT, RIGHT, UP},
        inventory::Item,
        physics::{Depth, Terrain},
        rig::{Grip, WeaponMode},
        terrain::{self, Query},
    },
};
use tenchu_assets::tenchu2::collision::Rules;

pub struct SwimInput {
    pub held: u16,
    pub tick: i32,
    pub camera_actor: bool,
    pub camera: i16,
    pub previous_camera: i16,
    pub aim: Position,
    pub grapple: GrappleStatus,
    pub model_height: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwimCue {
    Recover,
    CommitRoot,
    Submerge,
    PublishRoot,
    Jump,
    Attack,
    Voice(u8),
    Splash(u8),
    Weapon(WeaponMode),
    Grip {
        slot: usize,
        spare: usize,
        grip: Grip,
    },
    EmptyItem,
    LaunchGrapple,
    UpdateGrapple,
    BreathingTube(bool),
    UseUnderwaterItem,
}

impl Actor {
    pub fn update_swimming(
        &mut self,
        request: &mut Request,
        input: &mut SwimInput,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        random: &mut Random,
        mut cue: impl FnMut(
            &mut Actor,
            &mut Request,
            &mut SwimInput,
            &mut Random,
            SwimCue,
        ) -> Result<()>,
    ) -> Result<()> {
        let mut border = if terrain.water.depth == Some(Depth::Swimming) {
            self.swim_border(terrain, 0, false)?
        } else {
            0
        };
        let held = input.held;
        let pressed = held & !self.pad.previous;
        let depth = terrain.water.bottom.wrapping_sub(terrain.water.surface);
        match self.motion.action {
            0x500 | 0x501 => {
                if matches!(terrain.water.depth, None | Some(Depth::Shallow))
                    || request.action == 0x500 && held & UP == 0
                    || request.action == 0x501 && held & DOWN == 0
                {
                    cue(self, request, input, random, SwimCue::Recover)?;
                } else if pressed & ATTACK != 0 {
                    cue(self, request, input, random, SwimCue::Attack)?;
                } else if held & JUMP != 0 {
                    cue(self, request, input, random, SwimCue::Jump)?;
                } else {
                    self.swim_turn(held, self.turn, resources)?;
                }
                let clip = resources
                    .clips
                    .get(&self.motion.clip.ok_or("No Tenchu 2 swim motion")?)
                    .ok_or("Missing Tenchu 2 swim motion")?;
                if self.motion.frame == 0 || self.motion.frame == clip.duration / 2 {
                    cue(self, request, input, random, SwimCue::Splash(2))?;
                }
            }
            0x510 => {
                if self.motion.loops > 0 {
                    request.action = 0x512;
                    request.velocity = Velocity::Motion;
                    self.rotation.x = 0;
                } else if self.motion.frame == 1 {
                    cue(self, request, input, random, SwimCue::Splash(2))?;
                } else if random.next15() % 5 == 0 {
                    cue(self, request, input, random, SwimCue::Splash(3))?;
                    self.rotation.x /= 2;
                }
            }
            0x511 => {
                let mut land = true;
                for angle in [-64, 64] {
                    let [x, _, z] =
                        resources
                            .math
                            .velocity(self.rotation.y.wrapping_add(angle), 750, 0);
                    let floor = terrain::level(
                        terrain.map,
                        terrain.cursor,
                        Position::new(
                            self.position.x.wrapping_add(i32::from(x)),
                            self.position.y.wrapping_sub(750),
                            self.position.z.wrapping_add(i32::from(z)),
                        ),
                        Query::Uncached,
                    )?;
                    land &= current_material(terrain) == Some(0)
                        && floor.unwrap_or(i32::MIN) <= self.position.y;
                }
                let next = if land {
                    Some(0x520)
                } else if held & JUMP != 0 && depth >= 2000 {
                    Some(0x521)
                } else if held & UP == 0 {
                    Some(0x512)
                } else {
                    None
                };
                if let Some(action) = next {
                    request.action = action;
                    request.velocity = Velocity::Motion;
                } else {
                    self.swim_turn(held, self.turn, resources)?;
                }
                self.escape_water(border, 50, terrain.rules);
                if self.motion.frame == 0 {
                    cue(self, request, input, random, SwimCue::Splash(2))?;
                }
            }
            0x512 => {
                let next = if held & CROUCH != 0 && depth >= 1000 {
                    Some(0x536)
                } else if held & JUMP != 0 && depth >= 2000 {
                    Some(0x521)
                } else if pressed & ITEM != 0
                    && input.camera_actor
                    && self.inventory.selected == Some(Item::GRAPPLE)
                {
                    Some(0x514)
                } else if held & UP != 0 {
                    Some(0x511)
                } else {
                    None
                };
                if let Some(action) = next {
                    request.action = action;
                    request.velocity = Velocity::Motion;
                } else {
                    self.swim_turn(held, self.turn, resources)?;
                }
                if border != 0 {
                    self.escape_water(border, 50, terrain.rules);
                    input.held = 0;
                }
                self.set_horizontal_velocity(
                    if input.held & DOWN != 0 { -30 } else { 0 },
                    0,
                    &resources.math,
                );
            }
            0x513 => {
                if self.motion.frame == 1 {
                    cue(self, request, input, random, SwimCue::Splash(1))?;
                    if held & ATTACK != 0 && self.kind != 2 {
                        cue(
                            self,
                            request,
                            input,
                            random,
                            SwimCue::Weapon(WeaponMode::Draw),
                        )?;
                    }
                } else {
                    self.rotation.x /= 2;
                }
                if self.motion.loops > 0 {
                    cue(self, request, input, random, SwimCue::CommitRoot)?;
                    request.action = 0x606;
                    self.position.y = self.position.y.wrapping_sub(80);
                    self.rotation.x = 0;
                    self.change_motion(0x606, Velocity::Motion, resources)?;
                    self.motion.frame = 5;
                    self.set_horizontal_velocity(150, 0, &resources.math);
                    cue(self, request, input, random, SwimCue::Voice(21))?;
                    cue(self, request, input, random, SwimCue::Voice(11))?;
                }
            }
            0x514 => {
                if self.motion.frame == 1 {
                    input.previous_camera = input.camera;
                    input.camera = 20;
                    cue(self, request, input, random, SwimCue::Voice(32))?;
                } else if self.motion.loops > 0 {
                    self.motion.loops = -1;
                } else if held & ITEM == 0 {
                    request.action = 0x515;
                    request.velocity = Velocity::Motion;
                } else if held & ATTACK != 0 {
                    cue(self, request, input, random, SwimCue::EmptyItem)?;
                    request.action = 0x512;
                    request.velocity = Velocity::Motion;
                    input.camera = input.previous_camera;
                }
            }
            0x515 => {
                if self.motion.frame == 1 {
                    input.camera = 19;
                    let yaw = self.rotation.y;
                    let angle = direction(&resources.math, input.aim.x, input.aim.z, yaw)?;
                    if angle < 1501 {
                        self.rotation.y = yaw.wrapping_add(angle);
                    }
                    cue(self, request, input, random, SwimCue::LaunchGrapple)?;
                    cue(self, request, input, random, SwimCue::UpdateGrapple)?;
                    if input.grapple == GrappleStatus::Failed {
                        request.action = 0x512;
                        request.velocity = Velocity::Motion;
                        self.rotation.y = yaw;
                        input.camera = input.previous_camera;
                        return Ok(());
                    }
                    cue(self, request, input, random, SwimCue::Voice(33))?;
                } else {
                    if self.motion.loops > 0 {
                        self.motion.loops = -1;
                        self.update_velocity(Velocity::Stop, resources)?;
                    }
                    cue(self, request, input, random, SwimCue::UpdateGrapple)?;
                    match input.grapple {
                        GrappleStatus::Flying => return Ok(()),
                        GrappleStatus::Attached => {
                            self.orient_grapple(input.aim, &resources.math)?;
                            cue(self, request, input, random, SwimCue::PublishRoot)?;
                            request.action = 0x804;
                            request.velocity = Velocity::Motion;
                            self.motion.mask = u32::MAX - 1;
                            cue(self, request, input, random, SwimCue::Voice(22))?;
                            cue(self, request, input, random, SwimCue::Splash(1))?;
                        }
                        _ => {
                            request.action = 0x512;
                            request.velocity = Velocity::Motion;
                            input.camera = input.previous_camera;
                        }
                    }
                }
            }
            0x520 => {
                if self.motion.frame == 1 {
                    cue(self, request, input, random, SwimCue::Splash(2))?;
                } else if self.motion.loops > 0 {
                    cue(self, request, input, random, SwimCue::Recover)?;
                }
                if self.motion.frame >= 40 {
                    let forward = if terrain.water.depth.is_none() {
                        20
                    } else {
                        self.velocity.y = -15;
                        45
                    };
                    self.set_horizontal_velocity(forward, 0, &resources.math);
                }
            }
            0x521 => {
                border = self.swim_border(terrain, border, true)?;
                self.escape_water(border, 100, terrain.rules);
                if terrain.water.depth == Some(Depth::Swimming) {
                    let splash = if self.motion.frame == 1 {
                        Some(2)
                    } else if self.motion.loops > 0 {
                        Some(4)
                    } else if random.next15() % 5 == 0 {
                        Some(3)
                    } else {
                        None
                    };
                    if let Some(splash) = splash {
                        cue(self, request, input, random, SwimCue::Splash(splash))?;
                    }
                }
                self.position.y = self.position.y.wrapping_add(150);
                if depth <= 1000 {
                    request.action = 0x512;
                    request.velocity = Velocity::Motion;
                }
                if self.motion.loops > 0 {
                    if self.kind != 2 {
                        cue(
                            self,
                            request,
                            input,
                            random,
                            SwimCue::Weapon(WeaponMode::Sheathe),
                        )?;
                    }
                    cue(self, request, input, random, SwimCue::Submerge)?;
                    self.height = ((i32::from(input.model_height) + 200) / 2) as i16;
                    self.rotation.x = 0;
                    cue(self, request, input, random, SwimCue::CommitRoot)?;
                    request.action = 0x530;
                    request.velocity = Velocity::Motion;
                }
            }
            0x522 => {
                if self.motion.frame == 1 {
                    cue(self, request, input, random, SwimCue::Splash(1))?;
                    if self.kind == 2 && self.flags & 0x200000 != 0 {
                        cue(
                            self,
                            request,
                            input,
                            random,
                            SwimCue::Grip {
                                slot: 0,
                                spare: 3,
                                grip: Grip::Release,
                            },
                        )?;
                    }
                } else if self.motion.loops > 0 {
                    request.action = 0x512;
                    request.velocity = Velocity::Motion;
                } else if random.next15() % 5 == 0 {
                    cue(self, request, input, random, SwimCue::Splash(3))?;
                }
                border = self.swim_border(terrain, border, true)?;
                self.escape_water(border, 100, terrain.rules);
            }
            0x530 | 0x531 => {
                if self.motion.action == 0x530 {
                    let next = if pressed & JUMP != 0 {
                        Some(0x533)
                    } else if pressed & ITEM != 0
                        && self.inventory.counts[3] != 0
                        && self
                            .inventory
                            .selected
                            .is_some_and(|item| u8::from(item) == 3)
                    {
                        Some(0x537)
                    } else if held & (UP | RIGHT | DOWN | LEFT) != 0 {
                        Some(0x531)
                    } else {
                        None
                    };
                    if let Some(action) = next {
                        request.action = action;
                        request.velocity = Velocity::Motion;
                    }
                } else {
                    if pressed & JUMP != 0 {
                        request.action = 0x533;
                        request.velocity = Velocity::Motion;
                    } else if held & (UP | RIGHT | DOWN | LEFT) == 0 {
                        request.action = 0x530;
                        request.velocity = Velocity::Motion;
                    } else {
                        self.steer_underwater(held, self.turn / 2);
                    }
                    self.velocity = self.velocity.map(|v| v / 2);
                }
                if self.motion.frame == 0 && random.next15() & 1 != 0 {
                    cue(self, request, input, random, SwimCue::Splash(5))?;
                }
            }
            0x532 => {
                self.rotation.x /= 2;
                if self.motion.frame == 1 {
                    cue(self, request, input, random, SwimCue::Voice(26))?;
                } else if self.motion.loops > 0 {
                    request.action = 0x530;
                    request.velocity = Velocity::Motion;
                    self.rotation.y = self.rotation.y.wrapping_add(2048);
                    self.motion.pose.rotation[0][1] = 0;
                    cue(self, request, input, random, SwimCue::Splash(5))?;
                }
            }
            0x533 | 0x534 => {
                let kicks = || {
                    self.pad
                        .history
                        .iter()
                        .filter(|&&pad| pad & JUMP != 0)
                        .count()
                };
                if self.motion.action == 0x533 {
                    if self.motion.loops > 0 {
                        self.motion.loops = -1;
                    } else if self.motion.loops < 0 {
                        self.motion.loops = self.motion.loops.wrapping_sub(1);
                        if self.motion.loops <= -31 {
                            request.action = 0x530;
                            request.velocity = Velocity::Motion;
                        } else if kicks() >= 3 {
                            request.action = 0x534;
                            request.velocity = Velocity::Motion;
                        } else if kicks() != 0 {
                            self.motion.loops = 0;
                        }
                    }
                } else if self.motion.loops >= 2 {
                    request.action = 0x533;
                    request.velocity = Velocity::Motion;
                } else if self.motion.loops != 0 && kicks() >= 3 {
                    self.motion.loops = 0;
                }
                if self.motion.frame == 1 && random.next15() & 1 != 0 {
                    cue(self, request, input, random, SwimCue::Splash(5))?;
                }
                if held & (JUMP | CROUCH) == JUMP | CROUCH {
                    request.action = 0x532;
                    request.velocity = Velocity::Motion;
                } else if held & (JUMP | UP | RIGHT | DOWN | LEFT) != 0 {
                    self.steer_underwater(held, self.turn);
                    self.update_velocity(Velocity::Motion, resources)?;
                    let clip = resources
                        .clips
                        .get(&self.motion.clip.ok_or("No Tenchu 2 swim motion")?)
                        .ok_or("Missing Tenchu 2 swim motion")?;
                    self.velocity.y = resources.math.velocity(
                        self.rotation.x,
                        clip.forward_speed,
                        clip.side_speed,
                    )[0]
                    .wrapping_neg();
                }
            }
            0x535 => {
                if self.motion.loops > 0 {
                    request.action = 0x530;
                    request.velocity = Velocity::Motion;
                }
            }
            0x536 => {
                if held & CROUCH == 0 {
                    request.action = 0x512;
                    request.velocity = Velocity::Motion;
                } else {
                    self.swim_turn(held, self.turn / 3, resources)?;
                }
                if self.motion.frame == 1 {
                    cue(self, request, input, random, SwimCue::BreathingTube(true))?;
                }
                if request.action != 0x536 {
                    cue(self, request, input, random, SwimCue::BreathingTube(false))?;
                }
                if random.next15() % 10 == 0 {
                    cue(self, request, input, random, SwimCue::Splash(3))?;
                }
            }
            0x537 => {
                if self.motion.loops > 0 {
                    request.action = 0x530;
                    request.velocity = Velocity::Motion;
                } else if self.motion.frame == 1 {
                    self.inventory.counts[3] = self.inventory.counts[3].wrapping_sub(1);
                } else {
                    let clip = resources
                        .clips
                        .get(&self.motion.clip.ok_or("No Tenchu 2 swim motion")?)
                        .ok_or("Missing Tenchu 2 swim motion")?;
                    if self.motion.frame == clip.duration / 2 {
                        self.breath = 100;
                        cue(self, request, input, random, SwimCue::UseUnderwaterItem)?;
                    }
                }
            }
            _ => {}
        }
        if terrain.water.depth != Some(Depth::Submerged)
            && (self.motion.loops >= 0 && self.motion.frame == 0 || input.tick % 30 == 0)
        {
            cue(self, request, input, random, SwimCue::Splash(3))?;
        }
        Ok(())
    }

    fn swim_turn(&mut self, held: u16, speed: i16, resources: &Resources) -> Result<()> {
        if held & (RIGHT | LEFT) != 0 {
            self.rotation.y = if held & RIGHT != 0 {
                self.rotation.y.wrapping_add(speed)
            } else {
                self.rotation.y.wrapping_sub(speed)
            };
            self.update_velocity(Velocity::Motion, resources)?;
        }
        Ok(())
    }

    fn steer_underwater(&mut self, held: u16, speed: i16) {
        if held & RIGHT != 0 {
            self.rotation.y = self.rotation.y.wrapping_add(speed);
        }
        if held & LEFT != 0 {
            self.rotation.y = self.rotation.y.wrapping_sub(speed);
        }
        if held & UP != 0 && self.rotation.x < 512 {
            self.rotation.x = self.rotation.x.wrapping_add(speed);
        }
        if held & DOWN != 0 && self.rotation.x > -512 {
            self.rotation.x = self.rotation.x.wrapping_sub(speed);
        }
    }

    pub(super) fn escape_water(&mut self, border: u16, speed: i16, rules: &Rules) {
        if border != 0 {
            let [x, z] = rules.escape[usize::from(border)].map(|v| v.wrapping_mul(speed));
            self.position.x = self.position.x.wrapping_add(i32::from(x));
            self.position.z = self.position.z.wrapping_add(i32::from(z));
        }
    }

    pub(super) fn swim_border(
        &self,
        terrain: &mut Terrain<'_>,
        mut border: u16,
        diving: bool,
    ) -> Result<u16> {
        let (radius, height, threshold) = if diving {
            (1000, 100, terrain.water.surface)
        } else {
            (500, 500, self.position.y)
        };
        for (i, [x, z]) in terrain.rules.corners.into_iter().enumerate() {
            let floor = terrain::level(
                terrain.map,
                terrain.cursor,
                Position::new(
                    self.position.x.wrapping_add(i32::from(x) * radius),
                    terrain.water.surface.wrapping_sub(height),
                    self.position.z.wrapping_add(i32::from(z) * radius),
                ),
                Query::Uncached,
            )?;
            let material = current_material(terrain);
            let blocked = if diving {
                material == Some(0)
            } else {
                material.is_some_and(|m| m != 0x200 && m != 0x300)
            };
            if floor.is_none_or(|floor| floor < threshold) || blocked {
                border |= 1 << i;
            }
        }
        Ok(border)
    }
}

fn current_material(terrain: &Terrain<'_>) -> Option<u16> {
    let surface = terrain.cursor.current?;
    Some(terrain.map.cells[surface.cell].nodes[surface.node].attributes & 0xf00)
}
