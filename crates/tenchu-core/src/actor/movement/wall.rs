use super::*;
use crate::actor::State as ActorState;
use crate::camera::Mode;
use crate::items::Kind;

fn direction(held: u16) -> Option<i16> {
    (0..4).find(|d| held & (UP << d) != 0)
}

fn slide(waist: i16, direction: i16) -> Option<i16> {
    if waist == (direction + 2) & 3 {
        None
    } else if waist == (direction + 1) & 3 {
        Some(MOT_STICKON_SLIDE_L)
    } else {
        Some(MOT_STICKON_SLIDE_R)
    }
}

impl Actor {
    pub(super) fn wall_check(&self, ctx: &mut Context) -> Result<Option<i16>> {
        if self.kind as u16 >= 2 || self.map.attribute & 0xc000 != 0 {
            return Ok(None);
        }
        let probe = terrain::vector(
            &ctx.mission.terrain,
            ctx.cursor,
            self.position,
            i32::from(ctx.mission.characters[self.definition].width) + 100,
            terrain::STEP_DOWN | terrain::ALLOW_DEEP,
        )?;
        let angle = ctx.body.reflect_angle[probe.vector as usize];
        Ok((probe.attribute & 0xc000 == 0
            && angle != -1
            && (self.status == ActorState::Wall || angle & 512 == 0))
            .then_some(angle))
    }

    fn wall_motion(&mut self, ctx: &Context, action: i16) {
        self.motion.change(
            action,
            &ctx.mission.characters[self.definition].motions,
            &ctx.mission.common_motions,
            &ctx.mission.clips,
        );
    }

