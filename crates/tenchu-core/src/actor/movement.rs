mod chase;
mod crouch;
mod hang;
mod swim;
#[cfg(test)]
mod tests;
mod wall;

use super::*;
use crate::input::*;
use tenchu_assets::executable::Command;

fn crouch_roll(command: u16) -> Option<i16> {
    match command {
        0x11 => Some(MOT_SQUAT_ROLL_F),
        0x12 => Some(MOT_SQUAT_ROLL_B),
        0x13 => Some(MOT_SQUAT_ROLL_L),
        0x14 => Some(MOT_SQUAT_ROLL_R),
        _ => None,
    }
}

impl Actor {
    pub fn control(
        &mut self,
        ctx: &mut Context,
        commands: &[Command],
        center: [i32; 3],
        world: &mut ActionContext,
    ) -> Result<bool> {
        let player = self.id == world.player;
        let command = self.pad.consume(commands);
        let held = if self.pad.held & L1 != 0 {
            self.pad.held & !0xf000
        } else {
            self.pad.held
        };
        let damage_pending = if self.attribute & ATTR_HIT != 0 {
            self.damage_control(ctx, world)?
        } else {
            if self.fall_check(ctx)? {
                self.hang_check(ctx)?;
            } else if self.map.attribute & 4 != 0 {
                self.swim_check(ctx, world)?;
            }
            None
        };
        let next = match self.status {
            0 => self.normal(ctx, world, held, command, center)?,
            1 => self.action(ctx, world, held)?,
            2 => self.walk(ctx, world, held, center)?,
            3 => self.swim(ctx, world, held)?,
            4 => self.grapple_control(ctx, world)?,
            5 => self.engage(ctx, world, held, command, center)?,
            6 => self.chase(ctx, world, held, command, center)?,
            7 => self.attack(ctx, world, held)?,
            8 => self.state(ctx, world, held)?,
            9 => self.jump(ctx, world, held)?,
            10 => self.hang(ctx, held, world.player)?,
            11 => self.crouch(ctx, world, held, command)?,
            12 => self.wall(ctx, world, held, command)?,
            13 => None,
            14 => self.shuriken_control(ctx, world)?,
            15 => self.item_control(ctx, world)?,
            16 => self.damaged(ctx, world)?,
            17 => self.dead(ctx, world)?,
            status => {
                return Err(format!(
                    "Actor state {status:#x}, motion {:#x} is not implemented yet",
                    self.motion.action
                ));
            }
        };
        if let Some((action, movement)) = next.or(damage_pending.filter(|_| self.status == 7)) {
            self.enter_unless_scripted(ctx, action, movement);
        }
        ctx.scene.set_translation(self.root_model, self.position);
        let visible = if player {
            true
        } else if let Some(clip) =
            crate::visibility::model(ctx.scene, ctx.camera.view, self.root_model)?
        {
            ctx.drawing.fog = clip.fog;
            true
        } else {
            false
        };
        if visible {
            self.draw_fog = ctx.drawing.fog;
        }
        let pose = visible || self.status == 7;
        let animated = pose && self.motion.loops >= 0;
        self.motion
            .play(&ctx.mission.clips, &ctx.math.tables.hermite, pose)?;
        if animated {
            let count = ctx.mission.clips[self.motion.clip.unwrap()].bones.len();
            self.motion
                .pose
                .publish(ctx.scene, &self.parts, ctx.math, self.motion.mask, count);
        }
        self.previous = self.position;
        for i in 0..3 {
            self.position[i] = self.position[i].wrapping_add(i32::from(self.velocity[i]));
        }
        ctx.scene.update(self.root_model, self.root(ctx.math));
        Ok(visible)
    }

    fn fall_check(&mut self, ctx: &mut Context) -> Result<bool> {
        if self.motion.action == MOT_STATE_FALL {
            return Ok(true);
        }
        if self.motion.action == MOT_ATTACK_DIVE {
            return Ok(false);
        }
        if self.status == 9 && self.map.height > 0 {
            return Ok(true);
        }
        if self.attribute & ATTR_FLOAT != 0 || self.map.height <= 1000 {
            return Ok(false);
        }
        if matches!(self.status, 4 | 10 | 13 | 16 | 17)
            || (self.status == 11 && self.motion.loops == 0)
        {
            return Ok(false);
        }
        self.motion.mask = 0x7fff;
        let [x, z] = ctx.body.reflect_move[self.map.angle_high as usize];
        let width = i32::from(ctx.mission.characters[self.definition].width);
        self.position[0] = self.position[0].wrapping_add((width * i32::from(x)) >> 2);
        self.position[2] = self.position[2].wrapping_add((width * i32::from(z)) >> 2);
        self.enter_unless_scripted(ctx, MOT_STATE_FALL, false);
        if self.status == 11 {
            self.motion.count >>= 2;
        }
        self.clear_attack(ctx, 3)?;
        Ok(true)
    }

