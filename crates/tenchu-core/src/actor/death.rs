use super::*;

impl Actor {
    pub(super) fn dead(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        if ctx.scene.models[self.parts[0]].conflict.is_none() && self.motion.loops < 0 {
            return Ok(None);
        }
        if self.motion.count != 0 || self.motion.loops != 0 {
            if self.motion.count == 0 {
                self.motion.loops = -1;
            }
            if self.motion.loops < 0 && self.velocity[1] == 0 {
                self.motion.count = ctx.mission.clips[self.motion.clip.unwrap()].duration;
                self.motion.loops = 0;
                self.play_pose(ctx)?;
                self.motion.loops = -1;
                if self.motion.action != MOT_DEAD_DROWN {
                    self.attribute &= !ATTR_SEARCH;
                } else {
                    self.attribute |= ATTR_SEARCH;
                    ctx.scene.models[self.root_model].attribute |= 1;
                }
                self.velocity[0] = 0;
                self.velocity[2] = 0;
                if self.id != world.player {
                    ctx.collision.remove(ctx.scene, self.parts[0]);
                    if self.kind & 0xf0 != 0x80 {
                        world.items.drop_inventory(self, ctx)?;
                    }
                }
                if self.motion.action >= MOT_DEAD_STEALTH_BACK {
                    world.intelligence.action_halt = 0;
                    ctx.camera.snap_pending = true;
                }
                return Ok(None);
            }
        }
        if self.motion.action > MOT_DEAD_DROWN
            && self.id != world.player
            && self.position[1] != world.roster.get(world.player).position[1]
        {
            self.position[1] = self.position[1].wrapping_sub(1);
            world.intelligence.action_halt = 0;
            if self.motion.count < 10 {
                world.intelligence.voices.push((self.id, 8));
                world.intelligence.voices.push((world.player, 5));
            }
            return Ok(Some((MOT_DEAD, true)));
        }
        let mut gore = None;
        match self.motion.action {
            MOT_DEAD_DROWN => {
                if ctx.random.next15() % 20 == 0 {
                    world.intelligence.voices.push((self.id, 0x16));
                }
                if ctx.random.next15() & 5 == 0 {
                    let width = i32::from(ctx.mission.characters[self.definition].width);
                    if width == 0 {
                        return Err("Drowning actor has zero width".into());
                    }
                    for _ in 0..5 {
                        let position = [
                            self.position[0]
                                .wrapping_add((ctx.random.next15() % width) * 2 - width),
                            self.map.level,
                            self.position[2]
                                .wrapping_add((ctx.random.next15() % width) * 2 - width),
                        ];
                        let size =
                            std::array::from_fn(|_| ((ctx.random.next15() & 7) << 12) as i16);
                        ctx.effects.splash(position, size, 6);
                    }
                }
            }
            MOT_DEAD_STEALTH_BACK..=MOT_DEAD_STEALTH_SIDE_AYAME => {
                let events = &ctx.mission.death_events
                    [(self.motion.action - MOT_DEAD_STEALTH_BACK) as usize];
                let event = events
                    .iter()
                    .find(|e| e.action == 4 || e.frame == self.motion.count)
                    .ok_or("Unterminated death event table")?;
                if self.motion.count < event.frame {
                    return Ok(None);
                }
                match event.action {
                    0 => world
                        .intelligence
                        .voices
                        .push((world.player, event.payload[0])),
                    1 => world.intelligence.voices.push((self.id, event.payload[0])),
                    2 => {}
                    3 | 4 => {
                        world.combat.life_bar = Some(self.id);
                        if event.payload[0] != -1 {
                            gore = Some((
                                event.payload[0] as usize,
                                (event.payload[1] as u16 >> 8) as i16,
                                (event.payload[1] as u16 & 255) as i16,
                            ));
                        }
                    }
                    _ => return Err("Unknown death event action".into()),
                }
            }
            _ => {
                if self.kind & 0xf0 != 0xa0 {
                    if self.motion.count == 5 && world.combat.death_sound == Some(self.id) {
                        world.intelligence.voices.push((self.id, 0x38));
                        world.combat.death_sound = None;
                    }
                    gore = Some((1, 100, 0));
                }
            }
        }
        if self.motion.count & 4 != 0
            && let Some((part, z, y)) = gore
        {
            let model = *self.parts.get(part).ok_or("Invalid death gore part")?;
            let position = if y == 0 {
                [0, -200, -240]
            } else {
                [0, -410, 0]
            };
            ctx.effects.gore(
                model,
                position,
                [0, -y, -z],
                world.clock,
                ctx.random,
                ctx.scene,
            )?;
        }
        Ok(None)
    }

