use super::*;
use crate::actor::State as ActorState;
use crate::math::Position;
use crate::{collision, scene::CONFLICT};

impl Actor {
    pub fn body_contact(&mut self, ctx: &mut Context, center: Position) -> Result<()> {
        let model = self.parts[0];
        if ctx.scene.models[model].attribute & CONFLICT == 0 {
            return Ok(());
        }
        let width = i32::from(ctx.mission.characters[self.definition].width);
        while let Some(slot) = ctx.collision.result(ctx.scene, model, None) {
            let other = &ctx.collision.slots[slot];
            if other.actor() == Some(self.id) {
                continue;
            }
            if other.class & collision::HIT != 0 {
                if self.status != ActorState::Dead {
                    self.hit = Some(slot);
                    self.attribute |= ATTR_HIT;
                }
                continue;
            }
            if other.class & collision::SOFT != 0 || self.attribute & ATTR_FLOAT != 0 {
                continue;
            }
            self.attribute |= ATTR_PUSH;
            let previous = self.position;
            for axis in [0, 2] {
                let sign = if ctx.collision.distance[axis] >= 0 {
                    1
                } else {
                    -1
                };
                self.position[axis] = self.position[axis].wrapping_sub(sign * width / 8);
            }
            let top = other.position.y.wrapping_sub(i32::from(other.size[1]));
            if center[1] < top {
                self.position.x = previous[0];
                self.position.z = previous[2];
                if other.class & collision::STAND != 0 {
                    self.velocity[1] = 0;
                    self.map.level = top;
                    self.position.y = top;
                    self.map.height = 0;
                    self.map.attribute &= !0x7f;
                    self.attribute &= !(ATTR_PUSH | ATTR_FALL);
                } else {
                    self.velocity[1] = 10;
                }
                continue;
            }
            if other.position.y.wrapping_add(i32::from(other.size[1])) < center[1]
                && matches!(
                    self.status,
                    ActorState::Normal | ActorState::Walk | ActorState::Engage | ActorState::Chase
                )
            {
                // The original passes world Y as yaw here.
                let direction = ctx.math.direction(
                    other.position.x.wrapping_sub(self.position.x),
                    other.position.z.wrapping_sub(self.position.z),
                    self.position.y as i16,
                )?;
                self.enter(
                    ctx,
                    if i32::from(direction).abs() < 1100 {
                        MOT_DAMAGE
                    } else {
                        MOT_DAMAGE_BACK_LIGHT
                    },
                    true,
                );
            }
            let turn = ctx.mission.characters[self.definition].turn;
            self.rotation[1] =
                self.rotation[1].wrapping_add(if slot & 1 != 0 { turn } else { -turn });
            if self.velocity[0] != 0 || self.velocity[2] != 0 {
                self.motion_velocity(ctx);
                if self.attribute & ATTR_TRACE != 0
                    && let Some(patrol) = &mut self.patrol
                {
                    patrol.count = -20;
                }
            }
        }
        Ok(())
    }

    pub fn terrain_contact(
        &mut self,
        ctx: &mut Context,
        center: Position,
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
        // Balma inherits the preceding query's terrain cache (HUMAN.C).
        if self.kind != 0x85 {
            ctx.cursor.area = self.map.area;
            ctx.cursor.row = self.map.row;
        }
        let position = if self.status == ActorState::Attack
            || (self.status != ActorState::Crouch
                && self.map.height == 0
                && (i32::from(self.velocity[0]).abs() > 80
                    || i32::from(self.velocity[2]).abs() > 80))
        {
            crate::math::Position::new(center[0], self.position.y, center[2])
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
            if self.map.attribute & 0x200 != 0
                && self.map.height < 25000
                && self.health != 0
                && !self.invulnerable
            {
                self.health = 0;
                if player {
                    ctx.camera.mode = crate::camera::Mode::Fall;
                }
            }
        } else if self.velocity[1] > 0 || self.map.level == terrain::NONE {
            if self.map.attribute & 0x200 == 0 {
                self.attribute |= ATTR_NOFLOOR;
            }
            if self.map.level != terrain::NONE {
                self.position.y = self.map.level;
            }
            self.velocity[1] = 0;
        }
        if self.map.attribute & 0x200 != 0
            && self.map.height == 0
            && self.status != ActorState::Dead
        {
            self.enter(ctx, MOT_DEAD, true);
        }
        if self.map.angle_low != 0 || self.map.angle_high != 0 {
            self.attribute |= ATTR_WALLANGLE;
            if self.map.height < -450 {
                self.attribute |= ATTR_LEDGE;
                self.position.y = self.map.level;
            } else if self.map.height < 0 {
                self.position.y = self.map.level;
            }
        }
        if self.map.vector == 0 {
            return Ok(());
        }
        self.attribute |= ATTR_WALL;
        if self.map.level == terrain::NONE {
            self.position = self.previous;
            let position = [center[0], self.position.y.wrapping_sub(500), center[2]];
            let probe = terrain::vector(
                &ctx.mission.terrain,
                ctx.cursor,
                position.into(),
                300,
                terrain::ALLOW_DEEP,
            )?;
            let [dx, dz] = ctx.body.reflect_move[probe.vector as usize].map(i32::from);
            self.position.x = self.position.x.wrapping_add(
                dx.wrapping_mul(position[0].wrapping_sub(self.position.x).wrapping_abs()),
            );
            self.position.z = self.position.z.wrapping_add(
                dz.wrapping_mul(position[2].wrapping_sub(self.position.z).wrapping_abs()),
            );
            if probe.level != terrain::NONE && self.position.y < probe.level {
                self.position.y = self.position.y.wrapping_add(if self.velocity[1] > 0 {
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
            self.position.x = self.position.x.wrapping_add(dx);
            self.position.z = self.position.z.wrapping_add(dz);
            return Ok(());
        }
        let [sin, cos] = ctx.math.pair(yaw);
        let mut dx = (sin * i32::from(width)) >> 14;
        let mut dz = (cos * i32::from(width)) >> 14;
        let mut difference = i32::from(self.rotation[1]) - i32::from(yaw);
        if difference.abs() >= 2000 {
            difference += if difference > 0 { -4096 } else { 4096 };
        }
        if difference.abs() < 1800 || !player || self.map.height != 0 {
            if self.map.angle_high == 0
                && matches!(self.status, ActorState::Walk | ActorState::Chase)
            {
                self.rotation[1] =
                    self.rotation[1].wrapping_add(if difference > 0 { -32 } else { 32 });
                self.motion_velocity(ctx);
            }
            dx >>= 1;
            dz >>= 1;
        }
        self.position.x = self.position.x.wrapping_sub(dx);
        self.position.z = self.position.z.wrapping_sub(dz);
        Ok(())
    }
}
