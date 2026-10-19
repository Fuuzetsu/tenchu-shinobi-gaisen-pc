use super::*;

impl Actor {
    fn hang_probe(&self, ctx: &mut Context, yaw: i16, forward: i16, y: i32) -> Result<i32> {
        let v = ctx.math.velocity(yaw, forward, 0);
        terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            [
                self.position[0].wrapping_add(i32::from(v[0])),
                y,
                self.position[2].wrapping_add(i32::from(v[2])),
            ],
            terrain::RETURN_DELTA,
        )
    }

    pub(super) fn hang_check(&mut self, ctx: &mut Context) -> Result<bool> {
        if self.kind & 0xf0 == 0xa0
            || self.map.height <= 0
            || self.motion.action == MOT_JUMP_WALLKICK
        {
            return Ok(false);
        }
        let definition = &ctx.mission.characters[self.definition];
        let (width, height) = (definition.width, definition.height);
        let head = self.position[1].wrapping_sub(i32::from(height));
        let v = ctx.math.velocity(self.rotation[1], width >> 1, 0);
        let y = terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            [
                self.position[0].wrapping_add(i32::from(v[0])),
                head.wrapping_sub(290),
                self.position[2].wrapping_add(i32::from(v[2])),
            ],
            0,
        )?;
        if y < self.position[1] {
            let y = terrain::level(
                &ctx.mission.terrain,
                ctx.cursor,
                [
                    self.position[0].wrapping_sub(i32::from(v[0])),
                    head.wrapping_sub(290),
                    self.position[2].wrapping_sub(i32::from(v[2])),
                ],
                0,
            )?;
            if y != terrain::NONE && self.position[1] < y {
                self.position[0] = self.position[0].wrapping_sub(i32::from(v[0] >> 1));
                self.position[2] = self.position[2].wrapping_sub(i32::from(v[2] >> 1));
            }
            return Ok(false);
        }
        let y = terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            [self.position[0], head.wrapping_sub(300), self.position[2]],
            0,
        )?;
        if y < head {
            return Ok(false);
        }
        let y = self.hang_probe(
            ctx,
            self.rotation[1],
            (width >> 1).wrapping_add(300),
            head.wrapping_sub(400),
        )?;
        if y == terrain::NONE || y > 400 {
            return Ok(false);
        }
        self.position[1] = self.position[1].wrapping_sub(105i32.wrapping_sub(y));
        if self.status == 10 {
            return Ok(true);
        }
        let old_y = y;
        let yaw = catch_yaw(self.rotation[1]);
        let probe_y = self.position[1]
            .wrapping_sub(i32::from(height))
            .wrapping_sub(400);
        let y = self.hang_probe(ctx, yaw, (width >> 1).wrapping_add(300), probe_y)?;
        if y == terrain::NONE || y > 400 {
            self.position[1] = self.position[1].wrapping_sub(old_y.wrapping_sub(5));
            return Ok(false);
        }
        self.rotation[1] = yaw;
        let y = self.hang_probe(ctx, yaw, (width >> 1).wrapping_add(100), probe_y)?;
        if y != terrain::NONE && y <= 400 {
            let v = ctx.math.velocity(yaw, -200, 0);
            self.position[0] = self.position[0].wrapping_add(i32::from(v[0]));
            self.position[2] = self.position[2].wrapping_add(i32::from(v[2]));
            self.position[1] = self.position[1].wrapping_sub(105i32.wrapping_sub(y));
        }
        self.enter(ctx, MOT_HANG_CATCH, true);
        Ok(true)
    }

    pub(super) fn hang(
        &mut self,
        ctx: &mut Context,
        held: u16,
        player: usize,
    ) -> Result<Option<(i16, bool)>> {
        self.velocity[1] = 0;
        let mut pending = None;
        match self.motion.action {
            MOT_HANG => {
                if held & DOWN != 0 {
                    let mut escaped = false;
                    for _ in 0..4096 {
                        self.position[1] = self.position[1].wrapping_add(100);
                        if !self.hang_check(ctx)? {
                            escaped = true;
                            break;
                        }
                    }
                    if !escaped {
                        return Err("Ledge release exceeded map bounds".into());
                    }
                    pending = Some((MOT_STATE_FALL, false));
                } else if held & RIGHT != 0 {
                    pending = Some((MOT_HANG_SHIMMY_RIGHT, true));
                } else if held & LEFT != 0 {
                    pending = Some((MOT_HANG_SHIMMY_LEFT, true));
                } else if held & UP != 0
                    && terrain::level(
                        &ctx.mission.terrain,
                        ctx.cursor,
                        [
                            self.position[0],
                            self.position[1].wrapping_sub(2000),
                            self.position[2],
                        ],
                        terrain::STEP_DOWN,
                    )? != terrain::NONE
                {
                    pending = Some((MOT_HANG_PULLUP, true));
                }
            }
            MOT_HANG_SHIMMY_LEFT | MOT_HANG_SHIMMY_RIGHT => {
                if held & (LEFT | RIGHT) == 0 {
                    pending = Some((MOT_HANG, true));
                } else if !self.hang_check(ctx)? {
                    pending = Some((MOT_STATE_FALL, false));
                }
            }
            MOT_HANG_PULLUP => {
                if self.motion.count == 0 && self.motion.loops != 0 {
                    return Ok(Some((self.return_normal(ctx, player)?, true)));
                }
                if self.motion.count >= 0 {
                    self.velocity[1] = -35;
                    if self.motion.count > 40 {
                        self.move_at(ctx.math, 100, 0);
                    }
                }
            }
            MOT_HANG_CATCH => {
                if self.motion.count == 0 && self.motion.loops != 0 {
                    pending = Some((MOT_HANG, true));
                }
            }
            _ => {}
        }
        if self.attribute & ATTR_PUSH != 0 {
            self.move_at(ctx.math, -10, 0);
            pending = Some((MOT_STATE_FALL, false));
        }
        Ok(pending)
    }
}

fn catch_yaw(yaw: i16) -> i16 {
    if yaw & 0xff == 0 {
        yaw
    } else {
        (yaw & 0xc00).wrapping_add(if yaw & 0x200 != 0 { 0x400 } else { 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ledge_catch_keeps_non_cardinal_multiples_of_256() {
        for (input, expected) in [
            (255, 0),
            (256, 256),
            (511, 0),
            (512, 512),
            (513, 1024),
            (4095, 4096),
            (-1, 4096),
            (-256, -256),
        ] {
            assert_eq!(catch_yaw(input), expected);
        }
    }
}