    fn normal(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        command: u16,
        center: [i32; 3],
    ) -> Result<Option<(i16, bool)>> {
        let mut pending = None;
        match self.motion.action {
            MOT_NORMAL => {
                if held & L2 != 0 && self.id != world.player {
                    return Ok(if held & DOWN != 0 {
                        Some((MOT_ACTION_GESTURE, true))
                    } else if held & RIGHT != 0 {
                        Some((MOT_ACTION_LOOP, true))
                    } else if held & LEFT != 0 {
                        Some((MOT_ACTION_NOTICE, true))
                    } else if held & UP != 0 {
                        Some((MOT_ACTION, true))
                    } else {
                        None
                    });
                }
                if held & RIGHT != 0 {
                    pending = Some((MOT_NORMAL_TURN_R, false));
                } else if held & LEFT != 0 {
                    pending = Some((MOT_NORMAL_TURN_L, false));
                } else if self.motion.count == 0 && ctx.random.next15() % 100 == 0 {
                    pending = Some((
                        if ctx.random.next15() & 1 != 0 {
                            MOT_ACTION_FIDGET_A
                        } else {
                            MOT_ACTION_FIDGET_B
                        },
                        true,
                    ));
                }
            }
            MOT_NORMAL_TURN_R | MOT_NORMAL_TURN_L => {
                if self.motion.count == 1 {
                    world.intelligence.voices.push((self.id, 0x10));
                }
                let right = self.motion.action == MOT_NORMAL_TURN_R;
                if held & if right { RIGHT } else { LEFT } != 0 {
                    let turn = ctx.mission.characters[self.definition].turn;
                    self.rotation[1] =
                        self.rotation[1].wrapping_add(if right { turn } else { -turn });
                } else {
                    pending = Some((MOT_NORMAL, true));
                }
            }
            _ => {}
        }
        if self.attribute & ATTR_WEAPON_DRAWN != 0 {
            return Ok(Some((MOT_ENGAGE_STANCE, true)));
        }
        if command != 0 {
            return Ok(match command {
                1 => Some((MOT_MOVE_DASH_FWD, true)),
                2 => Some((MOT_MOVE_DASH_BACK, true)),
                3 => Some((MOT_MOVE_DASH_LEFT, true)),
                4 => Some((MOT_MOVE_DASH_RIGHT, true)),
                _ => pending,
            });
        }
        if self.pad.trigger & CROSS != 0 {
            return Ok(self.start_jump(ctx, world, held, center));
        }
        if self.pad.trigger & TRIANGLE != 0 {
            return self.selected_item(ctx, world, pending);
        }
        if held & CIRCLE != 0 {
            return Ok(Some((MOT_SQUAT, true)));
        }
        if held & UP != 0 {
            return Ok(Some((MOT_MOVE, true)));
        }
        if held & DOWN != 0 {
            return Ok(Some((MOT_MOVE_BACK, true)));
        }
        if self.pad.trigger & SQUARE != 0 {
            return Ok(Some((MOT_STATE_DRAW, true)));
        }
        Ok(pending)
    }

    fn action(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
    ) -> Result<Option<(i16, bool)>> {
        let complete = self.motion.count == 0 && self.motion.loops != 0;
        let mut pending = None;
        match self.motion.action {
            MOT_ACTION_LOOP => {
                if self.motion.loops != 0 && held != 0 {
                    pending = Some((self.return_normal(ctx, world.player)?, true));
                }
            }
            MOT_ACTION_FIDGET_A | MOT_ACTION_FIDGET_B => {
                if self.health != self.max_health || self.attribute & 1 != 0 {
                    pending = Some((self.return_normal(ctx, world.player)?, true));
                }
                if self.motion.count == 1 {
                    world.intelligence.voices.push((self.id, 15));
                } else if complete {
                    pending = Some((self.return_normal(ctx, world.player)?, true));
                }
            }
            MOT_ACTION => {
                if self.motion.count == 1 {
                    self.clear_attack(ctx, 3)?;
                    self.recover_blade(ctx, world);
                }
                if self.motion.count == 0 && self.motion.loops > 0 {
                    self.motion.count = ctx.mission.clips[self.motion.clip.unwrap()].duration - 1;
                    self.play_pose(ctx)?;
                    self.motion.loops = -1;
                    self.velocity[0] = 0;
                    self.velocity[2] = 0;
                }
                if self.motion.loops == -1 && held != 0 {
                    self.enter_unless_scripted(ctx, MOT_DAMAGE_GETUP, true);
                    self.motion.count = -15;
                }
            }
            MOT_ACTION_NOTICE => {
                if self.motion.count == 1 {
                    world.intelligence.voices.push((self.id, 6));
                } else if complete {
                    pending = Some((MOT_STATE_DRAW, true));
                }
            }
            _ => {
                if complete {
                    pending = Some((self.return_normal(ctx, world.player)?, true));
                }
            }
        }
        Ok(pending)
    }

