use super::*;
use crate::{
    collision::{DOOR_OWNER, SOFT},
    math::Math,
    packets,
    scene::CONFLICT,
};

fn direction(angle: i16, yaw: i16) -> i16 {
    if (i32::from(angle) + i32::from(yaw) + 8192) % 4096 <= 2048 {
        64
    } else {
        -64
    }
}

fn underflow_direction(math: &Math, position: [i32; 3], yaw: i16) -> Result<i16> {
    // ConflictObject[-1] aliases 24-bit ordering-table links. Prove the same
    // direction for their entire retail range instead of emulating GPU pointers.
    let range = |axis| -> Option<[i32; 2]> {
        Some([
            0x98040i32.checked_sub(position[axis])?,
            0xffffffi32.checked_sub(position[axis])?,
        ])
    };
    let unknown = "Door conflict underflow has no proven direction";
    let x = range(0).ok_or(unknown)?;
    let z = range(2).ok_or(unknown)?;
    if x[0] <= 0 || z[0] <= 0 {
        return Err(unknown.into());
    }
    let quotient = (i64::from(x[0].min(z[0])) * 1024 / i64::from(x[1].max(z[1]))) as usize;
    let expected = direction(math.tables.atan[quotient], yaw);
    for &angle in &math.tables.atan[quotient..=1024] {
        if [angle, (1024 - i32::from(angle)) as i16]
            .into_iter()
            .any(|a| direction(a, yaw) != expected)
        {
            return Err(unknown.into());
        }
    }
    Ok(expected)
}

impl Object {
    pub(super) fn pause(&mut self, paused: bool, ctx: &mut Context) -> Result<()> {
        let Some(model) = self.model else {
            return Ok(());
        };
        if paused {
            ctx.collision.remove(ctx.scene, model);
            return Ok(());
        }
        let (width, door) = match self.kind {
            Kind::Door { kind, .. } => (ctx.mission.misc.doors[kind].width, true),
            Kind::Pitfall { kind, .. } => (ctx.mission.misc.pitfalls[kind].width, false),
            _ => unreachable!(),
        };
        let id = ctx.collision.insert(ctx.scene, model)?;
        let slot = &mut ctx.collision.slots[id];
        let narrow = (i32::from(width) / 3 * 2) as i16;
        slot.owner = Some(DOOR_OWNER);
        slot.class = SOFT;
        slot.offset = [
            0,
            if door {
                (-i32::from(width) / 2) as i16
            } else {
                0
            },
            0,
        ];
        slot.size = if door {
            [narrow, width, narrow]
        } else {
            [width, narrow, narrow]
        };
        if door {
            self.angle = 0;
        }
        Ok(())
    }

    pub(super) fn hinged(
        &mut self,
        ctx: &mut Context,
        leaves: [Option<ModelId>; 2],
        drawings: &mut Vec<(usize, Matrix)>,
    ) -> Result<()> {
        let (data, yaw, door) = match self.kind {
            Kind::Door { kind, rotation } => (ctx.mission.misc.doors[kind], rotation, true),
            Kind::Pitfall { kind, rotation } => (ctx.mission.misc.pitfalls[kind], rotation, false),
            _ => unreachable!(),
        };
        let model = self.model.unwrap();
        match self.mode {
            0 if ctx.scene.models[model].attribute & CONFLICT != 0 => {
                let contact = ctx
                    .collision
                    .result(ctx.scene, model, None)
                    .map(|id| &ctx.collision.slots[id]);
                // The exhausted-result alias is systemAFS.maxElementArea,
                // which is not the door-owner sentinel on the supported disc.
                if contact.is_none_or(|slot| slot.owner != Some(DOOR_OWNER)) {
                    if door {
                        self.speed = match contact {
                            Some(contact) => direction(
                                ctx.math.atan2(
                                    contact.position[2].wrapping_sub(self.position[2]),
                                    contact.position[0].wrapping_sub(self.position[0]),
                                )?,
                                yaw,
                            ),
                            None => underflow_direction(ctx.math, self.position, yaw)?,
                        };
                    }
                    self.mode += 1;
                    if !door || self.angle == 0 {
                        ctx.effects.sounds.push((self.position, 0x40));
                    }
                }
            }
            1 if door => {
                if i32::from(self.angle).abs() < 960 {
                    self.angle = self.angle.wrapping_add(self.speed);
                } else {
                    self.mode = 0;
                }
            }
            1 => {
                self.angle = self.angle.wrapping_add(170);
                if self.angle >= 1024 {
                    self.angle = 1024;
                    self.mode += 1;
                }
            }
            _ => {}
        }
        let mut width = i32::from(data.width);
        if door {
            width -= width * i32::from(self.angle).abs() / 10240;
        }
        for (side, leaf) in leaves.into_iter().enumerate() {
            let Some(leaf) = leaf else {
                continue;
            };
            let sign = if side == 0 { -1 } else { 1 };
            let mut rotation = [0; 3];
            rotation[if door { 1 } else { 2 }] = self.angle.wrapping_mul(-sign as i16);
            ctx.scene.models[leaf].parent = Some(model);
            ctx.scene.update(
                leaf,
                Matrix {
                    translation: [width * sign, 0, 0],
                    rotation: ctx.math.rotate_yxz(rotation),
                },
            );
            let world = ctx.scene.world(leaf)?;
            let matrix = ctx.camera.view.compose(&world);
            let node = &ctx.scene.models[leaf];
            if let Some(clip) = visibility::clip(&matrix, node.attribute, node.clip) {
                ctx.drawing.fog = clip.fog;
                let mesh = data.models[side].unwrap();
                if let Some(geometry) = ctx.mission.common_models[&mesh].first() {
                    ctx.drawing.bytes += packets::draw_tmd(geometry, &matrix, clip.fog);
                }
                drawings.push((mesh, world));
            }
        }
        Ok(())
    }
}
