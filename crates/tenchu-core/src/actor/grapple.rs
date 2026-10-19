use super::*;
use crate::{
    camera::{Environment, Mode},
    input::{CIRCLE, CROSS, SQUARE},
};

impl Actor {
    pub(super) fn grapple_control(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        match self.motion.action {
            MOT_KAGI => {
                if self.motion.count == 0 && self.motion.loops != 0 {
                    self.motion.loops = -1;
                }
                let mut next = None;
                if self.motion.count == 1 {
                    let mut start = self.position;
                    start[1] = start[1]
                        .wrapping_sub(i32::from(ctx.mission.characters[self.definition].height))
                        .wrapping_add(300);
                    world
                        .items
                        .request(0, self, start, start, world.player, world.roster, ctx)?;
                    ctx.effects.sounds.push((self.position, 0x1e));
                } else if self.inventory[25] != 1 {
                    let dx = ctx.camera.target[0].wrapping_sub(self.position[0]);
                    let dz = ctx.camera.target[2].wrapping_sub(self.position[2]);
                    if dx == 0 && dz == 0 {
                        next = Some((self.return_normal(ctx, world.player)?, true));
                    } else {
                        self.rotation[1] = self.rotation[1].wrapping_add(ctx.math.direction(
                            dx,
                            dz,
                            self.rotation[1],
                        )?);
                        next = Some((MOT_KAGI_FLY, true));
                        ctx.effects.sounds.push((self.position, 0x1f));
                    }
                } else if self.pad.trigger & (SQUARE | CROSS | CIRCLE) != 0 {
                    self.inventory[25] = 0;
                    next = Some((self.return_normal(ctx, world.player)?, true));
                }
                if self.map.attribute & 4 != 0 && next.is_some_and(|(action, _)| action >> 8 != 4) {
                    for &part in self.parts.iter().take(13).skip(7).chain(self.parts.first()) {
                        ctx.scene.models[part].attribute |= 1;
                    }
                    next = Some((MOT_SWIM, true));
                    self.motion.mask = 0x7fff;
                }
                Ok(next)
            }
            MOT_KAGI_FLY => {
                if self.motion.count == 0 && self.motion.loops == 1 {
                    let start = ctx.scene.world(self.parts[14])?.translation;
                    self.motion.loops = -1;
                    self.motion.count =
                        ctx.effects
                            .fly_wire(start, ctx.camera.target, ctx.math, ctx.random)?;
                }
                if self.motion.loops != -1 {
                    return Ok(None);
                }
                self.motion.count = self.motion.count.wrapping_sub(1);
                if self.motion.count >= 0 {
                    return Ok(None);
                }
                self.motion.mask = !1;
                if self.map.attribute & 4 != 0 {
                    ctx.effects.sounds.push((self.position, 0x15));
                }
                ctx.effects.sounds.push((self.position, 0x18));
                Ok(Some((MOT_KAGI_PULL, true)))
            }
            MOT_KAGI_PULL => {
                ctx.camera.set_mode(
                    Mode::Aim,
                    self,
                    &mut Environment {
                        terrain: &ctx.mission.terrain,
                        cursor: ctx.cursor,
                        math: ctx.math,
                        body: ctx.body,
                    },
                    ctx.random,
                )?;
                let mut delta: [i32; 3] =
                    std::array::from_fn(|i| ctx.camera.target[i].wrapping_sub(self.position[i]));
                let distance = ctx.math.length([delta[0], 0, delta[2]]);
                let waist = &mut self.motion.pose.rotation[0];
                waist[1] = ctx.math.direction(delta[0], delta[2], self.rotation[1])?;
                waist[0] = ctx.math.atan2(distance, delta[1].wrapping_neg())?;
                ctx.scene.models[self.parts[0]].local.rotation = ctx.math.rotate_yxz(*waist);
                ctx.scene.models[self.parts[0]].flags = 0;
                if delta.iter().all(|v| v.wrapping_abs() < 400) {
                    self.attribute |= ATTR_WALL;
                }
                if self.attribute & (ATTR_PUSH | ATTR_HIT | ATTR_NOFLOOR | ATTR_WALL | ATTR_BUOYANT)
                    != 0
                {
                    let old = self.rotation[1];
                    let sum = old.wrapping_add(waist[1]) as u16;
                    self.rotation[1] =
                        ((sum & 0xc00) + if sum & 512 != 0 { 1024 } else { 0 }) as i16;
                    waist[1] = waist[1].wrapping_add(old.wrapping_sub(self.rotation[1]));
                    self.motion.mask = 0x7fff;
                    self.enter(ctx, MOT_STATE_FALL, false);
                    self.motion.count >>= 1;
                    for i in [0, 2] {
                        self.velocity[i] = if self.map.vector == 15 {
                            self.velocity[i] >> 1
                        } else {
                            0
                        };
                    }
                    self.velocity[1] = 0;
                    return Ok(None);
                }
                while delta.iter().any(|v| v.wrapping_abs() > 400) {
                    delta = delta.map(|v| v >> 1);
                }
                self.velocity = delta.map(|v| v as i16);
                let start = ctx.scene.world(self.parts[14])?.translation;
                if let Some(fog) = ctx.effects.wire(
                    start,
                    ctx.camera.target,
                    ctx.math,
                    crate::effects::View {
                        position: ctx.camera.position,
                        matrix: ctx.camera.view,
                    },
                )? {
                    ctx.drawing.fog = fog;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
