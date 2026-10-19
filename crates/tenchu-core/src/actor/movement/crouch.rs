use super::*;

impl Actor {
    pub(super) fn crouch(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        command: u16,
    ) -> Result<Option<(i16, bool)>> {
        let turn = ctx.mission.characters[self.definition].turn / 2;
        let count = self.motion.count;
        let mut pending = None;
        match self.motion.action {
            MOT_SQUAT => {
                if count == 0 && self.motion.loops != 0 {
                    self.motion.loops = -1;
                }
                if held & UP != 0 {
                    pending = Some((MOT_SQUAT_WALK_F, true));
                } else if held & DOWN != 0 {
                    pending = Some((MOT_SQUAT_WALK_B, true));
                } else if held & RIGHT != 0 {
                    pending = Some((MOT_SQUAT_WALK_R, true));
                } else if held & LEFT != 0 {
                    pending = Some((MOT_SQUAT_WALK_L, true));
                } else if self.pad.trigger & CROSS != 0 {
                    pending = Some((MOT_SQUAT_BACKFLIP, true));
                    self.rotation[1] = self.rotation[1].wrapping_add(2048);
                }
            }
            MOT_SQUAT_WALK_F => {
                if held & UP == 0 {
                    pending = Some((MOT_SQUAT, true));
                } else if self.pad.trigger & CROSS != 0 {
                    pending = Some((MOT_SQUAT_BACKFLIP, true));
                    self.rotation[1] = self.rotation[1].wrapping_add(2048);
                }
            }
            MOT_SQUAT_WALK_B | MOT_SQUAT_WALK_R | MOT_SQUAT_WALK_L => {
                let mid = self.motion.action;
                let button = match mid {
                    MOT_SQUAT_WALK_B => DOWN,
                    MOT_SQUAT_WALK_R => RIGHT,
                    _ => LEFT,
                };
                if held & button == 0 {
                    pending = Some((MOT_SQUAT, true));
                } else {
                    let turning = if mid == MOT_SQUAT_WALK_B {
                        held & (LEFT | RIGHT) != 0
                    } else {
                        held & DOWN != 0
                    };
                    if turning {
                        let right = if mid == MOT_SQUAT_WALK_B {
                            held & RIGHT != 0
                        } else {
                            mid == MOT_SQUAT_WALK_R
                        };
                        self.rotation[1] =
                            self.rotation[1].wrapping_add(if right { turn } else { -turn });
                        self.velocity[0] = 0;
                        self.velocity[2] = 0;
                    } else if self.velocity[0] == 0 && self.velocity[2] == 0 {
                        self.motion_velocity(ctx);
                    }
                }
            }
            MOT_SQUAT_BACKFLIP => {
                if count == 0 && self.motion.loops != 0 {
                    pending = Some((MOT_SQUAT, true));
                }
            }
            _ => {
                if count == 1 {
                    return Ok(None);
                }
                if count == 0 && self.motion.loops != 0 {
                    return Ok(Some((MOT_SQUAT, true)));
                }
                if self.velocity[0] == 0 && self.velocity[2] == 0 {
                    return Ok(None);
                }
                let p = [
                    self.position[0].wrapping_add(i32::from(self.velocity[0]) * 4),
                    self.position[1],
                    self.position[2].wrapping_add(i32::from(self.velocity[2]) * 4),
                ];
                if terrain::level(
                    &ctx.mission.terrain,
                    ctx.cursor,
                    p,
                    terrain::STEP_DOWN | terrain::RETURN_DELTA,
                )? < -450
                {
                    self.position[0] = self.position[0].wrapping_sub(i32::from(self.velocity[0]));
                    self.position[2] = self.position[2].wrapping_sub(i32::from(self.velocity[2]));
                    self.velocity[0] = 0;
                    self.velocity[2] = 0;
                }
                return Ok(None);
            }
        }
        let next = pending.map_or(self.motion.action, |p| p.0);
        if next == MOT_SQUAT_BACKFLIP {
            return Ok(pending);
        }
        if next != MOT_SQUAT && (self.velocity[0] != 0 || self.velocity[2] != 0) {
            let p = [
                self.position[0].wrapping_add(i32::from(self.velocity[0]) * 16),
                self.position[1],
                self.position[2].wrapping_add(i32::from(self.velocity[2]) * 16),
            ];
            if terrain::level(
                &ctx.mission.terrain,
                ctx.cursor,
                p,
                terrain::STEP_DOWN | terrain::RETURN_DELTA,
            )?
            .wrapping_abs()
                >= 500
            {
                self.velocity[0] = 0;
                self.velocity[2] = 0;
            }
        }
        if command != 0 {
            return Ok(match command {
                0x11 => Some((MOT_SQUAT_ROLL_F, true)),
                0x12 => Some((MOT_SQUAT_ROLL_B, true)),
                0x13 => Some((MOT_SQUAT_ROLL_L, true)),
                0x14 => Some((MOT_SQUAT_ROLL_R, true)),
                _ => pending,
            });
        }
        if self.pad.trigger & SQUARE != 0 {
            return self.attack_input(ctx, world, held, pending);
        }
        if self.pad.trigger & TRIANGLE != 0 {
            return self.selected_item(ctx, world, pending);
        }
        if held & CIRCLE == 0 {
            if self.id == 0 {
                ctx.camera.normal(self, ctx.math)?;
            }
            return Ok(Some((self.return_motion(), true)));
        }
        Ok(pending)
    }
}