    pub(super) fn wall(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        mut held: u16,
        command: u16,
    ) -> Result<Option<(i16, bool)>> {
        let mut pending = None;
        match self.motion.action {
            MOT_STICKON => {
                if self.motion.count < 0 {
                    let Some(wall) = self.wall_check(ctx)? else {
                        self.motion.mask = 0x7fff;
                        return Ok(Some((MOT_SQUAT, true)));
                    };
                    let yaw = i32::from(self.rotation[1]);
                    let target = (yaw & 0xc00) + if yaw & 512 != 0 { 1024 } else { 0 };
                    let mut relative = i32::from(wall.wrapping_sub(target as i16));
                    if relative == 0 {
                        self.rotation[1] = self.rotation[1].wrapping_add(2048);
                    }
                    if relative.abs() > 2048 {
                        relative += if relative > 0 { -4096 } else { 4096 };
                    }
                    self.rotation[1] = (i32::from(self.rotation[1])
                        + (target - i32::from(self.rotation[1])) / -i32::from(self.motion.count))
                        as i16;
                    let relative = relative as i16;
                    let velocity = ctx.math.velocity(relative, -300, 0);
                    let clip =
                        &mut ctx.mission.clips[self.motion.clip.ok_or("Missing wall motion")?];
                    clip.bones[0][0].value[1] = relative;
                    clip.bones[2][0].value[1] = if relative & 1024 != 0 {
                        relative.wrapping_neg()
                    } else {
                        0
                    };
                    clip.root[0].value[0] = velocity[0];
                    clip.root[0].value[2] = velocity[2];
                } else if self.motion.loops > 0 {
                    self.motion.loops = -1;
                }
                if let Some(action) = crouch_roll(command) {
                    self.motion.mask = 0x7fff;
                    self.enter_unless_scripted(ctx, action, true);
                    self.motion.count = -5;
                } else if self.motion.loops == -1 {
                    let quadrant = (self.motion.pose.rotation[0][1] >> 10) & 3;
                    if let Some(direction) = direction(held) {
                        if let Some(action) = slide(quadrant, direction) {
                            self.wall_motion(ctx, action);
                            self.status = ActorState::Wall;
                            self.velocity[0] = 0;
                            self.velocity[2] = 0;
                            self.motion.mask = !1;
                            self.motion.pose.rotation[0][0] = -105;
                            let waist = &mut ctx.scene.models[self.parts[0]];
                            waist.local.rotation =
                                ctx.math.rotate_yxz(self.motion.pose.rotation[0]);
                            waist.flags = 0;
                        }
                    } else if self.pad.trigger & TRIANGLE != 0 {
                        world.items.wall_item = world.selected_item;
                        let action = match (ctx.camera.mode, quadrant) {
                            (Mode::StickLeft, 2) | (Mode::PeepLeft, 3) => Some(MOT_STICKON_THROW_L),
                            (Mode::StickRight, 2) | (Mode::PeepRight, 1) => {
                                Some(MOT_STICKON_THROW_R)
                            }
                            _ => None,
                        };
                        if let Some(action) = action.filter(|_| {
                            matches!(
                                world.selected_item,
                                Some(
                                    Kind::Caltrops
                                        | Kind::Grenade
                                        | Kind::SmokeBomb
                                        | Kind::PoisonedFood
                                )
                            )
                        }) {
                            self.motion.mask = !1;
                            pending = Some((action, true));
                        } else {
                            ctx.effects.sounds.push((self.position, 0x0c));
                        }
                    }
                }
            }
            MOT_STICKON_SLIDE_L | MOT_STICKON_SLIDE_R => {
                if let Some(action) = crouch_roll(command) {
                    self.motion.mask = 0x7fff;
                    self.enter_unless_scripted(ctx, action, true);
                    self.motion.count = -5;
                } else if self.motion.count >= 0 {
                    if let Some(direction) = direction(held) {
                        let waist = self.motion.pose.rotation[0][1];
                        let quadrant = (waist >> 10) & 3;
                        if let Some(action) = slide(quadrant, direction) {
                            if self.motion.action != action {
                                self.wall_motion(ctx, action);
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
                            self.move_at(ctx.math, forward, side);
                            let expected = waist.wrapping_add(self.rotation[1]) & 4095;
                            for i in [0, 2] {
                                self.position[i] =
                                    self.position[i].wrapping_add(i32::from(self.velocity[i]));
                            }
                            // Retail dereferences RAM + 0x0c here; its captured vector byte is zero.
                            let wall = self.wall_check(ctx)?.unwrap_or(ctx.body.reflect_angle[0]);
                            if expected != wall {
                                if quadrant == direction {
                                    held = 0;
                                } else {
                                    for i in [0, 2] {
                                        self.position[i] = self.position[i]
                                            .wrapping_sub(i32::from(self.velocity[i]));
                                    }
                                    self.wall_motion(ctx, MOT_STICKON);
                                    self.motion.loops = -1;
                                    self.motion.mask = 0x7fff;
                                }
                            }
                            self.velocity[0] = 0;
                            self.velocity[2] = 0;
                            if self.motion.count == 1 {
                                ctx.effects.sounds.push((self.position, 0x11));
                            }
                        }
                    } else {
                        pending = Some((MOT_STICKON, true));
                        self.motion.mask = 0x7fff;
                    }
                }
            }
            MOT_STICKON_THROW_L | MOT_STICKON_THROW_R => {
                if self.motion.count != 0 || self.motion.loops == 0 {
                    return Ok(None);
                }
                let right = self.motion.action == MOT_STICKON_THROW_R;
                let base = self.motion.pose.rotation[0][1].wrapping_add(self.rotation[1]);
                let mut angle = base.wrapping_add(if right { -1024 } else { 1024 }) & 0xf00;
                let kind = world.items.wall_item.ok_or("Missing latched wall item")?;
                self.inventory[kind.index()] = self.inventory[kind.index()].wrapping_sub(1);
                let mut start = ctx
                    .scene
                    .world(self.parts[13 + usize::from(right)])?
                    .translation;
                let [sin, cos] = ctx.math.pair(angle);
                start[0] = start[0].wrapping_sub((sin * 500) >> 12);
                start[2] = start[2].wrapping_sub((cos * 500) >> 12);
                angle = angle.wrapping_add(if right { -512 } else { 512 });
                world
                    .items
                    .wall_throw(kind, self, start, angle, world.roster, ctx)?;
                self.motion.mask = 0x7fff;
                return Ok(Some((MOT_STICKON, true)));
            }
            _ => {}
        }
        if held & CIRCLE == 0 {
            self.motion.mask = 0x7fff;
            pending = Some((self.return_normal(ctx, world.player)?, true));
        }
        Ok(pending)
    }
}