    pub(crate) fn corpse(&mut self, ctx: &mut Context, world: &mut ActionContext) -> Result<()> {
        self.death_witness(ctx, world)?;
        if self.motion.loops >= 0 || self.map.attribute & 12 != 0 || self.brain.chase[0] < 0 {
            self.brain.chase[0] = 0;
            return Ok(());
        }
        if self.map.attribute & 0x100 != 0 {
            if world.clock & 15 == 0 {
                self.damage_flash(ctx)?;
            }
            return Ok(());
        }
        self.brain.chase[0] = self.brain.chase[0].wrapping_add(0x88).min(4096);
        let mut position = ctx.scene.world(self.parts[0])?.translation;
        position[1] = ctx.scene.models[self.root_model].local.translation[1];
        let height = i32::from(ctx.mission.models[&self.kind][0].position[1]);
        let scale =
            (self.brain.chase[0].wrapping_mul(-height) / 1024).wrapping_sub(self.map.height >> 1);
        let rotation = if self.map.angle_high != 0 {
            [256, ctx.body.reflect_angle[self.map.angle_high as usize], 0]
        } else {
            [0; 3]
        };
        let mut matrix = Matrix {
            translation: position,
            rotation: ctx.math.rotate_yxz(rotation),
        };
        for row in &mut matrix.rotation {
            for value in row {
                *value = (i32::from(*value).wrapping_mul(scale) >> 12) as i16;
            }
        }
        let matrix = ctx.camera.view.compose(&matrix);
        let depth = ((crate::projection::point(&matrix, [0; 3]).z >> 2) as i16) >> 2;
        if depth < 1250 {
            ctx.drawing.fog = true;
            let mesh = ctx.mission.common_models[&31]
                .first()
                .ok_or("Missing blood pool geometry")?;
            ctx.drawing.bytes += crate::packets::draw_tmd(mesh, &matrix, true);
            ctx.effects.blood_pools.push(crate::effects::Shadow {
                position,
                rotation,
                scale,
            });
        }
        Ok(())
    }

    fn death_witness(&mut self, ctx: &mut Context, world: &mut ActionContext) -> Result<()> {
        if self.attribute & ATTR_SEARCH != 0
            || world.intelligence.difficulty == crate::ai::Difficulty::Easy
        {
            return Ok(());
        }
        let next = world.intelligence.death_index.wrapping_add(1) as i16;
        let index = i32::from(next) % world.roster.order.len() as i32;
        let id = *world
            .roster
            .order
            .get(index as usize)
            .ok_or("Invalid death witness index")?;
        world.intelligence.death_index = index as u16;
        if id == self.id || id == world.player {
            return Ok(());
        }
        let witness = world.roster.get_mut(id);
        if witness.attribute & (ATTR_SUSPEND | ATTR_PHASE) != 0 || matches!(witness.status, 16 | 17)
        {
            return Ok(());
        }
        let mut delta = [
            self.position[0].wrapping_sub(witness.position[0]),
            0,
            self.position[2].wrapping_sub(witness.position[2]),
        ];
        if i32::from(
            ctx.math
                .direction(delta[0], delta[2], witness.rotation[1])?,
        )
        .abs()
            > 900
            || ctx.math.length(delta) > 20000
        {
            return Ok(());
        }
        delta[1] = self.position[1]
            .wrapping_sub(witness.position[1])
            .wrapping_sub(i32::from(ctx.mission.characters[witness.definition].height));
        let mut count: i16 = 1;
        while delta.iter().any(|v| v.wrapping_abs() > 500) {
            count = count.wrapping_shl(1);
            delta = delta.map(|v| v >> 1);
        }
        if terrain::passage(
            &ctx.mission.terrain,
            ctx.cursor,
            witness.position,
            delta.map(|v| v as i16),
            count,
        )?
        .is_none()
        {
            world.intelligence.reset_alert();
            world.intelligence.voices.push((id, 12));
            witness.enter(ctx, MOT_STATE_DRAW, true);
            self.attribute |= ATTR_SEARCH;
            witness.attribute |= ATTR_SEARCH | 1;
            witness.brain.chase = [self.position[0], self.position[2]];
            witness.brain.count = 0;
            witness.brain.timer = 0;
        }
        Ok(())
    }
}
