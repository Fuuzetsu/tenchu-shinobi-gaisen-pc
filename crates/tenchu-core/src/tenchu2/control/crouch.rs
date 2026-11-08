use super::Request;
use crate::{
    Result,
    scene::Scene,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        ai::direction,
        input::{ATTACK, CROUCH, DOWN, GUARD, ITEM, JUMP, LEFT, RIGHT, UP},
        physics::{Terrain, UNEVEN},
        rig::Rig,
        terrain::{Query, level},
    },
};
use tenchu_assets::tenchu2::Edition;

pub struct CrouchInput<'a> {
    pub held: u16,
    pub body: Option<&'a mut Actor>,
    pub unlocked: &'a [bool; 20],
    pub wall_allowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrouchCue {
    Root,
    Recover,
    Attack,
    Carry,
    Item,
    Voice(u8),
    Sound(u8),
    Splash(u8),
    Pickup(usize),
}

impl Actor {
    pub fn update_crouched(
        &mut self,
        request: &mut Request,
        input: &mut CrouchInput<'_>,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        mut cue: impl FnMut(&mut Actor, &mut Request, u16, CrouchCue) -> Result<()>,
    ) -> Result<()> {
        let clip = resources
            .clips
            .get(&self.motion.clip.ok_or("No Tenchu 2 crouch motion")?)
            .ok_or("Missing Tenchu 2 crouch motion")?;
        let mut next = None;
        let held = input.held;
        match self.motion.action {
            0x200 => {
                if self.motion.frame == 1i16.wrapping_sub(clip.blend)
                    && let Some(body) = input.body.as_deref_mut().filter(|body| {
                        body.health == 0
                            && body.motion.loops == i16::MIN
                            && !body.inventory.searched
                    })
                {
                    let delta = body.position.wrapping_sub(self.position);
                    if i32::from(direction(
                        &resources.math,
                        delta.x,
                        delta.z,
                        self.rotation.y,
                    )?)
                    .abs()
                        < 513
                    {
                        let mut found = false;
                        for (item, count) in body.inventory.counts.iter_mut().enumerate() {
                            if *count == 0
                                || !(input.unlocked[item]
                                    || (resources.registry.edition == Edition::Japan && item == 15))
                            {
                                continue;
                            }
                            *count >>= 1;
                            if *count != 0 {
                                cue(self, request, held, CrouchCue::Pickup(item))?;
                                found = true;
                            }
                            self.inventory.counts[item] =
                                self.inventory.counts[item].wrapping_add(*count);
                            *count = 0;
                        }
                        body.inventory.searched = true;
                        cue(
                            self,
                            request,
                            held,
                            CrouchCue::Sound(if found { 7 } else { 6 }),
                        )?;
                        if found {
                            request.action = 0x801;
                            request.velocity = Velocity::Motion;
                        }
                        return Ok(());
                    }
                } else {
                    if let Some(action) = self.pad.consume(&resources.commands.crouched) {
                        request.action = action;
                        request.velocity = Velocity::Motion;
                        return Ok(());
                    }
                    next = if held & RIGHT != 0 {
                        Some(0x203)
                    } else if held & LEFT != 0 {
                        Some(0x204)
                    } else if held & !self.pad.previous & JUMP != 0 {
                        Some(0x220)
                    } else {
                        None
                    };
                }
            }
            action @ (0x201 | 0x202) => {
                if held & if action == 0x201 { UP } else { DOWN } == 0 {
                    next = Some(0x200);
                } else if held & (LEFT | RIGHT) != 0 {
                    let turn = self.turn / 2;
                    self.rotation.y = self.rotation.y.wrapping_add(if held & RIGHT != 0 {
                        turn
                    } else {
                        turn.wrapping_neg()
                    });
                    self.update_velocity(Velocity::Motion, resources)?;
                }
            }
            action @ (0x203 | 0x204) => {
                if held & if action == 0x203 { RIGHT } else { LEFT } == 0 {
                    next = Some(0x200);
                }
            }
            0x210..=0x213 | 0x220 => {
                if self.motion.action == 0x220 && self.motion.loops > 0 {
                    cue(self, request, held, CrouchCue::Root)?;
                    self.rotation.y = self.rotation.y.wrapping_add(2048);
                    self.motion.pose.rotation[0][1] = clip
                        .bones
                        .first()
                        .and_then(|keys| keys.first())
                        .ok_or("Missing Tenchu 2 turnaround root")?
                        .value[1];
                }
                if self.flags & UNEVEN != 0
                    && (self.velocity.x != 0 || self.velocity.z != 0)
                    && self.terrain.height < i32::from(self.height)
                {
                    let mut point = self.position;
                    point.y = point.y.wrapping_sub(i32::from(self.height));
                    if level(terrain.map, terrain.cursor, point, Query::Base)?.is_some() {
                        let surface = terrain
                            .cursor
                            .current
                            .ok_or("Missing Tenchu 2 roll surface")?;
                        let node = &terrain.map.cells[surface.cell].nodes[surface.node];
                        let [x1, z1, x2, z2] = node.bounds.map(i32::from);
                        let span = match node.attributes & 0xc000 {
                            0x8000 => Some(x2 - x1),
                            0xc000 => Some(z2 - z1),
                            _ => None,
                        };
                        if let Some(span) = span {
                            let gradient = i32::from(node.dy)
                                .checked_div(span)
                                .ok_or("Empty Tenchu 2 roll surface")?;
                            if gradient.abs() > 1 {
                                self.update_velocity(Velocity::Stop, resources)?;
                            }
                        }
                    }
                }
                if self.motion.loops > 0 {
                    if self.weapon_drawn() && held & ATTACK != 0 {
                        cue(self, request, held, CrouchCue::Attack)?;
                    } else {
                        next = Some(0x200);
                    }
                }
            }
            _ => {}
        }
        if let Some(action) = next {
            request.action = action;
            request.velocity = Velocity::Motion;
        }
        if self.motion.frame == 1 && self.motion.action != 0x200 {
            cue(
                self,
                request,
                held,
                if terrain.water.depth.is_none() || self.terrain.height > i32::from(self.height) {
                    CrouchCue::Voice(19)
                } else {
                    CrouchCue::Splash(2)
                },
            )?;
        }
        if self.motion.action > 0x20f {
            return Ok(());
        }
        let [x, _, z] =
            resources
                .math
                .velocity(self.rotation.y, clip.forward_speed, clip.side_speed);
        let mut ahead = self.position;
        ahead.x = ahead.x.wrapping_add(i32::from(x) * 16);
        ahead.y = ahead.y.wrapping_sub(100);
        ahead.z = ahead.z.wrapping_add(i32::from(z) * 16);
        let mut floor =
            level(terrain.map, terrain.cursor, ahead, Query::Surface)?.unwrap_or(i32::MIN);
        if self.terrain.material & 0xf00 == 0
            && terrain.cursor.current.is_some_and(|surface| {
                terrain.map.cells[surface.cell].nodes[surface.node].attributes & 0xf00 != 0
            })
        {
            floor = self.position.y.wrapping_add(i32::from(self.width) + 1);
        }
        if self.position.y.wrapping_add(i32::from(self.width)) < floor && held & GUARD == 0 {
            self.update_velocity(Velocity::Stop, resources)?;
            input.held &= !(UP | DOWN);
        }
        let held = input.held;
        if held & CROUCH == 0 {
            cue(self, request, held, CrouchCue::Recover)?;
        } else if held & !self.pad.previous & ATTACK != 0 {
            cue(self, request, held, CrouchCue::Attack)?;
        } else if held & !self.pad.previous & GUARD != 0 {
            cue(self, request, held, CrouchCue::Carry)?;
        } else if held & GUARD != 0 && held & (UP | DOWN | RIGHT | LEFT) != 0 {
            request.action = if held & UP != 0 {
                0x210
            } else if held & DOWN != 0 {
                0x211
            } else if held & RIGHT != 0 {
                0x212
            } else {
                0x213
            };
            request.velocity = Velocity::Motion;
        } else {
            if self.enter_wall(request, held, input.wall_allowed, terrain, resources)? {
                return Ok(());
            }
            if held & ITEM != 0 {
                cue(self, request, held, CrouchCue::Item)?;
            } else if held & (UP | DOWN) != 0 {
                request.action = if held & UP != 0 { 0x201 } else { 0x202 };
                request.velocity = Velocity::Motion;
            }
        }
        if self.motion.frame == 0 && terrain.water.depth.is_some() {
            cue(self, request, held, CrouchCue::Splash(3))?;
        }
        Ok(())
    }

