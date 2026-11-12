use super::super::{GrappleStatus, Request};
use crate::tenchu2::actor::flags::POISONED;
use crate::{
    Result,
    math::{Math, Position},
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        ai::direction,
        input::{ATTACK, ITEM},
        inventory::Item,
    },
};

pub struct ItemControlInput {
    pub held: u16,
    pub player: bool,
    pub camera_actor: bool,
    pub camera: i16,
    pub previous_camera: i16,
    pub camera_yaw: i16,
    pub aim: Position,
    pub target_position: Option<Position>,
    pub grapple: GrappleStatus,
    pub grapple_position: Position,
    pub difficulty: i16,
    pub map: i16,
    pub story_triggered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemCue {
    Recover,
    Voice(u8),
    Sound {
        id: u8,
        positional: bool,
    },
    PublishRoot,
    HeldModel(Option<u8>),
    LaunchGrapple(Position),
    UpdateGrapple,
    ReleaseGrapple,
    GrapplePosition,
    Use {
        item: Item,
        direction: Option<Position>,
    },
    Medicine(u8),
    ShowHealth,
}

impl Actor {
    pub fn update_item(
        &mut self,
        request: &mut Request,
        input: &mut ItemControlInput,
        resources: &Resources,
        random: &mut Random,
        mut cue: impl FnMut(
            &mut Actor,
            &mut Request,
            &mut ItemControlInput,
            &mut Random,
            ItemCue,
        ) -> Result<()>,
    ) -> Result<()> {
        let duration = resources
            .clips
            .get(&self.motion.clip.ok_or("No Tenchu 2 item motion")?)
            .ok_or("Missing Tenchu 2 item motion")?
            .duration;
        match self.motion.action {
            0x800 | 0x801 => {
                if self.motion.frame == 1 {
                    self.update_velocity(Velocity::Stop, resources)?;
                }
            }
            0x802 => {
                if input.held & ITEM == 0 {
                    request.action = 0x803;
                    request.velocity = Velocity::Motion;
                } else if self.motion.frame == 1 {
                    cue(self, request, input, random, ItemCue::Voice(32))?;
                } else if self.motion.loops > 0 {
                    self.motion.loops = -1;
                    self.hold_item_aim(input, resources)?;
                }
            }
            0x803 => {
                let release = 4;
                if self.motion.loops != 0 || self.motion.frame > release {
                    if self.inventory.active == Item::GRAPPLE {
                        if self.motion.loops > 0 {
                            self.motion.loops = -1;
                            self.update_velocity(Velocity::Stop, resources)?;
                        }
                        cue(self, request, input, random, ItemCue::UpdateGrapple)?;
                        match input.grapple {
                            GrappleStatus::Flying => return Ok(()),
                            GrappleStatus::Attached => {
                                self.orient_grapple(input.aim, &resources.math)?;
                                cue(self, request, input, random, ItemCue::PublishRoot)?;
                                request.action = 0x804;
                                request.velocity = Velocity::Motion;
                                self.motion.mask = u32::MAX - 1;
                                cue(self, request, input, random, ItemCue::Voice(22))?;
                            }
                            _ => self.motion.loops = 1,
                        }
                    }
                } else if self.motion.frame == release {
                    let yaw = self.rotation.y;
                    let direction = if input.camera_actor && input.camera == 20 {
                        input.camera = 19;
                        self.turn_to_aim(input.aim, &resources.math)?;
                        input.aim
                    } else if input.player {
                        let value = self.forward_direction(&resources.math);
                        input.aim = value;
                        value
                    } else {
                        let value = input
                            .target_position
                            .ok_or("Tenchu 2 throw has no target")?
                            .wrapping_sub(self.position);
                        input.aim = resources.math.normalize_vector(value);
                        value
                    };
                    if self.inventory.active == Item::GRAPPLE {
                        if !input.camera_actor {
                            return Ok(());
                        }
                        input.camera = 19;
                        cue(
                            self,
                            request,
                            input,
                            random,
                            ItemCue::LaunchGrapple(direction),
                        )?;
                        cue(self, request, input, random, ItemCue::UpdateGrapple)?;
                        if input.grapple == GrappleStatus::Failed {
                            self.rotation.y = yaw;
                            input.camera = input.previous_camera;
                            cue(self, request, input, random, ItemCue::Recover)?;
                            return Ok(());
                        }
                    } else if u8::from(self.inventory.active) == 4 {
                        if input.camera_actor {
                            input.camera = input.previous_camera;
                        }
                        cue(
                            self,
                            request,
                            input,
                            random,
                            ItemCue::Use {
                                item: self.inventory.active,
                                direction: Some(direction),
                            },
                        )?;
                        if self.kind & 0xf0 == 0x40
                            && random.next15() % 3 == 0
                            && input.difficulty == 1
                        {
                            self.motion.frame = 0;
                        }
                    }
                    cue(self, request, input, random, ItemCue::Voice(33))?;
                }
            }
            0x804 => {
                cue(self, request, input, random, ItemCue::UpdateGrapple)?;
                if input.grapple == GrappleStatus::Attached && self.flags & 0x5c000 == 0 {
                    input.aim = self.forward_direction(&resources.math);
                    cue(self, request, input, random, ItemCue::GrapplePosition)?;
                    self.position = input.grapple_position;
                    self.position.y = self.position.y.wrapping_add(i32::from(self.height) / 4);
                    return Ok(());
                }
                cue(self, request, input, random, ItemCue::ReleaseGrapple)?;
                let mut step = input.aim;
                while step.iter().any(|v| v.wrapping_abs() > 400) {
                    step = step.map(|v| v >> 1);
                }
                self.position.x = self.position.x.wrapping_sub(step.x);
                self.position.z = self.position.z.wrapping_sub(step.z);
                self.velocity = Default::default();
                if step.x.wrapping_abs() < step.y.wrapping_abs()
                    && step.z.wrapping_abs() < step.y.wrapping_abs()
                {
                    self.velocity.y = -100;
                }
                request.action = 0x702;
                request.velocity = Velocity::Keep;
                self.motion.mask = u32::MAX;
                self.change_motion(0x702, Velocity::Keep, resources)?;
                self.motion.frame = -10;
                return Ok(());
            }
            0x805 if self.motion.frame == duration / 2 => {
                cue(self, request, input, random, ItemCue::Voice(10))?;
                let aim = match u8::from(self.inventory.active) {
                    6 | 18 => Some(if input.camera_actor && input.camera == 6 {
                        self.rotation.y = self.rotation.y.wrapping_add(direction(
                            &resources.math,
                            input.aim.x,
                            input.aim.z,
                            self.rotation.y,
                        )?);
                        input.camera_yaw = 0;
                        input.aim
                    } else {
                        self.forward_direction(&resources.math)
                    }),
                    10 => None,
                    _ => return self.finish_item(request, input, random, cue),
                };
                cue(
                    self,
                    request,
                    input,
                    random,
                    ItemCue::Use {
                        item: self.inventory.active,
                        direction: aim,
                    },
                )?;
            }
            0x806 | 0x807 if self.motion.frame == duration / 2 => {
                let item = u8::from(self.inventory.active);
                if (self.motion.action == 0x806 && item == 5)
                    || (self.motion.action == 0x807 && item == 11)
                {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        ItemCue::Use {
                            item: self.inventory.active,
                            direction: None,
                        },
                    )?;
                    if item == 11
                        && !input.story_triggered
                        && input.map == 18
                        && self.position.x / 1000 == 31
                        && self.position.z / 1000 == -54
                    {
                        input.story_triggered = true;
                        self.max_health = 150;
                        self.health = 150;
                        self.flags &= !POISONED;
                        cue(self, request, input, random, ItemCue::Medicine(0))?;
                        cue(
                            self,
                            request,
                            input,
                            random,
                            ItemCue::Sound {
                                id: 1,
                                positional: false,
                            },
                        )?;
                    }
                }
            }
            0x808 => {
                if self.motion.frame == duration / 2 {
                    cue(self, request, input, random, ItemCue::Voice(34))?;
                }
                if self.motion.loops > 0 && matches!(u8::from(self.inventory.active), 7 | 12) {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        ItemCue::Use {
                            item: self.inventory.active,
                            direction: None,
                        },
                    )?;
                }
            }
            0x809 => {
                if input.player && self.motion.frame == 1 {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        ItemCue::HeldModel(Some(u8::from(self.inventory.active))),
                    )?;
                }
                if i32::from(self.motion.frame) == i32::from(duration) * 2 / 3 {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        ItemCue::Sound {
                            id: 54,
                            positional: true,
                        },
                    )?;
                    if input.player {
                        cue(self, request, input, random, ItemCue::HeldModel(None))?;
                    }
                    let effect = match u8::from(self.inventory.active) {
                        1 => {
                            self.health = self.max_health;
                            Some(0)
                        }
                        2 => {
                            self.health = self.health.wrapping_add(20).min(self.max_health);
                            self.flags &= !POISONED;
                            Some(1)
                        }
                        15 => {
                            self.health = self.health.wrapping_add(20).min(self.max_health);
                            Some(0)
                        }
                        _ => None,
                    };
                    if let Some(effect) = effect {
                        cue(self, request, input, random, ItemCue::Medicine(effect))?;
                    }
                    if !input.player {
                        cue(self, request, input, random, ItemCue::ShowHealth)?;
                    }
                }
            }
            0x80a if self.motion.frame == 1 => {
                cue(self, request, input, random, ItemCue::Voice(36))?;
                if u8::from(self.inventory.active) == 13 {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        ItemCue::Use {
                            item: self.inventory.active,
                            direction: None,
                        },
                    )?;
                }
            }
            0x80b | 0x80c => {
                let item = u8::from(self.inventory.active);
                if self.motion.action == 0x80b && item == 8
                    || self.motion.action == 0x80c && item == 9
                {
                    if item == 8 && input.player {
                        if self.motion.frame == 1 {
                            cue(self, request, input, random, ItemCue::HeldModel(Some(23)))?;
                        } else if self.motion.loops > 0 {
                            cue(self, request, input, random, ItemCue::HeldModel(None))?;
                        }
                    }
                    let hold = if item == 8 { 14 } else { 17 };
                    if self.motion.frame == hold && input.held & ITEM != 0 {
                        if self.motion.loops == 0 {
                            self.motion.frame = self.motion.frame.wrapping_add(1);
                            self.motion.loops = -1;
                        }
                        self.hold_item_aim(input, resources)?;
                    } else if self.motion.loops != 0 && self.motion.frame == hold + 1 {
                        if input.held & ITEM == 0 {
                            self.motion.loops = 0;
                        }
                    } else if self.motion.frame == hold + 2 {
                        let aim = if input.camera_actor && input.camera == 20 {
                            input.camera = input.previous_camera;
                            self.turn_to_aim(input.aim, &resources.math)?;
                            input.aim
                        } else if input.player || item == 9 {
                            self.forward_direction(&resources.math)
                        } else {
                            resources.math.normalize_vector(
                                input
                                    .target_position
                                    .ok_or("Tenchu 2 item has no target")?
                                    .wrapping_sub(self.position),
                            )
                        };
                        cue(
                            self,
                            request,
                            input,
                            random,
                            ItemCue::Use {
                                item: self.inventory.active,
                                direction: Some(aim),
                            },
                        )?;
                        if item == 8 {
                            cue(self, request, input, random, ItemCue::Voice(38))?;
                        }
                    }
                } else if self.motion.action == 0x80b && item == 14 && self.motion.frame == 15 {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        ItemCue::Use {
                            item: self.inventory.active,
                            direction: None,
                        },
                    )?;
                    cue(self, request, input, random, ItemCue::Voice(38))?;
                }
            }
            0x80d | 0x80e => {
                if u8::from(self.inventory.active) == 16 {
                    if self.motion.frame == 1 {
                        cue(
                            self,
                            request,
                            input,
                            random,
                            ItemCue::Use {
                                item: self.inventory.active,
                                direction: None,
                            },
                        )?;
                        cue(self, request, input, random, ItemCue::Voice(37))?;
                    } else if self.motion.frame > 109 {
                        cue(self, request, input, random, ItemCue::Recover)?;
                    }
                } else if u8::from(self.inventory.active) == 17 && self.motion.frame == 30 {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        ItemCue::Use {
                            item: self.inventory.active,
                            direction: None,
                        },
                    )?;
                }
            }
            _ => {}
        }
        self.finish_item(request, input, random, cue)
    }

    fn finish_item(
        &mut self,
        request: &mut Request,
        input: &mut ItemControlInput,
        random: &mut Random,
        mut cue: impl FnMut(
            &mut Actor,
            &mut Request,
            &mut ItemControlInput,
            &mut Random,
            ItemCue,
        ) -> Result<()>,
    ) -> Result<()> {
        if matches!(request.action, 0x802 | 0x80b | 0x80c)
            && self.motion.loops == -1
            && input.held & ATTACK != 0
        {
            self.inventory.refund();
            cue(
                self,
                request,
                input,
                random,
                ItemCue::Sound {
                    id: 6,
                    positional: true,
                },
            )?;
            cue(self, request, input, random, ItemCue::HeldModel(None))?;
            if input.camera_actor && input.camera == 20 {
                input.camera = input.previous_camera;
            }
            self.motion.loops = 1;
        }
        if self.motion.loops > 0 {
            cue(self, request, input, random, ItemCue::Recover)?;
        }
        Ok(())
    }

    fn hold_item_aim(&mut self, input: &mut ItemControlInput, resources: &Resources) -> Result<()> {
        self.update_velocity(Velocity::Stop, resources)?;
        if input.camera_actor {
            input.previous_camera = input.camera;
            input.camera = 20;
        }
        Ok(())
    }

    pub(crate) fn orient_grapple(&mut self, aim: Position, math: &Math) -> Result<()> {
        self.rotation.y = direction(math, aim.x, aim.z, 0)?;
        self.motion.pose.rotation[0] = [
            math.atan2(
                math.length(Position::new(aim.x, 0, aim.z)),
                aim.y.wrapping_neg(),
            )?,
            0,
            0,
        ];
        self.motion.pose.translation[0] = Position::default();
        Ok(())
    }

    fn turn_to_aim(&mut self, aim: Position, math: &Math) -> Result<()> {
        let angle = direction(math, aim.x, aim.z, self.rotation.y)?;
        if angle < 1501 {
            self.rotation.y = self.rotation.y.wrapping_add(angle);
        }
        Ok(())
    }

    fn forward_direction(&self, math: &Math) -> Position {
        let [sin, cos] = math.pair(self.rotation.y);
        Position::new(-sin, 0, -cos)
    }
}
