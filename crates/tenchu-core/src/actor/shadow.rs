use super::*;
use crate::effects::Shadow;

impl Actor {
    pub fn shadow(
        &mut self,
        ctx: &mut Context,
        items: &mut crate::items::Items,
        roster: &mut crate::roster::Roster,
        clock: i32,
    ) -> Result<()> {
        let mut position = ctx.scene.world(self.parts[0])?.translation;
        if self.map.level < position[1] || self.map.level == terrain::NONE {
            self.map.attribute |= 8;
        }
        position[1] = self.map.level;
        if self.map.attribute & 8 != 0 {
            if (self.velocity[0] != 0
                || self.velocity[1] != 0
                || matches!(self.motion.action, MOT_STATE_LAND | MOT_ATTACK_DIVE_LAND))
                && self.map.height == 0
                && clock & 1 != 0
            {
                let range = if self.status == 3 {
                    position[1] = position[1].wrapping_add(100);
                    300
                } else {
                    100
                };
                position[0] = position[0].wrapping_add(ctx.random.next15() % (range * 2) - range);
                position[2] = position[2].wrapping_add(ctx.random.next15() % (range * 2) - range);
                ctx.effects.splash(position, [8192; 2], 4);
            }
        } else if self.map.attribute & 0x100 != 0 {
            if self.map.height == 0 {
                if clock & 63 == 1 {
                    let mut start = ctx.scene.models[self.root_model].local.translation;
                    start[1] = start[1].wrapping_sub(100);
                    let end = [
                        start[0].wrapping_add(i32::from(self.velocity[0])),
                        start[1].wrapping_sub(15),
                        start[2].wrapping_add(i32::from(self.velocity[2])),
                    ];
                    items.request_napalm(
                        self.id,
                        start,
                        end.into(),
                        clock,
                        &mut crate::roster::Actors {
                            roster,
                            active: Some(self),
                        },
                        ctx,
                    )?;
                }
                if clock & 15 == 0 {
                    self.damage_flash(ctx)?;
                }
            }
        } else {
            let height = -i32::from(ctx.mission.models[&self.kind][0].position[1]);
            let rotation = if self.map.angle_high != 0 {
                [256, ctx.body.reflect_angle[self.map.angle_high as usize], 0]
            } else {
                [0; 3]
            };
            ctx.effects.shadows.push(Shadow {
                position,
                rotation,
                scale: height.wrapping_mul(4).wrapping_sub(self.map.height >> 1),
            });
        }
        Ok(())
    }

    pub(crate) fn damage_flash(&mut self, ctx: &mut Context) -> Result<()> {
        let part = if self.parts.is_empty() {
            0
        } else {
            ctx.random.next15() as usize % self.parts.len()
        };
        let model = *self
            .parts
            .get(part)
            .ok_or("Damaged actor has no model parts")?;
        let position = std::array::from_fn(|_| ctx.random.next15() % 200 - 100);
        let time = (ctx.random.next15() % 60 + 60) as i16;
        ctx.effects.flash(Some(model), position.into(), 12288, time);
        let absolute = ctx.scene.world(model)?.translation;
        ctx.effects.bleeds(
            absolute,
            [0, -60, 0],
            100,
            10,
            30,
            [100, 100, 60],
            ctx.random,
        );
        ctx.effects.sounds.push((self.position, 0x39));
        Ok(())
    }
}
