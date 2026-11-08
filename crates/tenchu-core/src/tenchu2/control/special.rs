use super::Request;
use crate::{
    Result,
    math::{Math, Position},
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        input::{ATTACK, CROUCH, JUMP},
        physics::{Depth, LANDED, Terrain, UNEVEN},
        rig::WeaponMode,
        terrain::{self, Query},
    },
};
use tenchu_assets::tenchu2::collision::Rules;

pub struct SpecialInput {
    pub held: u16,
    pub player: bool,
    pub player_dead: bool,
    pub difficulty: i16,
    pub water: Option<Depth>,
    pub camera_actor: bool,
    pub camera: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialCue {
    Weapon(WeaponMode),
    Voice(u8),
    Attack,
    Recover,
    AlignToFloor,
    ShowHealth,
    Landing,
    Dust,
    Splash,
    Throw {
        bone: usize,
        center: Position,
        direction: Position,
    },
}

impl Actor {
    pub fn update_special(
        &mut self,
        request: &mut Request,
        input: &mut SpecialInput,
        rules: &Rules,
        resources: &Resources,
        random: &mut Random,
        mut cue: impl FnMut(&mut Actor, &mut Request, &mut Random, SpecialCue) -> Result<()>,
    ) -> Result<()> {
        match self.motion.action {
            0x700 | 0x701 => {
                let drawing = self.motion.action == 0x700;
                if drawing != self.weapon_drawn() {
                    let id = self.motion.clip.ok_or("No Tenchu 2 weapon motion")?;
                    let clip = resources
                        .clips
                        .get(&id)
                        .ok_or("Missing Tenchu 2 weapon motion")?;
                    if self.motion.frame == resources.registry.weapon_frame(id, clip.duration) {
                        cue(
                            self,
                            request,
                            random,
                            SpecialCue::Weapon(if drawing {
                                WeaponMode::Draw
                            } else {
                                WeaponMode::Sheathe
                            }),
                        )?;
                        let kind = resources
                            .registry
                            .actor(self.kind)
                            .ok_or("Unknown Tenchu 2 actor")?
                            .weapon;
                        if resources
                            .weapons
                            .parts(kind)
                            .next()
                            .is_some_and(|part| part.model.is_some())
                        {
                            cue(self, request, random, SpecialCue::Voice(u8::from(!drawing)))?;
                        }
                    }
                }
                if self.motion.loops != 0 {
                    request.action = if drawing { 0x100 } else { 0 };
                    request.velocity = Velocity::Motion;
                }
            }
            0x702 => {
                self.velocity.x = self.velocity.x.wrapping_sub(self.velocity.x >> 3);
                self.velocity.z = self.velocity.z.wrapping_sub(self.velocity.z >> 3);
                if self.flags & LANDED == 0 && i32::from(self.height) < self.terrain.height {
                    if input.held & ATTACK != 0 {
                        cue(self, request, random, SpecialCue::Attack)?;
                    }
                    return Ok(());
                }
                request.action = 0x703;
                request.velocity = Velocity::Keep;
                let hard = match self.kind & 0xf0 {
                    0 | 0x40 | 0x50 => self.motion.loops != 0,
                    0xe0 | 0xf0 => false,
                    _ => true,
                };
                if self.health != 0 && (!hard || self.last_vertical_velocity < 400) {
                    return Ok(());
                }
                cue(self, request, random, SpecialCue::AlignToFloor)?;
                if self.health != 0 && (self.last_vertical_velocity < 100 || input.difficulty == 3)
                {
                    return Ok(());
                }
                if input.difficulty == 1 {
                    self.health = self.health.wrapping_sub(10).max(0);
                    if !input.player {
                        cue(self, request, random, SpecialCue::ShowHealth)?;
                    }
                }
                request.action = if self.health == 0 && self.kind & 0xf0 == 0x70 {
                    0xb00
                } else if random.next15() & 1 != 0 {
                    0xa23
                } else {
                    0xa24
                };
                request.velocity = Velocity::Motion;
                self.change_motion(request.action, request.velocity, resources)?;
                self.motion.frame = -5;
            }
            0x703 => {
                if self.terrain.blocked != 0 {
                    let escape = rules.escape[usize::from(self.terrain.blocked)];
                    let distance = 100;
                    self.position.x = self
                        .position
                        .x
                        .wrapping_add(i32::from(escape[0]) * distance);
                    self.position.z = self
                        .position
                        .z
                        .wrapping_add(i32::from(escape[1]) * distance);
                    self.velocity.x = 0;
                    self.velocity.y = 0;
                }
                self.velocity.x = self.velocity.x.wrapping_sub(self.velocity.x >> 2);
                self.velocity.z = self.velocity.z.wrapping_sub(self.velocity.z >> 2);
                if self.motion.frame == 1 {
                    self.motion.mask = u32::MAX;
                    self.rotation.x = 0;
                    self.rotation.z = 0;
                    cue(self, request, random, SpecialCue::Landing)?;
                    if input.water.is_none() {
                        if self.motion.frame == 1 {
                            cue(self, request, random, SpecialCue::Dust)?;
                            cue(self, request, random, SpecialCue::Voice(23))?;
                        }
                    } else {
                        cue(self, request, random, SpecialCue::Splash)?;
                    }
                } else if self.motion.loops > 0 {
                    cue(self, request, random, SpecialCue::Recover)?;
                    if input.camera_actor && input.camera == 19 {
                        input.camera = 0;
                    }
                }
                if input.held & CROUCH != 0 && input.held & !self.pad.previous & JUMP != 0 {
                    request.action = 0x220;
                    request.velocity = Velocity::Motion;
                }
            }
            0x707..=0x709 | 0x70d => {
                if self.motion.action == 0x70d
                    && self.motion.clip == Some(0x121)
                    && self.motion.frame == 20
                {
                    let kind = resources
                        .registry
                        .actor(self.kind)
                        .ok_or("Unknown Tenchu 2 actor")?
                        .weapon;
                    let part = resources
                        .weapons
                        .parts(kind)
                        .next()
                        .ok_or("Missing Tenchu 2 taunt attachment")?;
                    let [sin, cos] = resources.math.pair(self.rotation.y);
                    cue(
                        self,
                        request,
                        random,
                        SpecialCue::Throw {
                            bone: part.bone,
                            center: Position::from(part.center.map(i32::from)),
                            direction: Position::new(-sin >> 10, 0, -cos >> 10),
                        },
                    )?;
                }
                if self.motion.frame == 1 {
                    let voice = match request.action {
                        0x707 => Some(6),
                        0x709 => Some(8),
                        0x70d => Some(if !input.player_dead {
                            12
                        } else if random.next15() & 1 != 0 {
                            10
                        } else {
                            11
                        }),
                        _ => None,
                    };
                    if let Some(voice) = voice {
                        cue(self, request, random, SpecialCue::Voice(voice))?;
                    }
                } else if self.motion.loops > 0 {
                    if request.action == 0x709 {
                        request.action = 0x700;
                        request.velocity = Velocity::Motion;
                    } else {
                        cue(self, request, random, SpecialCue::Recover)?;
                    }
                }
            }
            0x70a | 0x70f => {
                if self.motion.frame == 1 {
                    self.update_velocity(Velocity::Stop, resources)?;
                } else if self.motion.loops > 0 {
                    cue(self, request, random, SpecialCue::Recover)?;
                }
            }
            0x70c => {
                if self.motion.loops > 0 {
                    self.update_velocity(Velocity::Stop, resources)?;
                    self.motion.loops = -1;
                } else if input.held != 0 {
                    request.action = 0xa26;
                    request.velocity = Velocity::Motion;
                    self.change_motion(request.action, request.velocity, resources)?;
                    self.motion.frame = -10;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn align_to_floor(
        &mut self,
        terrain: &mut Terrain<'_>,
        math: &Math,
        mut root: impl FnMut(&Actor) -> Result<Position>,
    ) -> Result<bool> {
        if matches!(self.terrain.material & 0xf00, 0x200 | 0x300) || self.flags & UNEVEN == 0 {
            return Ok(false);
        }
        let [x, _, z] = math.velocity(self.rotation.y, 1000, 0);
        let root = root(self)?;
        let mut sample = |x: i32, z: i32| {
            terrain::level(
                terrain.map,
                terrain.cursor,
                Position::new(x, root.y.wrapping_sub(1000), z),
                Query::Surface,
            )
        };
        let front = sample(
            root.x.wrapping_add(i32::from(x)),
            root.z.wrapping_add(i32::from(z)),
        )?;
        let back = sample(
            root.x.wrapping_sub(i32::from(x)),
            root.z.wrapping_sub(i32::from(z)),
        )?;
        let (Some(front), Some(back)) = (front, back) else {
            return Ok(false);
        };
        let front = front.wrapping_sub(root.y);
        let back = back.wrapping_sub(root.y);
        if (1..=1000).contains(&front) && back <= 1500i32.wrapping_sub(front) {
            self.rotation.x = math.atan2(front, 1000)?;
        } else if (1..=1000).contains(&back) && front <= 1500i32.wrapping_sub(back) {
            self.rotation.x = math.atan2(back, 1000)?.wrapping_neg();
        }
        Ok(true)
    }
}
