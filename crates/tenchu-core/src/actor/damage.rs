use super::*;
use crate::{
    ai::Difficulty,
    effects::{Environment, Impact},
};
use tenchu_assets::weapon::kind::KATANAL;

impl Actor {
    fn score_death(&self, world: &mut ActionContext) {
        let counter = if self.kind & 0xf0 == 0x90 {
            &mut world.statistics.friend_hits
        } else if self.attribute & (ATTR_WEAPON_DRAWN | 2) == 0 {
            &mut world.statistics.criticals
        } else {
            &mut world.statistics.murders
        };
        *counter = counter.wrapping_add(1);
    }

    fn recoil(&mut self, damage: i16) {
        if self.status == 7 {
            self.motion.loops = damage / -3 - 1;
            self.velocity[0] = 0;
            self.velocity[2] = 0;
        }
    }

    fn recoil_source(&mut self, world: &mut ActionContext, id: usize, damage: i16) {
        if id == self.id {
            self.recoil(damage);
        } else {
            world.roster.get_mut(id).recoil(damage);
        }
    }

    fn consume_hit(&self, ctx: &mut Context, hit: usize) {
        if let Some(model) = ctx.collision.slots[hit].model {
            ctx.collision.remove(ctx.scene, model);
        }
    }

    fn feedback(&self, world: &mut ActionContext, attacker: usize) {
        world.combat.life_bar = Some(if self.id == world.player {
            attacker
        } else {
            self.id
        });
    }

    pub(super) fn blood(
        &self,
        ctx: &mut Context,
        position: [i32; 3],
        count: i16,
        time: i16,
        clock: i32,
    ) -> Result<()> {
        ctx.effects.blood(
            position,
            count,
            time,
            ctx.random,
            &mut Environment {
                terrain: &ctx.mission.terrain,
                cursor: ctx.cursor,
                clock,
            },
        )
    }

