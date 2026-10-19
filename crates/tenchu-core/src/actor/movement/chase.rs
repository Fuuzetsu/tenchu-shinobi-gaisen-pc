use super::*;

impl Actor {
    pub(super) fn engage(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        command: u16,
        center: [i32; 3],
    ) -> Result<Option<(i16, bool)>> {
        let mut pending = None;
        match self.motion.action {
            MOT_ENGAGE_STANCE => {
                if held & RIGHT != 0 {
                    pending = Some((MOT_ENGAGE_TURN_R, false));
                } else if held & LEFT != 0 {
                    pending = Some((MOT_ENGAGE_TURN_L, false));
                } else if command == 0x22 {
                    pending = Some((MOT_ATTACK_LUNGE_BACK, true));
                } else if command == 0x31 {
                    pending = Some((MOT_JUMP_FLIP, false));
                    self.move_at(ctx.math, 120, 0);
                } else if self.motion.count == 0 && ctx.random.next15() % 20 == 0 {
                    pending = Some((MOT_ATTACK_TAUNT, true));
                }
            }
            MOT_ENGAGE_TURN_R | MOT_ENGAGE_TURN_L => {
                let right = self.motion.action == MOT_ENGAGE_TURN_R;
                let turn = ctx.mission.characters[self.definition].turn;
                self.rotation[1] = self.rotation[1].wrapping_add(if right { turn } else { -turn });
                if held & if right { RIGHT } else { LEFT } == 0 {
                    pending = Some((MOT_ENGAGE_STANCE, true));
                }
            }
            MOT_ENGAGE_SHEATHE => {
                return Ok((self.motion.count == 0 && self.motion.loops != 0)
                    .then_some((MOT_STATE_SHEATHE, true)));
            }
            MOT_ENGAGE_VARIANT_2 => {
                return Ok((self.motion.count == 0 && self.motion.loops != 0)
                    .then_some((MOT_ENGAGE_STANCE, true)));
            }
            MOT_ENGAGE => {
                for i in [0, 2] {
                    if self.velocity[i] != 0 {
                        self.velocity[i] = self.velocity[i].wrapping_add(if self.velocity[i] > 0 {
                            -4
                        } else {
                            4
                        });
                    }
                }
                self.motion.count = self.motion.count.wrapping_sub(1);
                if self.motion.count < self.motion.loops {
                    pending = Some((
                        if held & DOWN != 0 {
                            MOT_CHASE_BACK
                        } else {
                            self.return_normal(ctx, world.player)?
                        },
                        true,
                    ));
                }
                if world.clock & 3 == 0 {
                    ctx.effects
                        .smoke_burst(self.position, 300, 10, 5, ctx.random);
                }
                return Ok(pending);
            }
            _ => (),
        }
        if self.attribute & ATTR_WEAPON_DRAWN == 0 {
            return Ok(Some((MOT_NORMAL, true)));
        }
        if command != 0 {
            return Ok(match command {
                1 => Some((MOT_CHASE_DASH_FWD, true)),
                2 => Some((MOT_CHASE_DASH_BACK, true)),
                3 => Some((MOT_CHASE_DASH_LEFT, true)),
                4 => Some((MOT_CHASE_DASH_RIGHT, true)),
                0x21 => Some((MOT_ATTACK_LUNGE, true)),
                _ => pending,
            });
        }
        if self.pad.trigger & CROSS != 0 {
            return Ok(self.start_jump(ctx, held, center));
        }
        if self.pad.trigger & TRIANGLE != 0 {
            return self.selected_item(ctx, world, pending);
        }
        if held & CIRCLE != 0 {
            return Ok(Some((
                if self.pad.trigger & SQUARE != 0 {
                    MOT_ATTACK_CROUCH
                } else {
                    MOT_SQUAT
                },
                true,
            )));
        }
        if self.pad.trigger & SQUARE != 0 {
            return self.attack_input(ctx, world, held, pending);
        }
        if held & UP != 0 {
            return Ok(Some((MOT_CHASE, true)));
        }
        if held & DOWN != 0 {
            return Ok(Some((MOT_CHASE_BACK, true)));
        }
        Ok(pending)
    }

    pub(super) fn chase(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        command: u16,
        center: [i32; 3],
    ) -> Result<Option<(i16, bool)>> {
        let turn = ctx.mission.characters[self.definition].turn / 2;
        let mut pending = None;
        match self.motion.action {
            MOT_CHASE => {
                if held & UP == 0 {
                    pending = Some((MOT_ENGAGE_STANCE, true));
                } else if self.attribute & ATTR_LEDGE != 0 {
                    self.enter(ctx, MOT_STATE_CLIMB, false);
                    self.move_at(ctx.math, 35, 0);
                    if self.motion.mode & 1 != 0 {
                        self.motion.mode &= !1;
                        self.motion.count = 13;
                    } else {
                        self.motion.mode |= 1;
                    }
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
                } else if held & CIRCLE != 0 {
                    pending = Some((MOT_SQUAT, true));
                }
            }
            MOT_CHASE_BACK => {
                if held & DOWN == 0 {
                    pending = Some((MOT_ENGAGE_STANCE, true));
                } else if command == 0x22 {
                    pending = Some((MOT_ATTACK_LUNGE_BACK, true));
                } else if held & (LEFT | RIGHT) != 0 {
                    self.rotation[1] = self.rotation[1].wrapping_add(if held & RIGHT != 0 {
                        turn * 4
                    } else {
                        -turn * 4
                    });
                    self.motion_velocity(ctx);
                }
                if held & CIRCLE != 0 {
                    if self.pad.trigger & SQUARE != 0 {
                        return Ok(Some((MOT_ATTACK_CROUCH, true)));
                    }
                    pending = Some((MOT_SQUAT, true));
                }
            }
            MOT_CHASE_DASH_BACK..=MOT_CHASE_DASH_FWD => {
                if self.motion.action == MOT_CHASE_DASH_FWD {
                    if self.pad.trigger & SQUARE != 0 {
                        pending = self.attack_input(ctx, world, held, pending)?;
                    }
                    if self.pad.trigger & CROSS != 0 {
                        pending = self.start_jump(ctx, held, center);
                    }
                }
                if self.motion.count < 7 {
                    ctx.effects
                        .smoke_burst(self.position, 150, 12, 1, ctx.random);
                }
                if self.motion.count == 0 && self.motion.loops != 0 {
                    pending = Some((MOT_ENGAGE_STANCE, true));
                }
                return Ok(pending);
            }
            _ => {}
        }
        if command == 0x31 {
            self.move_at(ctx.math, 120, 0);
            return Ok(Some((MOT_JUMP_FLIP, false)));
        }
        if self.pad.trigger & CROSS != 0 {
            return Ok(self.start_jump(ctx, held, center));
        }
        if self.pad.trigger & TRIANGLE != 0 {
            return self.selected_item(ctx, world, pending);
        }
        if self.pad.trigger & SQUARE != 0 {
            return self.attack_input(ctx, world, held, pending);
        }
        Ok(pending)
    }
}
