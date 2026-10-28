use super::*;

impl Actor {
    pub(in crate::actor) fn swim_check(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<bool> {
        if self.map.height > 0 || self.map.attribute & 4 == 0 || self.status == 4 {
            return Ok(false);
        }
        if self.status == 3 || (self.status == 17 && self.motion.action == MOT_DEAD_DROWN) {
            return Ok(true);
        }
        if self.status == 17 && self.motion.loops == -1 {
            return Ok(false);
        }
        if self.status != 11 {
            self.snap_attack(ctx);
        }
        let width = i32::from(ctx.mission.characters[self.definition].width);
        if width <= 0 {
            return Err("Swimming actor has nonpositive width".into());
        }
        for _ in 0..20 {
            let position = [
                self.position
                    .x
                    .wrapping_add((ctx.random.next15() % width) * 2 - width),
                self.map.level,
                self.position
                    .z
                    .wrapping_add((ctx.random.next15() % width) * 2 - width),
            ];
            let size = std::array::from_fn(|_| ((ctx.random.next15() & 7) << 12) as i16);
            ctx.effects.splash(position.into(), size, 6);
        }
        self.clear_attack(ctx, 3)?;
        if self.id == world.player {
            ctx.camera.mode = crate::camera::Mode::Swim;
            ctx.effects
                .rumble(self.id, RumbleAction::WaterEntry, 255, 10, 0);
        }
        world.intelligence.action_halt = 0;
        self.hide_body(ctx.scene, true);
        let action = if !self.registered(ctx, MOT_SWIM) || self.health == 0 {
            world.intelligence.voices.push((self.id, 8));
            self.health = 0;
            world.combat.request_life(self);
            MOT_DEAD_DROWN
        } else {
            MOT_SWIM
        };
        self.enter_unless_scripted(ctx, action, true);
        ctx.effects.sounds.push((self.position, 0x16));
        world.intelligence.reset_alert();
        Ok(true)
    }

    fn turn_swimmer(&mut self, ctx: &Context, held: u16, reverse: bool) {
        let turn = ctx.mission.characters[self.definition].turn;
        let turn = if held & RIGHT != 0 {
            turn
        } else {
            turn.wrapping_neg()
        };
        self.rotation[1] =
            self.rotation[1].wrapping_add(if reverse { turn.wrapping_neg() } else { turn });
    }

    pub(super) fn swim(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
    ) -> Result<Option<(i16, bool)>> {
        let mut pending = None;
        match self.motion.action {
            MOT_SWIM => {
                if !self.swim_check(ctx, world)? {
                    pending = Some((MOT_SWIM_EXIT, true));
                } else if held & (LEFT | RIGHT) != 0 {
                    if self.motion.count == 1 {
                        ctx.effects.sounds.push((self.position, 0x15));
                    }
                    self.turn_swimmer(ctx, held, false);
                } else if held & (UP | DOWN) != 0 {
                    pending = Some((MOT_SWIM_STROKE, false));
                    if held & UP != 0 {
                        self.move_at(ctx.math, 60, 0);
                    } else if self.map.angle_high == 0 {
                        self.move_at(ctx.math, -60, 0);
                    }
                }
            }
            MOT_SWIM_STROKE => {
                if self.motion.count == 1 {
                    ctx.effects.sounds.push((self.position, 0x15));
                }
                if held & UP != 0 {
                    if !self.swim_check(ctx, world)? {
                        pending = Some((MOT_SWIM_EXIT, true));
                    } else {
                        if held & (LEFT | RIGHT) != 0 {
                            self.turn_swimmer(ctx, held, false);
                        }
                        self.move_at(ctx.math, 60, 0);
                    }
                } else if held & DOWN != 0 {
                    if self.map.angle_high != 0 || !self.swim_check(ctx, world)? {
                        for i in [0, 2] {
                            self.position[i] =
                                self.position[i].wrapping_sub(i32::from(self.velocity[i]));
                            self.velocity[i] = 0;
                        }
                    } else {
                        if held & (LEFT | RIGHT) != 0 {
                            self.turn_swimmer(ctx, held, true);
                        }
                        self.move_at(ctx.math, -60, 0);
                    }
                } else {
                    pending = Some((MOT_SWIM, true));
                }
            }
            MOT_SWIM_EXIT => {
                if self.motion.count == 1 {
                    self.hide_body(ctx.scene, false);
                    ctx.effects.sounds.push((self.position, 0x15));
                } else if self.motion.count == 0 && self.motion.loops != 0 {
                    return Ok(Some((self.return_normal(ctx, world.player)?, true)));
                } else if self.motion.count > 40 {
                    self.move_at(ctx.math, 100, 0);
                }
                return Ok(None);
            }
            _ => {}
        }
        if self.pad.trigger & TRIANGLE != 0 && world.selected_item == Some(0) {
            self.motion.mask = !1;
            self.hide_body(ctx.scene, false);
            return self.selected_item(ctx, world, pending);
        }
        Ok(pending)
    }
}