    pub(super) fn damage_control(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        if self.health < 1 || self.motion.action == MOT_SWIM_EXIT {
            return Ok(None);
        }
        let hit = self.hit.ok_or("Hit actor has no collision slot")?;
        if self.id == world.player {
            ctx.camera.normal(self, ctx.math)?;
        }
        if self.kind & 0xf0 == 0xa0 {
            let source = ctx.collision.slots[hit].owner;
            if let Some(id) = source {
                world.intelligence.voices.push((id, 4));
                self.consume_hit(ctx, hit);
                let enemy = if id == self.id {
                    &*self
                } else {
                    world.roster.get(id)
                };
                let damage =
                    ctx.mission.weapons.attacks[enemy.attack_id(ctx, enemy.motion.action)].power;
                self.health = self.health.wrapping_sub(damage);
                if self.health < 0 || self.attribute & ATTR_WEAPON_DRAWN == 0 {
                    self.health = 0;
                }
                let mut position = self.position;
                position[1] = position[1]
                    .wrapping_sub(i32::from(ctx.mission.characters[self.definition].height) / 2);
                ctx.effects.impact(position, 24576, 2, ctx.random);
            } else {
                self.health = 0;
            }
            world.combat.life_bar = Some(self.id);
            let motion = if self.health != 0 {
                world.intelligence.voices.push((self.id, 6));
                world.intelligence.reset_alert();
                MOT_DAMAGE
            } else {
                if self.kind != 0xa9 && (source == Some(world.player) || source.is_none()) {
                    self.score_death(world);
                }
                if self.attribute & (ATTR_WEAPON_DRAWN | 2) != 0 {
                    world.intelligence.voices.push((self.id, 8));
                    world.intelligence.reset_alert();
                }
                MOT_DEAD
            };
            self.enter(ctx, motion, true);
            self.clear_attack(ctx, 3)?;
            return Ok(None);
        }
        if (MOT_ATTACK_STEALTH_BACK..=MOT_ATTACK_STEALTH_SIDE_AYAME).contains(&self.motion.action) {
            world.intelligence.action_halt = 0;
            self.position[1] = self.position[1].wrapping_sub(1);
            return Ok(Some((self.return_motion(), true)));
        }
        if self.motion.action == MOT_DAMAGE_DOWNED {
            return Ok(None);
        }
        self.clear_attack(ctx, 3)?;
        // Cleanup compacts the pool. The source deliberately reads the saved index again.
        let Some(source) = ctx.collision.slots[hit].owner else {
            return self.item_damage(ctx, world, hit);
        };
        let enemy = if source == self.id {
            &*self
        } else {
            world.roster.get(source)
        };
        if self.kind & 0xf0 == 0x80 && source != world.player {
            return Ok(None);
        }
        let enemy_position = enemy.position;
        let enemy_kind = enemy.kind;
        let protection = enemy.active_item == 12;
        let power = ctx.mission.weapons.attacks[enemy.attack_id(ctx, enemy.motion.action)].power;
        let delta =
            std::array::from_fn::<_, 3, _>(|i| enemy_position[i].wrapping_sub(self.position[i]));
        let mut step = delta.map(|v| v as i16);
        let mut steps: i16 = 1;
        while step.iter().any(|v| i32::from(*v).abs() > 100) {
            steps = steps.wrapping_shl(1);
            step = step.map(|v| v >> 1);
        }
        let mut from = self.position;
        from[1] = from[1].wrapping_sub(1000);
        if terrain::passage(&ctx.mission.terrain, ctx.cursor, from, step, steps)?.is_some() {
            return Ok(None);
        }
        let direction = ctx.math.direction(delta[0], delta[2], self.rotation[1])?;
        let angle = i32::from(direction).abs();
        let mut action = self.motion.action;
        if self.id != world.player
            && self.status != 7
            && self.attribute & ATTR_WEAPON_DRAWN != 0
            && self.map.height == 0
            && world.intelligence.difficulty != Difficulty::Easy
        {
            let engage = 3 - world.intelligence.difficulty as i32;
            if ctx.random.next15() % (engage + 1) != 0
                || (matches!(self.kind, 0x87 | 0x8a) && ctx.random.next15() & 1 != 0)
            {
                action = MOT_CHASE_BACK;
            }
        }
        if angle < 700 && action == MOT_ENGAGE {
            return Ok(None);
        }
        if ((angle < 700 && action == MOT_CHASE_BACK) || action == MOT_DAMAGE_GETUP)
            && self.enter(ctx, MOT_ENGAGE, false)
        {
            self.snap_attack(ctx);
            self.rotation[1] = self.rotation[1].wrapping_add(direction);
            self.motion.count = 0;
            self.play_pose(ctx)?;
            self.motion.loops = power.wrapping_neg().wrapping_sub(8);
            self.move_at(
                ctx.math,
                ((i32::from(power) * 5 / 2) as i16)
                    .wrapping_add(80)
                    .wrapping_neg(),
                0,
            );
            self.recoil_source(world, source, power);
            self.consume_hit(ctx, hit);
            let point = ctx.scene.world(self.parts[2])?.transform([
                0,
                (i32::from(power) * 10 + 100) as i16,
                0,
            ]);
            for _ in 0..10 {
                let velocity = std::array::from_fn(|_| (ctx.random.next15() % 100 - 50) as i16);
                let time = (ctx.random.next15() % 20 + 20) as u8;
                ctx.effects.bleed(point, velocity, time, [255, 255, 0]);
            }
            self.feedback(world, source);
            let rotation = (ctx.random.next15() % 360) as i16;
            ctx.effects.impact_ex(Impact {
                parent: None,
                position: point,
                size: [8192, 24576],
                color: [[220; 3], [0; 3]],
                rotation,
                speed: 6,
                time: 9,
                count: 0,
                sprite: 1,
            });
            if ctx.random.next15() & 1 != 0 {
                world.intelligence.voices.push((self.id, 10));
            }
            if enemy_kind & 0xf0 != 0xa0 {
                world.intelligence.voices.push((self.id, 3));
            }
            return Ok(None);
        }
        self.snap_attack(ctx);
        let mut damage = power;
        if source == world.player {
            if self.attribute & ATTR_WEAPON_DRAWN == 0 {
                self.health = 0;
            }
            damage = damage.wrapping_sub(world.intelligence.difficulty as i16 - 2);
        } else if self.id != world.player {
            damage /= 3;
        }
        if enemy_kind == 0xa9 {
            damage = damage.wrapping_mul(6);
        }
        if self.active_item == 12 {
            damage /= 3;
        }
        if protection {
            damage = damage.wrapping_mul(2);
        }
        if self.id == world.player && world.combat.armor {
            damage = (i32::from(damage) * 7 / 10) as i16;
        }
        let mut tier = (damage >> 3).min(3);
        if self.map.height > 0 {
            tier = 3;
        }
        let mut speed = (i32::from(damage) * 5 / 2 + 80) as i16;
        self.rotation[1] = self.rotation[1].wrapping_add(direction);
        if angle < 1024 {
            speed = speed.wrapping_neg();
        } else {
            self.rotation[1] = self.rotation[1].wrapping_sub(2048);
        }
        self.move_at(
            ctx.math,
            if tier == 3 {
                if angle > 1024 { 70 } else { -70 }
            } else {
                speed
            },
            0,
        );
        let motions = [
            MOT_DAMAGE,
            MOT_DAMAGE_FRONT_MID,
            MOT_DAMAGE_FRONT_HEAVY,
            MOT_DAMAGE_LAUNCH_BACK,
            MOT_DAMAGE_BACK_LIGHT,
            MOT_DAMAGE_BACK_LIGHT,
            MOT_DAMAGE_BACK_HEAVY,
            MOT_DAMAGE_LAUNCH_FORE,
        ];
        self.health = self.health.wrapping_sub(damage);
        let (mut next, movement);
        if self.health <= 0 {
            self.health = 0;
            world.combat.death_sound = Some(self.id);
            if tier != 3 {
                next = MOT_DEAD;
                movement = true;
                self.enter(ctx, next, movement);
                if ctx.random.next15() & 1 != 0 {
                    next = MOT_DEAD_ALT;
                }
            } else {
                if angle > 1024 {
                    tier += 4;
                }
                self.motion.action = -1;
                next = *motions
                    .get(tier as usize)
                    .ok_or("Invalid damage motion tier")?;
                movement = true;
            }
            if source == world.player {
                self.score_death(world);
            }
            if self.attribute & (ATTR_WEAPON_DRAWN | 2) != 0 {
                world.intelligence.voices.push((self.id, 8));
                world.intelligence.reset_alert();
            }
        } else {
            if angle > 1024 {
                tier += 4;
            }
            self.motion.action = -1;
            next = *motions
                .get(tier as usize)
                .ok_or("Invalid damage motion tier")?;
            movement = false;
            world.intelligence.reset_alert();
        }
        self.recoil_source(world, source, damage);
        self.consume_hit(ctx, hit);
        let mut position = self.position;
        position[1] =
            position[1].wrapping_sub(i32::from(ctx.mission.characters[self.definition].height) / 2);
        self.blood(ctx, position, 5, 120, world.clock)?;
        ctx.effects.impact(position, 24576, 2, ctx.random);
        self.feedback(world, source);
        let voice = if ctx.random.next15() & 1 != 0 { 6 } else { 7 };
        world.intelligence.voices.push((self.id, voice));
        let sound = if ctx.random.next15() & 1 == 0 && self.health == 0 {
            5
        } else {
            4
        };
        world.intelligence.voices.push((source, sound));
        self.finish_damage(ctx, world, next, movement)
    }

