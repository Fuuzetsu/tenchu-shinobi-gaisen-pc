use super::Request;
use crate::{
    Result,
    math::{Position, Vector3},
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        ai::direction,
        physics::{Depth, Terrain},
        rig::Grip,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{Edition, registry::StealthEffect};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StealthPlayback {
    pub hero: usize,
    pub style: usize,
    pub next: usize,
}

pub struct DeathPlayer {
    pub position: Position,
    pub yaw: i16,
    pub velocity: Vector3<i16>,
    pub turn: i16,
    pub action: i16,
    pub target: Option<usize>,
}

pub struct DeathInput {
    pub actor: usize,
    pub is_player: bool,
    pub player: DeathPlayer,
    pub tick: i32,
    pub map: i16,
    pub stealth: Option<StealthPlayback>,
    pub focus: Option<usize>,
    pub carry_point: Position,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCue {
    CarryPoint,
    BodyDisabled(bool),
    Voice { player: bool, id: u8 },
    Sound(u8),
    Rumble { kind: u8, strength: u8 },
    Blood { joint: usize, angle: Option<i16> },
    Bubbles,
    WaterRipple { position: Position, size: i16 },
    Splash(u8),
    PlayerGrip(Grip),
    HideHeldItem,
    RemoveWeapons,
    GroundPool,
    RemoveJoint(usize),
    BodyCollision,
    ResizeBody,
    FinishContacts,
    AlignToFloor,
}

impl Actor {
    pub fn update_dead(
        &mut self,
        request: &mut Request,
        input: &mut DeathInput,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        random: &mut Random,
        mut cue: impl FnMut(
            &mut Actor,
            &mut Request,
            &mut DeathInput,
            &mut Random,
            DeathCue,
        ) -> Result<()>,
    ) -> Result<()> {
        if request.action == 0xb09 {
            cue(self, request, input, random, DeathCue::CarryPoint)?;
            let step = resources
                .math
                .velocity(input.player.yaw, -(self.width / 2), 0);
            input.carry_point.x = input.carry_point.x.wrapping_add(i32::from(step[0]));
            input.carry_point.z = input.carry_point.z.wrapping_add(i32::from(step[2]));
            if input.player.velocity.x == 0 && input.player.velocity.z == 0 {
                if self.motion.loops >= 0 {
                    self.motion.loops = -1;
                }
            } else {
                let angle = direction(
                    &resources.math,
                    input.carry_point.x.wrapping_sub(self.position.x),
                    input.carry_point.z.wrapping_sub(self.position.z),
                    self.rotation.y,
                )?;
                if i32::from(input.player.turn) < i32::from(angle).abs() {
                    self.rotation.y = self.rotation.y.wrapping_add(if angle < 1 {
                        input.player.turn.wrapping_neg()
                    } else {
                        input.player.turn
                    });
                }
                self.position.x = input.carry_point.x;
                self.position.z = input.carry_point.z;
                if self.motion.loops < 0 {
                    self.motion.loops = 0;
                }
            }
            if input.player.action == 0x120
                && input.player.target == Some(input.actor)
                && input
                    .player
                    .position
                    .y
                    .wrapping_sub(self.position.y)
                    .wrapping_abs()
                    < 501
                && self.terrain.blocked == 0
            {
                return Ok(());
            }
            self.rotation.y = ((i32::from(self.rotation.y) + 2048) % 4096) as i16;
            request.action = 0xa25;
            request.velocity = Velocity::Motion;
            self.change_motion(request.action, request.velocity, resources)?;
            self.motion.frame = -10;
            self.update_velocity(Velocity::Stop, resources)?;
            cue(self, request, input, random, DeathCue::AlignToFloor)?;
            self.motion.pose.rotation[0] = resources
                .clips
                .get(&self.motion.clip.ok_or("No Tenchu 2 corpse motion")?)
                .and_then(|c| c.bones.first())
                .and_then(|keys| keys.first())
                .ok_or("Missing Tenchu 2 corpse pose")?
                .value;
            cue(self, request, input, random, DeathCue::BodyDisabled(false))?;
            self.flags &= !0x80;
            return Ok(());
        }
        if self.motion.loops == i16::MIN {
            if matches!(request.action, 0xb08 | 0xb0a) {
                if terrain.water.depth == Some(Depth::Submerged) {
                    self.position.y = self.position.y.wrapping_sub(10);
                    self.velocity.y = 0;
                } else if input.tick % 10 == 0 && random.next15() % 3 == 0 {
                    cue(self, request, input, random, DeathCue::Splash(3))?;
                }
            } else if input.player.action == 0x120 && input.player.target == Some(input.actor) {
                request.action = 0xb09;
                request.velocity = Velocity::Stop;
                self.rotation.x = 0;
                self.rotation.y = input.player.yaw;
                self.change_motion(request.action, request.velocity, resources)?;
                self.motion.frame = 0;
                cue(self, request, input, random, DeathCue::BodyDisabled(true))?;
            }
            return Ok(());
        }
        match self.motion.action {
            0xb06
                if self.motion.frame
                    == if resources.registry.edition == Edition::Europe {
                        56
                    } else {
                        68
                    } =>
            {
                cue(
                    self,
                    request,
                    input,
                    random,
                    DeathCue::Voice {
                        player: false,
                        id: 24,
                    },
                )?;
            }
            0xb06 => {}
            0xb07 if self.motion.frame == 1 => {
                cue(
                    self,
                    request,
                    input,
                    random,
                    DeathCue::Blood {
                        joint: 2,
                        angle: Some(self.rotation.y),
                    },
                )?;
            }
            0xb07 => {}
            0xb08 | 0xb0a => {
                if self.motion.action == 0xb08 {
                    let border = self.swim_border(terrain, 0, false)?;
                    self.escape_water(border, 500, terrain.rules);
                }
                self.velocity.y = 0;
                let turn = self.turn / 2;
                if turn < self.rotation.x {
                    self.rotation.x = self.rotation.x.wrapping_sub(turn);
                } else if self.rotation.x < turn.wrapping_neg() {
                    self.rotation.x = self.rotation.x.wrapping_add(turn);
                }
                if terrain.water.depth == Some(Depth::Submerged) {
                    self.position.y = self.position.y.wrapping_sub(50);
                    if self.motion.frame % 5 == 0 && random.next15() & 1 != 0 {
                        cue(
                            self,
                            request,
                            input,
                            random,
                            DeathCue::Voice {
                                player: false,
                                id: 27,
                            },
                        )?;
                        cue(self, request, input, random, DeathCue::Bubbles)?;
                    }
                } else if self.motion.frame % 3 == 0 && random.next15() & 1 != 0 {
                    let position = Position::new(
                        self.position.x.wrapping_add(random.next15() % 1000 - 500),
                        terrain.water.surface,
                        self.position.z.wrapping_add(random.next15() % 1000 - 500),
                    );
                    cue(
                        self,
                        request,
                        input,
                        random,
                        DeathCue::WaterRipple {
                            position,
                            size: 512,
                        },
                    )?;
                    if random.next15() & 1 != 0 {
                        cue(self, request, input, random, DeathCue::Splash(2))?;
                    }
                }
            }
            0xb10..=0xb16 => {
                if input.player.target.is_none()
                    || self.position.y != input.player.position.y && self.motion.action != 0xb14
                    || self.flags & 0x2f000 != 0
                {
                    self.target = None;
                    request.action = 0xb01 + (random.next15() % 5) as i16;
                    request.velocity = Velocity::Motion;
                    self.motion.mask = u32::MAX;
                    let id = (random.next15() & 1) as u8 + 13;
                    cue(
                        self,
                        request,
                        input,
                        random,
                        DeathCue::Voice { player: false, id },
                    )?;
                    cue(
                        self,
                        request,
                        input,
                        random,
                        DeathCue::Rumble {
                            kind: 4,
                            strength: 2,
                        },
                    )?;
                    return Ok(());
                }
                if self.motion.loops > 0 {
                    cue(self, request, input, random, DeathCue::BodyCollision)?;
                    self.update_velocity(Velocity::Stop, resources)?;
                    input.focus = None;
                    self.motion.loops = i16::MIN;
                    self.motion.mask = u32::MAX;
                    return Ok(());
                }
                let playback = input
                    .stealth
                    .as_mut()
                    .ok_or("Missing Tenchu 2 stealth playback")?;
                let events = resources
                    .registry
                    .stealth_events(playback.hero, playback.style)
                    .ok_or("Unknown Tenchu 2 stealth sequence")?;
                if let Some(event) = events
                    .get(playback.next)
                    .filter(|event| event.frame == self.motion.frame)
                {
                    match event.effect {
                        StealthEffect::VictimVoice(id) => cue(
                            self,
                            request,
                            input,
                            random,
                            DeathCue::Voice { player: false, id },
                        )?,
                        StealthEffect::Blood { joint, turn } => {
                            cue(
                                self,
                                request,
                                input,
                                random,
                                DeathCue::Voice {
                                    player: false,
                                    id: 30,
                                },
                            )?;
                            cue(
                                self,
                                request,
                                input,
                                random,
                                DeathCue::Blood {
                                    joint,
                                    angle: Some(self.rotation.y.wrapping_add(turn)),
                                },
                            )?;
                        }
                        StealthEffect::AttackerCue(id) if id < 12 => cue(
                            self,
                            request,
                            input,
                            random,
                            DeathCue::Voice { player: true, id },
                        )?,
                        StealthEffect::AttackerCue(id) => {
                            cue(self, request, input, random, DeathCue::Sound(id))?;
                            let rumble = match id {
                                12 | 14 => Some((4, 1)),
                                13 => Some((3, 2)),
                                15 => Some((2, 3)),
                                _ => None,
                            };
                            if let Some((kind, strength)) = rumble {
                                cue(
                                    self,
                                    request,
                                    input,
                                    random,
                                    DeathCue::Rumble { kind, strength },
                                )?;
                            }
                        }
                        StealthEffect::HoldWeapon => cue(
                            self,
                            request,
                            input,
                            random,
                            DeathCue::PlayerGrip(Grip::Hold),
                        )?,
                        StealthEffect::ReleaseWeapon => cue(
                            self,
                            request,
                            input,
                            random,
                            DeathCue::PlayerGrip(Grip::Release),
                        )?,
                        StealthEffect::FreezeRoot => {
                            self.update_velocity(Velocity::Stop, resources)?;
                            self.motion.mask = u32::MAX - 1;
                        }
                        StealthEffect::Finish => self.motion.loops = 1,
                        StealthEffect::FocusVictim => input.focus = Some(input.actor),
                    }
                    input
                        .stealth
                        .as_mut()
                        .ok_or("Tenchu 2 stealth callback removed playback")?
                        .next += 1;
                }
            }
            _ if input.map != 12 => {
                if self.motion.frame == 1
                    && !matches!(self.kind as u8 & 0xf0, 0 | 0x10 | 0x60 | 0x70 | 0xe0 | 0xf0)
                {
                    random.next15();
                }
                if input.map != 0 && self.motion.frame % 5 == 0 && random.next15() % 5 == 0 {
                    cue(
                        self,
                        request,
                        input,
                        random,
                        DeathCue::Blood {
                            joint: 0,
                            angle: None,
                        },
                    )?;
                }
            }
            _ => {}
        }
        if (0xb10..=0xb16).contains(&self.motion.action) {
            return Ok(());
        }
        if self.motion.frame == 1 {
            if input.is_player {
                cue(self, request, input, random, DeathCue::HideHeldItem)?;
            }
            cue(self, request, input, random, DeathCue::RemoveWeapons)?;
        } else if self.motion.loops > 0 {
            if (0xb01..0xb06).contains(&request.action) && !matches!(input.map, 0 | 12) {
                cue(self, request, input, random, DeathCue::GroundPool)?;
            }
            cue(self, request, input, random, DeathCue::RemoveJoint(1))?;
            cue(self, request, input, random, DeathCue::BodyCollision)?;
            self.height /= 8;
            self.width /= 2;
            cue(self, request, input, random, DeathCue::ResizeBody)?;
            cue(self, request, input, random, DeathCue::FinishContacts)?;
            self.update_velocity(Velocity::Stop, resources)?;
            cue(self, request, input, random, DeathCue::AlignToFloor)?;
            input.focus = None;
            self.motion.loops = i16::MIN;
        }
        Ok(())
    }
}
