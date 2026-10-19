use super::*;

impl Actor {
    pub fn terrain_contact(
        &mut self,
        ctx: &mut Context,
        center: [i32; 3],
        player: bool,
    ) -> Result<()> {
        let width = ctx.mission.characters[self.definition].width;
        self.rotation[1] &= 4095;
        self.attribute &= 0xff;
        let mode = terrain::STEP_DOWN
            | if self.map.vector == 0 {
                terrain::REUSE_CACHED
            } else {
                0
            };
        ctx.cursor.area = self.map.area;
        ctx.cursor.row = self.map.row;
        let position = if self.status == 7
            || (self.status != 11
                && self.map.height == 0
                && (i32::from(self.velocity[0]).abs() > 80
                    || i32::from(self.velocity[2]).abs() > 80))
        {
            [center[0], self.position[1], center[2]]
        } else {
            self.position
        };
        self.map = terrain::vector(
            &ctx.mission.terrain,
            ctx.cursor,
            position,
            i32::from(width),
            mode,
        )?;
        if self.map.attribute & 2 != 0 {
            self.attribute |= ATTR_BUOYANT;
            if self.velocity[1] < 0 {
                self.velocity[1] = 0;
            }
            self.map.height = 1;
        }
        if self.map.height > 0 && self.attribute & ATTR_FLOAT == 0 {
            self.attribute |= ATTR_FALL;
            if self.velocity[1] < 400 {
                self.velocity[1] = self.velocity[1].wrapping_add(20);
            }
            if self.map.attribute & 0x200 != 0 && self.map.height < 25000 {
                self.health = 0;
            }
        } else if self.velocity[1] > 0 || self.map.level == terrain::NONE {
            if self.map.attribute & 0x200 == 0 {
                self.attribute |= ATTR_NOFLOOR;
            }
            if self.map.level != terrain::NONE {
                self.position[1] = self.map.level;
            }
            self.velocity[1] = 0;
        }
        if self.map.attribute & 0x200 != 0 && self.map.height == 0 && self.status != 17 {
            self.enter(ctx, MOT_DEAD, true);
        }
        if self.map.angle_low != 0 || self.map.angle_high != 0 {
            self.attribute |= ATTR_WALLANGLE;
            if self.map.height < -450 {
                self.attribute |= ATTR_LEDGE;
                self.position[1] = self.map.level;
            } else if self.map.height < 0 {
                self.position[1] = self.map.level;
            }
        }
        if self.map.vector == 0 {
            return Ok(());
        }
        self.attribute |= ATTR_WALL;
        if self.map.level == terrain::NONE {
            self.position = self.previous;
            let position = [center[0], self.position[1].wrapping_sub(500), center[2]];
            let probe = terrain::vector(
                &ctx.mission.terrain,
                ctx.cursor,
                position,
                300,
                terrain::ALLOW_DEEP,
            )?;
            let [dx, dz] = ctx.body.reflect_move[probe.vector as usize].map(i32::from);
            self.position[0] = self.position[0].wrapping_add(
                dx.wrapping_mul(position[0].wrapping_sub(self.position[0]).wrapping_abs()),
            );
            self.position[2] = self.position[2].wrapping_add(
                dz.wrapping_mul(position[2].wrapping_sub(self.position[2]).wrapping_abs()),
            );
            if probe.level != terrain::NONE && self.position[1] < probe.level {
                self.position[1] = self.position[1].wrapping_add(if self.velocity[1] > 0 {
                    i32::from(self.velocity[1])
                } else {
                    20
                });
            } else {
                self.velocity[0] = 0;
                self.velocity[2] = 0;
            }
            return Ok(());
        }
        let direction = self.map.vector as usize;
        let yaw = ctx.body.reflect_angle[direction];
        if yaw == -1 {
            let [dx, dz] = if self.velocity[0] != 0 || self.velocity[2] != 0 {
                [-i32::from(self.velocity[0]), -i32::from(self.velocity[2])]
            } else {
                ctx.body.reflect_move[direction].map(|v| i32::from(v) * i32::from(width >> 2))
            };
            self.position[0] = self.position[0].wrapping_add(dx);
            self.position[2] = self.position[2].wrapping_add(dz);
            return Ok(());
        }
        let [sin, cos] = ctx.math.pair(yaw);
        let mut dx = sin * i32::from(width) >> 14;
        let mut dz = cos * i32::from(width) >> 14;
        let mut difference = i32::from(self.rotation[1]) - i32::from(yaw);
        if difference.abs() >= 2000 {
            difference += if difference > 0 { -4096 } else { 4096 };
        }
        if difference.abs() < 1800 || !player || self.map.height != 0 {
            if self.map.angle_high == 0 && matches!(self.status, 2 | 6) {
                self.rotation[1] =
                    self.rotation[1].wrapping_add(if difference > 0 { -32 } else { 32 });
                self.motion_velocity(ctx);
            }
            dx >>= 1;
            dz = (dz & !1) >> 1;
        }
        self.position[0] = self.position[0].wrapping_sub(dx);
        self.position[2] = self.position[2].wrapping_sub(dz);
        Ok(())
    }
}