    fn finish_damage(
        &mut self,
        ctx: &mut Context,
        _world: &mut ActionContext,
        mut next: i16,
        mut movement: bool,
    ) -> Result<Option<(i16, bool)>> {
        if self.health == 0 && self.inventory[10] != 0 {
            return Err("Kawarimi resurrection is not implemented yet".into());
        }
        self.pad.time = 0;
        if matches!(self.motion.action, MOT_SWIM | MOT_SWIM_STROKE) {
            self.velocity[0] = 0;
            self.velocity[2] = 0;
            if self.health != 0 {
                return Ok(None);
            }
            next = MOT_DEAD_DROWN;
            movement = true;
        }
        self.enter(ctx, next, movement);
        Ok(None)
    }

    fn item_damage(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        hit: usize,
    ) -> Result<Option<(i16, bool)>> {
        if self.status == 16 {
            return Ok(None);
        }
        ctx.collision.result(ctx.scene, self.root_model, Some(hit));
        let kind = world.items.kind_for_contact(ctx.scene, hit);
        let mut next = self.motion.action;
        let mut movement = true;
        let mut damage: i16 = match kind {
            2 => {
                next = MOT_DAMAGE_MAKIBISHI;
                movement = true;
                3
            }
            1 | 15 | 20 | 21 => {
                if kind == 1 && matches!(self.kind, 0x87 | 0x8a) {
                    self.inventory[1] = self.inventory[1].wrapping_add(1);
                }
                let mut position = self.position;
                position[1] = position[1]
                    .wrapping_sub(i32::from(ctx.mission.characters[self.definition].height) / 2);
                next = MOT_DAMAGE;
                movement = true;
                self.blood(ctx, position, 5, 90, world.clock)?;
                match kind {
                    15 => 30,
                    21 => 10,
                    _ => 20,
                }
            }
            22 => {
                next = if ctx.random.next15() & 1 == 0 {
                    MOT_DAMAGE_BACK_LIGHT
                } else {
                    MOT_DAMAGE_FRONT_MID
                };
                movement = true;
                25
            }
            4 | 6 | 23 => {
                if self.map.attribute & 4 == 0 {
                    self.map.height = 1;
                }
                if kind == 6 { 45 } else { 30 }
            }
            _ => 0,
        };
        if self.map.height > 0 {
            let direction = ctx.math.direction(
                i32::from(ctx.collision.distance[0]),
                i32::from(ctx.collision.distance[2]),
                self.rotation[1],
            )?;
            let forward = i32::from(direction).abs() >= 1024;
            next = if forward {
                MOT_DAMAGE_LAUNCH_FORE
            } else {
                MOT_DAMAGE_LAUNCH_BACK
            };
            movement = false;
            self.rotation[1] = self.rotation[1]
                .wrapping_add(direction)
                .wrapping_add(if forward { 2048 } else { 0 });
            self.move_at(ctx.math, if forward { 70 } else { -70 }, 0);
        }
        if self.id == world.player && world.combat.armor {
            damage = damage * 7 / 10;
        }
        self.health = self.health.wrapping_sub(damage);
        let voice = if self.health <= 0 {
            self.health = 0;
            if !matches!(next, MOT_DAMAGE_LAUNCH_BACK | MOT_DAMAGE_LAUNCH_FORE) {
                next = MOT_DEAD;
                movement = true;
            }
            if !matches!(world.items.kind_for_contact(ctx.scene, hit), 20 | 21 | 23) {
                self.score_death(world);
            }
            8
        } else if ctx.random.next15() & 1 != 0 {
            6
        } else {
            7
        };
        world.intelligence.voices.push((self.id, voice));
        if self.id != world.player {
            world.combat.life_bar = Some(self.id);
        }
        world.intelligence.reset_alert();
        self.finish_damage(ctx, world, next, movement)
    }