    fn walk(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        center: [i32; 3],
    ) -> Result<Option<(i16, bool)>> {
        let turn = ctx.mission.characters[self.definition].turn;
        let mut pending = None;
        match self.motion.action {
            MOT_MOVE => {
                let duration = ctx.mission.clips[self.motion.clip.unwrap()].duration;
                if self.motion.count == 1 || self.motion.count == duration / 2 {
                    world.intelligence.voices.push((self.id, 0x11));
                }
                if held & UP == 0 {
                    pending = Some((MOT_NORMAL, true));
                } else if self.attribute & ATTR_WALL != 0 {
                    let (y, height) = (self.position[1], self.map.height);
                    self.position[1] = y.wrapping_sub(400);
                    self.map.height = 1;
                    if !self.hang_check(ctx)? {
                        self.position[1] = y;
                        self.map.height = height;
                    }
                } else if held & (LEFT | RIGHT) != 0 {
                    self.rotation[1] =
                        self.rotation[1].wrapping_add(if held & RIGHT != 0 { turn } else { -turn });
                    self.motion_velocity(ctx);
                }
            }
            MOT_MOVE_BACK => {
                if self.motion.count == 1 {
                    world.intelligence.voices.push((self.id, 0x11));
                }
                if held & DOWN == 0 {
                    pending = Some((MOT_NORMAL, true));
                }
                // The retail backward walk takes the left branch even without a turn button.
                self.rotation[1] =
                    self.rotation[1].wrapping_add(if held & RIGHT != 0 { turn } else { -turn });
                self.motion_velocity(ctx);
            }
            MOT_MOVE_DASH_FWD..=MOT_MOVE_DASH_LEFT => {
                if self.motion.count == 1 {
                    world.intelligence.voices.push((self.id, 0x13));
                }
                if self.motion.count == 0 && self.motion.loops != 0 {
                    pending = Some((MOT_NORMAL, true));
                }
            }
            _ => {}
        }
        if self.pad.trigger & CROSS != 0 {
            return Ok(self.start_jump(ctx, world, held, center));
        }
        if self.pad.trigger & TRIANGLE != 0 {
            return self.selected_item(ctx, world, pending);
        }
        if held & CIRCLE != 0 {
            return Ok(Some((MOT_SQUAT, true)));
        }
        if self.pad.trigger & SQUARE != 0 {
            return Ok(Some((MOT_STATE_DRAW, true)));
        }
        Ok(pending)
    }

    fn start_jump(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        center: [i32; 3],
    ) -> Option<(i16, bool)> {
        ctx.effects
            .smoke_burst(self.position, 150, 12, 8, ctx.random);
        if !self.registered(ctx, MOT_JUMP) {
            return None;
        }
        if self.motion.action == MOT_CHASE_DASH_FWD {
            if self.motion.count < 11 && self.registered(ctx, MOT_JUMP_RUN) {
                self.move_at(ctx.math, 127, 0);
                if self.id == world.player {
                    world.intelligence.voices.push((self.id, 0x48));
                }
                world.intelligence.voices.push((self.id, 0x17));
                return Some((MOT_JUMP_RUN, false));
            }
            return None;
        }
        self.position[0] = center[0];
        self.position[2] = center[2];
        self.velocity[1] = 0;
        let (specific, forward, side) = if held & UP != 0 {
            (MOT_JUMP_FORWARD, 100, 0)
        } else if held & DOWN != 0 {
            (MOT_JUMP_BACK, -100, 0)
        } else if held & RIGHT != 0 {
            (MOT_JUMP_RIGHT, 0, -100)
        } else if held & LEFT != 0 {
            (MOT_JUMP_LEFT, 0, 100)
        } else {
            (MOT_JUMP, 0, 0)
        };
        self.move_at(ctx.math, forward, side);
        Some((
            if self.registered(ctx, specific) {
                specific
            } else {
                MOT_JUMP
            },
            false,
        ))
    }

