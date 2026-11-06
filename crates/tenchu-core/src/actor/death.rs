use super::*;
mod dismemberment;
use crate::actor::State as ActorState;

impl Actor {
    pub(super) fn request_witness_aid(
        &self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<()> {
        use crate::ai::Recruitment;
        if !world.roster.keep_all_active
            || !world.roster.reinforcement_call.witnessed_kills
            || world.roster.reinforcement_call.count == 0
            || !world.roster.can_call_reinforcements()
            || world.intelligence.cinematic
            || world.intelligence.ignore_player
            || self.scripted
            || self.id == world.player
            || !(0x10..0x80).contains(&self.kind)
            || ctx.mission.stage.id == 8
        {
            return Ok(());
        }
        let mut nearest = None;
        let range = crate::actor::sight::profile(false, world.intelligence.sight_percent);
        for &id in &world.roster.order {
            if id == self.id || id == world.player {
                continue;
            }
            let witness = world.roster.get(id);
            if witness.health <= 0
                || witness.scripted
                || !(0x10..0x80).contains(&witness.kind)
                || witness.attribute & (ATTR_CUSTOMAI | ATTR_SUSPEND) != ATTR_CUSTOMAI
                || witness.recruitment != Recruitment::Available
                || matches!(witness.status, ActorState::Dead | ActorState::Damaged)
            {
                continue;
            }
            let delta = self.position.wrapping_sub(witness.position);
            let distance = ctx.math.length_wide(delta);
            if distance >= range.sight
                || nearest.is_some_and(|(best, _)| distance >= best)
                || i32::from(ctx.math.direction(delta.x, delta.z, witness.rotation[1])?).abs()
                    >= i32::from(range.half_angle)
            {
                continue;
            }
            let mut from = witness.position;
            from.y -= i32::from(ctx.mission.characters[witness.definition].height) - 300;
            let mut to = self.position;
            to.y -= i32::from(ctx.mission.characters[self.definition].height) / 2;
            let mut step = to.wrapping_sub(from);
            let mut count = 1;
            while step.iter().any(|v| v.abs() > 500) {
                step = step.map(|v| v >> 1);
                count *= 2;
            }
            if terrain::passage(
                &ctx.mission.terrain,
                ctx.cursor,
                from,
                step.map(|v| v as i16).to_array(),
                count,
            )?
            .is_none()
            {
                nearest = Some((distance, id));
            }
        }
        if let Some((_, id)) = nearest {
            let target = world.roster.get(world.player).root_model;
            let witness = world.roster.get_mut(id);
            witness.clear_attack(ctx, 3)?;
            witness.enter(ctx, MOT_ENGAGE_STANCE, true);
            witness.target = Some(target);
            witness.recruitment = Recruitment::Requested;
            witness.attribute = (witness.attribute & !ATTR_PHASE) | ATTR_SEARCH | 1;
            witness.brain.chase = [self.position.x, self.position.z];
            witness.brain.timer = 2;
            witness.brain.count = 0;
            world.intelligence.voices.push((id, 14));
        }
        Ok(())
    }

    pub(super) fn dead(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        if ctx.scene.models[self.parts[0]].conflict.is_none() && self.motion.loops < 0 {
            return Ok(None);
        }
        self.advance_severed(ctx, world)?;
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
                        world.items.drop_inventory(self, world.roster, ctx)?;
                    }
                }
                if self.motion.action >= MOT_DEAD_STEALTH_BACK {
                    world.intelligence.action_halt = crate::ai::Halt::Running;
                    ctx.camera.snap_pending = true;
                }
                self.settle_severed(ctx)?;
                return Ok(None);
            }
        }
        if self.motion.action > MOT_DEAD_DROWN
            && self.id != world.player
            && self.position.y != world.roster.get(world.player).position.y
        {
            self.position.y = self.position.y.wrapping_sub(1);
            world.intelligence.action_halt = crate::ai::Halt::Running;
            if self.motion.count < 10 {
                ctx.effects
                    .rumble(world.player, RumbleAction::StealthInterrupted, 255, 10, 10);
                world.intelligence.voices.push((self.id, 8));
                world.intelligence.voices.push((world.player, 5));
            }
            return Ok(Some((MOT_DEAD, true)));
        }
        let mut gore = None;
        let dismemberment = world.combat.dismemberment;
        let mut sever = None;
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
                            self.position
                                .x
                                .wrapping_add((ctx.random.next15() % width) * 2 - width),
                            self.map.level,
                            self.position
                                .z
                                .wrapping_add((ctx.random.next15() % width) * 2 - width),
                        ];
                        let size =
                            std::array::from_fn(|_| ((ctx.random.next15() & 7) << 12) as i16);
                        ctx.effects.splash(position.into(), size, 6);
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
                    2 => ctx.effects.rumble(
                        world.player,
                        RumbleAction::StealthKill,
                        255,
                        event.payload[0].max(0) as u16,
                        event.payload[1].max(0) as u16,
                    ),
                    3 | 4 => {
                        world.combat.request_life(self);
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
                if dismemberment.enabled
                    && matches!(
                        (self.motion.action, self.motion.count),
                        (MOT_DEAD_STEALTH_BACK, 68) | (MOT_DEAD_STEALTH_BACK_AYAME, 36)
                    )
                    && crate::game::dismemberment::Settings::head(
                        dismemberment.stealth_head,
                        ctx.random,
                    )
                {
                    sever = Some(2);
                }
            }
            _ => {
                if self.kind & 0xf0 != 0xa0 {
                    if dismemberment.enabled && self.kind == 0x8f && self.motion.count == 1 {
                        sever = Some(2);
                    }
                    if self.motion.count == 5 && world.combat.death_sound == Some(self.id) {
                        world.intelligence.voices.push((self.id, 0x38));
                        if dismemberment.enabled {
                            let action = if self.id == world.player {
                                self.motion.action
                            } else {
                                world.roster.get(world.player).motion.action
                            };
                            if matches!(action, MOT_ATTACK_DIVE | MOT_ATTACK_DIVE_LAND)
                                && crate::game::dismemberment::Settings::head(
                                    dismemberment.dive_head,
                                    ctx.random,
                                )
                            {
                                sever = Some(2);
                            } else if crate::game::dismemberment::Settings::arm(
                                dismemberment.arms,
                                ctx.random,
                            ) {
                                sever = Some(
                                    if crate::game::dismemberment::Settings::arm(
                                        dismemberment.right_arm,
                                        ctx.random,
                                    ) {
                                        4
                                    } else {
                                        6
                                    },
                                );
                            }
                        }
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
        if let Some(part) = sever {
            self.sever_part(ctx, world, part)?;
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
        position[1] = ctx.scene.models[self.root_model].local.translation.y;
        let height = i32::from(ctx.mission.models[&self.mesh_kind][0].position[1]);
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
                owner: self.root_model,
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
        let index = (world.intelligence.death_index + 1) % world.roster.order.len();
        let id = *world
            .roster
            .order
            .get(index)
            .ok_or("Invalid death witness index")?;
        world.intelligence.death_index = index;
        if id == self.id || id == world.player {
            return Ok(());
        }
        let witness = world.roster.get(id);
        if witness.attribute & (ATTR_SUSPEND | ATTR_PHASE) != 0
            || matches!(witness.status, ActorState::Damaged | ActorState::Dead)
        {
            return Ok(());
        }
        let mut delta = [
            self.position.x.wrapping_sub(witness.position.x),
            0,
            self.position.z.wrapping_sub(witness.position.z),
        ];
        if i32::from(
            ctx.math
                .direction(delta[0], delta[2], witness.rotation[1])?,
        )
        .abs()
            > 900
            || world.roster.distance(ctx.math, delta.into()) > 20000
        {
            return Ok(());
        }
        delta[1] = self
            .position
            .y
            .wrapping_sub(witness.position.y)
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
            if world.intelligence.strict_corpse_sight {
                let mut from = witness.position;
                from.y += 300 - i32::from(ctx.mission.characters[witness.definition].height);
                let body = ctx.scene.world(self.parts[0])?.translation;
                let mut step = body.wrapping_sub(from);
                let mut count = 1;
                while step.iter().any(|v| v.wrapping_abs() > 300) {
                    step = step.map(|v| v >> 1);
                    count *= 2;
                }
                if terrain::passage(
                    &ctx.mission.terrain,
                    ctx.cursor,
                    from,
                    step.map(|v| v as i16).to_array(),
                    count,
                )?
                .is_some()
                {
                    return Ok(());
                }
            }
            if !world.roster.keep_all_active
                || ctx.math.length_wide(
                    witness
                        .position
                        .wrapping_sub(world.roster.get(world.player).position),
                ) < 20000
            {
                world.intelligence.reset_alert();
            }
            world.intelligence.voices.push((id, 12));
            let witness = world.roster.get_mut(id);
            witness.enter(ctx, MOT_STATE_DRAW, true);
            self.attribute |= ATTR_SEARCH;
            witness.attribute |= ATTR_SEARCH | 1;
            witness.brain.chase = [self.position.x, self.position.z];
            witness.brain.count = 0;
            witness.brain.timer = 0;
        }
        Ok(())
    }
}