    pub(super) fn recover_blade(&mut self, ctx: &Context, world: &mut ActionContext) {
        if ctx.mission.characters[self.definition].weapon == KATANAL && self.weapons[3].is_some() {
            self.weapons[2] = self.weapons[0].take();
            self.weapons[0] = self.weapons[3].take();
            world.intelligence.voices.push((self.id, 1));
        }
    }

    pub(super) fn damaged(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        let count = self.motion.count;
        let complete = count == 0 && self.motion.loops != 0;
        let mut done = false;
        let mut pending = None;
        match self.motion.action {
            MOT_DAMAGE_LAUNCH_BACK | MOT_DAMAGE_LAUNCH_FORE => {
                if count == 1 {
                    for (i, &model) in self.parts.iter().enumerate() {
                        if i == 0 || (7..=12).contains(&i) {
                            ctx.scene.models[model].attribute &= !1;
                        }
                    }
                } else if complete {
                    self.motion.loops = -1;
                } else {
                    self.velocity[1] = count
                        .wrapping_sub(ctx.mission.clips[self.motion.clip.unwrap()].duration)
                        .wrapping_mul(6);
                }
                if self.attribute & ATTR_NOFLOOR != 0 || self.map.height < 0 {
                    pending = Some((
                        if self.motion.action == MOT_DAMAGE_LAUNCH_BACK {
                            MOT_DAMAGE_SLAM_BACK
                        } else {
                            MOT_DAMAGE_SLAM_FORE
                        },
                        false,
                    ));
                }
                if count & 4 != 0 {
                    self.blood(ctx, self.position, 1, 60, world.clock)?;
                }
            }
            MOT_DAMAGE_SLAM_BACK | MOT_DAMAGE_SLAM_FORE => {
                if count == 1 {
                    world.intelligence.voices.push((self.id, 0x1d));
                    ctx.effects
                        .smoke_burst(self.position, 500, 30, 30, ctx.random);
                }
                if complete {
                    pending = Some((MOT_DAMAGE_DOWNED, true));
                } else {
                    self.velocity[0] = self.velocity[0].wrapping_sub(self.velocity[0] >> 2);
                    self.velocity[2] = self.velocity[2].wrapping_sub(self.velocity[2] >> 2);
                }
            }
            MOT_DAMAGE_DOWNED => {
                if self.health == 0 {
                    self.motion.loops = 0;
                    self.motion.count = 0;
                    self.play_pose(ctx)?;
                    self.motion.loops = -2;
                    self.status = 17;
                    self.attribute &= !ATTR_SEARCH;
                    self.velocity = [0; 3];
                    if self.id != world.player {
                        ctx.collision.remove(ctx.scene, self.parts[0]);
                        world.items.drop_inventory(self, ctx)?;
                    }
                    return Ok(None);
                }
                self.motion.loops = self.motion.loops.wrapping_sub(1);
                if i32::from(self.health) - i32::from(self.max_health)
                    >= i32::from(self.motion.loops)
                {
                    pending = Some((MOT_DAMAGE_GETUP, true));
                }
            }
            MOT_DAMAGE_MAKIBISHI | MOT_DAMAGE_CHOKE | MOT_DAMAGE_GETUP => done = complete,
            _ => {
                for i in [0, 2] {
                    self.velocity[i] =
                        self.velocity[i].wrapping_add(match self.velocity[i].cmp(&0) {
                            std::cmp::Ordering::Less => 4,
                            std::cmp::Ordering::Greater => -4,
                            _ => 0,
                        });
                }
                if complete {
                    done = true;
                    self.recover_blade(ctx, world);
                }
            }
        }
        if done {
            pending = Some((
                if self.attribute & ATTR_WEAPON_DRAWN != 0 {
                    self.attribute = (self.attribute & !ATTR_PHASE) | 2;
                    MOT_ENGAGE_STANCE
                } else {
                    MOT_STATE_DRAW
                },
                true,
            ));
        }
        Ok(pending)
    }
}