    fn jump(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
    ) -> Result<Option<(i16, bool)>> {
        let mid = self.motion.action;
        if self.pad.trigger & CROSS != 0 && mid != MOT_JUMP_WALLKICK {
            let width = ctx.mission.characters[self.definition].width;
            let map = terrain::vector(
                &ctx.mission.terrain,
                ctx.cursor,
                self.position,
                i32::from(width) + 300,
                0,
            )?;
            if map.vector == 0 || !self.enter(ctx, MOT_JUMP_WALLKICK, false) {
                return Ok(None);
            }
            let yaw = ctx.body.reflect_angle[map.vector as usize];
            self.position[1] = self.position[1].wrapping_sub(500);
            if yaw == -1 {
                self.velocity[0] = self.velocity[0].wrapping_neg();
                self.velocity[2] = self.velocity[2].wrapping_neg();
            } else {
                self.rotation[1] = yaw.wrapping_add(2048);
                self.move_at(ctx.math, -100, 0);
            }
            world.intelligence.voices.push((self.id, 0x48));
            return Ok(None);
        }
        if self.attribute & (ATTR_NOFLOOR | ATTR_BUOYANT) != 0 && self.motion.count >= 2 {
            let level = terrain::level(&ctx.mission.terrain, ctx.cursor, self.position, 0)?;
            if self.position[1] < level {
                self.enter_unless_scripted(ctx, MOT_STATE_FALL, false);
                world.intelligence.voices.push((self.id, 6));
                self.motion.count >>= 2;
                return Ok(None);
            }
            if mid == MOT_JUMP_FLIP {
                self.rotation[1] = self.rotation[1].wrapping_add(self.motion.pose.rotation[0][1]);
                self.motion.pose.rotation[0][1] = 0;
                return Ok(Some((MOT_STATE_LAND_FLIP, true)));
            }
            return Ok(Some((MOT_STATE_LAND, false)));
        }
        if self.motion.count == 0 && self.motion.loops != 0 {
            self.enter_unless_scripted(ctx, MOT_STATE_FALL, false);
            if mid == MOT_JUMP_FLIP {
                self.rotation[1] = self.rotation[1].wrapping_add(2048);
                self.motion.pose.rotation[0][1] = 0;
            }
            if matches!(mid, MOT_JUMP_RUN | MOT_JUMP_FLIP) {
                self.motion.count >>= 1;
            }
            return Ok(None);
        }
        let duration = ctx.mission.clips[self.motion.clip.unwrap()].duration;
        self.velocity[1] = ((i32::from(self.motion.count) - i32::from(duration >> 1))
            * if mid == MOT_JUMP_RUN { 10 } else { 20 }) as i16;
        if held & 0xf000 != 0 && mid != MOT_JUMP_RUN {
            let (forward, side) = if held & UP != 0 {
                (10, 0)
            } else if held & DOWN != 0 {
                (-10, 0)
            } else if held & RIGHT != 0 {
                (0, -10)
            } else {
                (0, 10)
            };
            let v = ctx.math.velocity(self.rotation[1], forward, side);
            let (x, z) = (
                v[0].wrapping_add(self.velocity[0]),
                v[2].wrapping_add(self.velocity[2]),
            );
            if i32::from(x).abs() <= 100 && i32::from(z).abs() <= 100 {
                self.velocity[0] = x;
                self.velocity[2] = z;
            }
        }
        Ok((held & SQUARE != 0
            && mid != MOT_JUMP_FLIP
            && self.velocity[1] > 0
            && self.attribute & ATTR_WEAPON_DRAWN != 0)
            .then_some((MOT_ATTACK_DIVE, false)))
    }