    pub fn enter_wall(
        &self,
        request: &mut Request,
        held: u16,
        allowed: bool,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
    ) -> Result<bool> {
        if self.kind >= 3 || !allowed || self.terrain.material & 0xf00 != 0 {
            return Ok(false);
        }
        let mut point = self.position;
        point.y = point.y.wrapping_sub(i32::from(self.height));
        let width = self.width.wrapping_add(100);
        let blocked = terrain.sample(point, width)?;
        if (self.terrain.material | terrain.scratch.material) & 0x8000 != 0 {
            return Ok(false);
        }
        let angle = terrain
            .rules
            .wall_angles
            .get(usize::from(blocked))
            .ok_or("Invalid Tenchu 2 wall direction")?;
        let Some(angle) = *angle else {
            return Ok(false);
        };
        if self.state != State::Wall && angle & 0x200 != 0 {
            return Ok(false);
        }
        let [x, _, z] = resources.math.velocity(angle, width, 0);
        point.x = point.x.wrapping_sub(i32::from(x));
        point.z = point.z.wrapping_sub(i32::from(z));
        if level(terrain.map, terrain.cursor, point, Query::Base)?.is_some() {
            return Ok(false);
        }
        if self.state != State::Wall && held & CROUCH != 0 {
            request.action = 0x300;
            request.velocity = Velocity::Motion;
        }
        Ok(true)
    }

    pub fn can_carry(&self, body: &Actor, rig: &Rig, scene: &Scene, player: bool) -> bool {
        player
            && body.kind & 0xf0 < 0x60
            && body.terrain.blocked == 0
            && body.terrain.lower == 0
            && body.terrain.higher == 0
            && body.terrain.material & 0xf00 == 0
            && (!self.weapon_drawn() || self.kind > 1)
            && rig.joints.iter().skip(1).all(|&joint| {
                scene
                    .models
                    .get(joint)
                    .is_some_and(|model| model.parent.is_some())
            })
            && body.health == 0
            && body.motion.loops == i16::MIN
            && body.state == State::Dead
    }
}