    fn state(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
    ) -> Result<Option<(i16, bool)>> {
        let count = self.motion.count;
        match self.motion.action {
            MOT_STATE_DRAW | MOT_STATE_SHEATHE => {
                let drawing = self.motion.action == MOT_STATE_DRAW;
                if count == 1 {
                    self.clear_attack(ctx, 3)?;
                    if drawing && (2..7).contains(&self.kind) {
                        return Ok(Some((self.return_normal(ctx, world.player)?, true)));
                    }
                    if (self.attribute & ATTR_WEAPON_DRAWN != 0) == drawing {
                        return Ok(Some((
                            if drawing {
                                MOT_ENGAGE_STANCE
                            } else {
                                MOT_NORMAL
                            },
                            true,
                        )));
                    }
                } else if count == ctx.mission.clips[self.motion.clip.unwrap()].duration / 2 {
                    world
                        .intelligence
                        .voices
                        .push((self.id, if drawing { 0 } else { 1 }));
                    self.equip(ctx, drawing);
                } else if count == 0 && self.motion.loops != 0 {
                    if drawing && self.attribute & ATTR_PHASE == 0 {
                        self.attribute |= ATTR_SEARCH | 2;
                        let position = if self.id == world.player {
                            self.position
                        } else {
                            world.roster.get(world.player).position
                        };
                        self.brain.chase = [position[0], position[2]];
                        self.brain.timer = 1;
                    }
                    return Ok(Some((
                        if drawing {
                            MOT_ENGAGE_STANCE
                        } else {
                            MOT_NORMAL
                        },
                        true,
                    )));
                }
                Ok(None)
            }
            MOT_STATE_CLIMB => {
                let duration = ctx.mission.clips[self.motion.clip.unwrap()].duration;
                if count == duration / 2 || (count == 0 && self.motion.loops != 0) {
                    world.intelligence.voices.push((self.id, 0x13));
                }
                Ok(
                    (count == duration / 2 || (count == 0 && self.motion.loops != 0))
                        .then_some((MOT_CHASE, true)),
                )
            }
            MOT_STATE_FALL => {
                if count < -54 && self.velocity[1] > 200 {
                    self.motion.count = -30;
                }
                let mut pending = (self.velocity[1] > 0 && self.pad.trigger & SQUARE != 0)
                    .then_some((MOT_ATTACK_DIVE, false));
                if self.attribute & ATTR_NOFLOOR != 0 || self.map.height <= 0 {
                    pending = Some(if self.motion.count < -40 {
                        world.intelligence.voices.push((self.id, 0x19));
                        (self.return_normal(ctx, world.player)?, true)
                    } else if self.motion.count > -21 {
                        if self.kind & 0xf0 == 0x10 {
                            let action = if ctx.random.next15() & 1 != 0 {
                                MOT_DAMAGE_SLAM_BACK
                            } else {
                                MOT_DAMAGE_SLAM_FORE
                            };
                            self.health = self.health.wrapping_sub(10).max(0);
                            world.intelligence.voices.push((self.id, 8));
                            world.combat.request_life(self);
                            (action, false)
                        } else {
                            (MOT_STATE_LAND_HEAVY, false)
                        }
                    } else {
                        (MOT_STATE_LAND, false)
                    });
                    return Ok(pending);
                }
                if self.motion.count <= -46 {
                    self.velocity[0] >>= 1;
                    self.velocity[2] >>= 1;
                }
                Ok(pending)
            }
            MOT_STATE_LAND | MOT_STATE_LAND_HEAVY | MOT_STATE_LAND_FLIP => {
                let mut pending = None;
                if self.motion.action != MOT_STATE_LAND_FLIP
                    && count == 1
                    && self.id == world.player
                {
                    ctx.camera.normal(self, ctx.math)?;
                }
                if self.motion.action != MOT_STATE_LAND_FLIP
                    && count < 5
                    && held & CIRCLE != 0
                    && self.pad.trigger & CROSS != 0
                {
                    pending = Some((MOT_SQUAT_BACKFLIP, true));
                    self.rotation[1] = self.rotation[1].wrapping_add(2048);
                }
                if count == 1 {
                    let action = pending.map_or(self.motion.action, |p| p.0);
                    world
                        .intelligence
                        .voices
                        .push((self.id, if action == MOT_STATE_LAND { 0x19 } else { 0x1a }));
                    ctx.effects
                        .smoke_burst(self.position, 300, 12, 10, ctx.random);
                }
                if count == 0 && self.motion.loops != 0 {
                    if pending.map_or(self.motion.action, |p| p.0) == MOT_STATE_LAND_FLIP {
                        ctx.camera.snap_pending = true;
                    }
                    return Ok(Some((self.return_normal(ctx, world.player)?, true)));
                }
                self.velocity[0] = self.velocity[0].wrapping_sub(self.velocity[0] >> 2);
                self.velocity[2] = self.velocity[2].wrapping_sub(self.velocity[2] >> 2);
                Ok(pending)
            }
            MOT_STATE_PICKUP => Ok(if count == 0 && self.motion.loops != 0 {
                Some((self.return_normal(ctx, world.player)?, true))
            } else {
                None
            }),
            MOT_ITEM_PLANT => Ok(None),
            action => Err(format!(
                "State transition {action:#x} is not implemented yet"
            )),
        }
    }
}
