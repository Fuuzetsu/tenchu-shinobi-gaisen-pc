use super::*;
use crate::{
    collision,
    input::*,
    scene::{COLLIDE, CONFLICT},
};
use tenchu_assets::weapon::kind::*;

#[derive(Clone, Debug, Default)]
pub struct Combat {
    pub death_sound: Option<usize>,
    pub life_bars: Vec<(usize, i16, i16)>,
    pub armor: bool,
}

impl Combat {
    pub(crate) fn request_life(&mut self, actor: &Actor) {
        self.life_bars
            .push((actor.id, actor.health, actor.max_health));
    }
}

impl Actor {
    pub(crate) fn attack_id(&self, ctx: &Context, action: i16) -> usize {
        let clip = self.motion_id(ctx, action);
        ctx.mission
            .weapons
            .attacks
            .iter()
            .position(|b| b.clip == -1 || b.clip == clip)
            .unwrap()
    }

    fn hands(&self, ctx: &Context) -> Result<[ModelId; 2]> {
        let indices = match ctx.mission.characters[self.definition].weapon {
            FIST => [8, 11],
            JAW => [2, 1],
            _ => [13, 14],
        };
        Ok([
            *self
                .parts
                .get(indices[0])
                .ok_or("Missing primary attack part")?,
            *self
                .parts
                .get(indices[1])
                .ok_or("Missing secondary attack part")?,
        ])
    }

    pub(crate) fn clear_attack(&mut self, ctx: &mut Context, mode: u8) -> Result<()> {
        let kind = ctx.mission.characters[self.definition].weapon;
        if mode & 1 != 0 && kind != 0 {
            let hands = self.hands(ctx)?;
            ctx.collision.remove(ctx.scene, hands[0]);
            if kind != JAW {
                ctx.collision.remove(ctx.scene, hands[1]);
            }
        }
        if mode & 2 != 0 {
            self.trails.fill(None);
        }
        self.motion.mask = 0x7fff;
        Ok(())
    }

    pub(super) fn snap_attack(&mut self, ctx: &Context) {
        if let Some(slot) = ctx.scene.models[self.parts[0]].conflict {
            self.position.x = ctx.collision.slots[slot].position.x;
            self.position.z = ctx.collision.slots[slot].position.z;
        }
    }

    pub(super) fn attack_input(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        pending: Option<(i16, bool)>,
    ) -> Result<Option<(i16, bool)>> {
        if (self.kind as u16) < 2
            && let Some(id) = world.roster.nearest(
                self,
                ctx.mission.characters[self.definition]
                    .width
                    .wrapping_add(1000),
                ctx.math,
            )
        {
            let victim = world.roster.get_mut(id);
            let page = victim.kind & 0xf0;
            if !(matches!(page, 0x80 | 0x90 | 0xa0)
                || (page == 0 && !(7..=9).contains(&victim.kind)))
                && victim.attribute & (ATTR_WEAPON_DRAWN | ATTR_PHASE) == 0
                && !matches!(victim.status, 1 | 15)
            {
                self.target = Some(victim.root_model);
                let attacker_angle = ctx.math.direction(
                    victim.position.x.wrapping_sub(self.position.x),
                    victim.position.z.wrapping_sub(self.position.z),
                    self.rotation[1],
                )?;
                let victim_angle = ctx.math.direction(
                    self.position.x.wrapping_sub(victim.position.x),
                    self.position.z.wrapping_sub(victim.position.z),
                    victim.rotation[1],
                )?;
                if self.position.y == victim.position.y
                    && self.map.vector == 0
                    && self.map.angle_low == 0
                    && self.map.angle_high == 0
                {
                    let variant = if i32::from(victim_angle).abs() > 1000
                        && i32::from(attacker_angle).abs() < 1000
                    {
                        0
                    } else if i32::from(victim_angle).abs() < 1000
                        && i32::from(attacker_angle).abs() < 1000
                    {
                        1
                    } else {
                        2
                    } + if self.kind == 1 { 3 } else { 0 };
                    victim.rotation[1] = self.rotation[1];
                    victim.position.x = self.position.x;
                    victim.position.z = self.position.z;
                    ctx.scene
                        .set_translation(victim.root_model, victim.position);
                    victim.health = 0;
                    victim.enter(ctx, MOT_DEAD_STEALTH_BACK + variant, true);
                    ctx.collision.remove(ctx.scene, victim.parts[0]);
                    world.statistics.criticals = world.statistics.criticals.wrapping_add(1);
                    return Ok(Some((MOT_ATTACK_STEALTH_BACK + variant, true)));
                }
            }
        }
        let current = pending.map_or(self.motion.action, |p| p.0);
        let action = if held & DOWN != 0 {
            if !self.registered(ctx, MOT_ATTACK_BACK) {
                return Ok(pending);
            }
            MOT_ATTACK_BACK
        } else if current == MOT_SQUAT {
            if !self.registered(ctx, MOT_ATTACK_CROUCH) {
                return Ok(pending);
            }
            MOT_ATTACK_CROUCH
        } else if current == MOT_CHASE_DASH_FWD {
            if self.motion.count > 10 {
                return Ok(pending);
            }
            if self.registered(ctx, MOT_ATTACK_LUNGE) {
                MOT_ATTACK_LUNGE
            } else {
                MOT_ATTACK
            }
        } else if held & UP != 0 {
            MOT_ATTACK
        } else if held & RIGHT != 0 {
            MOT_ATTACK_RIGHT1
        } else if held & LEFT != 0 {
            MOT_ATTACK_LEFT1
        } else {
            MOT_ATTACK
        };
        if self.id == world.player {
            self.target = world
                .roster
                .nearest(self, 3000, ctx.math)
                .map(|id| world.roster.get(id).root_model);
        }
        Ok(Some((action, true)))
    }

    fn twin_blade(&mut self, world: &mut ActionContext, draw: i16, stow: i16) {
        if self.motion.count == draw {
            if self.weapons[3].is_some() {
                self.weapons[2] = self.weapons[0].take();
                self.weapons[0] = self.weapons[3].take();
                world.intelligence.voices.push((self.id, 1));
            }
        } else if self.motion.count == stow && self.weapons[2].is_some() {
            self.weapons[3] = self.weapons[0].take();
            self.weapons[0] = self.weapons[2].take();
            world.intelligence.voices.push((self.id, 0));
        }
    }

    fn weapon_clash(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        hand: ModelId,
    ) -> Result<()> {
        if ctx.scene.models[hand].attribute & CONFLICT == 0 {
            return Ok(());
        }
        while let Some(id) = ctx.collision.result(ctx.scene, hand, None) {
            let slot = &ctx.collision.slots[id];
            if slot.class & collision::HIT == 0 || slot.actor() == Some(self.id) {
                continue;
            }
            if let Some(other) = slot.actor() {
                world.roster.get_mut(other).move_at(ctx.math, -30, 0);
            }
            self.move_at(ctx.math, -30, 0);
            let position = ctx.collision.slots[ctx.scene.models[hand].conflict.unwrap()].position;
            for _ in 0..10 {
                let velocity = std::array::from_fn(|_| (ctx.random.next15() % 100 - 50) as i16);
                let time = (ctx.random.next15() % 20 + 20) as u8;
                ctx.effects.bleed(position, velocity, time, [0, 127, 255]);
            }
            ctx.scene.models[hand].attribute &= !COLLIDE;
            // Retail indexes BattleDB with the collision slot here.
            let battle = ctx
                .mission
                .weapons
                .attacks
                .get(id)
                .ok_or("Clash collision index exceeds loaded BattleDB")?;
            self.motion.loops = battle.power / -3 - 1;
            world.intelligence.voices.push((self.id, 0x36));
            ctx.effects
                .rumble(self.id, RumbleAction::WeaponClash, 255, 10, 0);
            break;
        }
        Ok(())
    }

    pub(super) fn attack(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
    ) -> Result<Option<(i16, bool)>> {
        if self.motion.count == 1 {
            if self.health == 0 {
                return Ok(Some((MOT_DEAD, true)));
            }
            self.attack = self.attack_id(ctx, self.motion.action);
            world.intelligence.voices.push((
                self.id,
                if self.motion.action == MOT_ATTACK_TAUNT {
                    11
                } else if self.motion.action & 1 != 0 {
                    9
                } else {
                    10
                },
            ));
        }
        let battle = ctx.mission.weapons.attacks[self.attack].clone();
        if let Some(target) = self.target
            && self.motion.count >= 0
            && self.motion.count < battle.steering
        {
            let position = if target == self.root_model {
                self.position
            } else {
                ctx.scene.models[target].local.translation
            };
            let direction = ctx.math.direction(
                position[0].wrapping_sub(self.position.x),
                position[2].wrapping_sub(self.position.z),
                self.rotation[1],
            )?;
            let turn = ctx.mission.characters[self.definition].turn;
            if direction > turn || direction < -turn {
                self.rotation[1] =
                    self.rotation[1].wrapping_add(if direction > turn { 100 } else { -100 });
                self.motion_velocity(ctx);
            }
        }
        let kind = ctx.mission.characters[self.definition].weapon;
        let action = self.motion.action;
        let mut pending = None;
        match action {
            MOT_ATTACK => match self.motion_id(ctx, MOT_ATTACK) {
                0xf1 => self.twin_blade(world, 42, 6),
                0xaa | 0x1a4 => self.bow_control(ctx, world, false)?,
                0xab => self.gun_control(ctx, world, 20, 100)?,
                0xac => self.gun_control(ctx, world, 22, 700)?,
                0xf5 => self.fire_control(ctx, world)?,
                0xe9 => self.lightning_control(ctx, world)?,
                _ => (),
            },
            MOT_ATTACK_SLASH2 => {
                if kind == KATANAL {
                    self.twin_blade(world, 52, 1);
                }
                if kind == KATAYUMI {
                    self.bow_control(ctx, world, true)?;
                }
                if kind == SEVEN {
                    self.lightning_control(ctx, world)?;
                }
            }
            MOT_ATTACK_SLASH3 if kind == KATAYUMI => self.bow_control(ctx, world, true)?,
            MOT_ATTACK_SLASH3 | MOT_ATTACK_SLASH4 if kind == SEVEN => {
                self.lightning_control(ctx, world)?;
            }
            MOT_ATTACK_RIGHT1 if kind == KATANAL => self.twin_blade(world, 52, 16),
            MOT_ATTACK_LEFT1 if kind == KATANAL => self.twin_blade(world, 43, 13),
            MOT_ATTACK_DIVE => {
                if self.motion.count == 1 && self.map.height > 3000 {
                    ctx.camera.mode = crate::camera::Mode::Fall;
                }
                if held & 0xf000 != 0 {
                    let [forward, side] = if held & UP != 0 {
                        [10, 0]
                    } else if held & DOWN != 0 {
                        [-10, 0]
                    } else if held & RIGHT != 0 {
                        [0, -10]
                    } else {
                        [0, 10]
                    };
                    let add = ctx.math.velocity(self.rotation[1], forward, side);
                    let x = self.velocity[0].wrapping_add(add[0]);
                    let z = self.velocity[2].wrapping_add(add[2]);
                    if i32::from(x).abs() <= 100 && i32::from(z).abs() <= 100 {
                        self.velocity[0] = x;
                        self.velocity[2] = z;
                    }
                }
                if self.attribute & ATTR_NOFLOOR != 0 {
                    pending = Some((MOT_ATTACK_DIVE_LAND, false));
                    world.intelligence.voices.push((self.id, 0x1a));
                    ctx.effects
                        .smoke_burst(self.position, 300, 12, 10, ctx.random);
                }
                if self.motion.count == 0 && self.motion.loops == 1 {
                    self.motion.loops = -1;
                }
                if self.motion.loops < 0 {
                    self.motion.loops = self.motion.loops.wrapping_sub(1);
                    if self.motion.loops < -30 {
                        pending = Some((MOT_STATE_FALL, false));
                    }
                }
                if let Some((next, _)) = pending {
                    self.clear_attack(ctx, 3)?;
                    ctx.camera.normal(self, ctx.math)?;
                    if next == MOT_STATE_FALL {
                        return Ok(pending);
                    }
                }
            }
            MOT_ATTACK_DIVE_LAND => {
                if self.motion.loops < 0 {
                    self.motion.loops = 0;
                }
                if self.motion.count == 0 && self.motion.loops != 0 {
                    self.clear_attack(ctx, 3)?;
                    return Ok(Some((MOT_ENGAGE_STANCE, true)));
                }
                if self.map.height <= 0 {
                    self.velocity[0] = self.velocity[0].wrapping_sub(self.velocity[0] >> 2);
                    self.velocity[2] = self.velocity[2].wrapping_sub(self.velocity[2] >> 2);
                }
                return Ok(None);
            }
            MOT_ATTACK_TAUNT => {
                return Ok((self.motion.count == 0 && self.motion.loops != 0)
                    .then_some((MOT_ENGAGE_STANCE, true)));
            }
            MOT_ATTACK_STEALTH_BACK..=MOT_ATTACK_STEALTH_SIDE_AYAME => {
                if self.motion.count == 1 {
                    world.intelligence.action_halt = crate::ai::Halt::Active;
                    ctx.camera.set_mode(
                        crate::camera::Mode::Critical,
                        self,
                        &mut crate::camera::Environment {
                            terrain: &ctx.mission.terrain,
                            cursor: ctx.cursor,
                            math: ctx.math,
                            body: ctx.body,
                        },
                        ctx.random,
                    )?;
                    ctx.camera.snap_pending = true;
                    return Ok(None);
                }
                let target = self.target.ok_or("Stealth attack lost its paired actor")?;
                if self.motion.loops == 0
                    && self.position.y == ctx.scene.models[target].local.translation.y
                {
                    return Ok(None);
                }
                world.intelligence.action_halt = crate::ai::Halt::Running;
                self.snap_attack(ctx);
                self.motion.pose.translation[0][0] = 0;
                self.motion.pose.translation[0][2] = 0;
                ctx.scene
                    .set_translation(self.parts[0], self.motion.pose.translation[0]);
                let action = self.return_normal(ctx, world.player)?;
                self.enter(ctx, action, true);
                self.motion.count = 0;
                self.motion.loops = 0;
                self.play_pose(ctx)?;
                ctx.camera.snap_pending = true;
                return Ok(None);
            }
            _ => (),
        }
        let combo = match action {
            MOT_ATTACK => Some(if held & RIGHT != 0 {
                MOT_ATTACK_SLASH2_RIGHT
            } else if held & LEFT != 0 {
                MOT_ATTACK_SLASH2_LEFT
            } else {
                MOT_ATTACK_SLASH2
            }),
            MOT_ATTACK_SLASH2 => Some(MOT_ATTACK_SLASH3),
            MOT_ATTACK_SLASH3 => Some(MOT_ATTACK_SLASH4),
            MOT_ATTACK_RIGHT1 if held & (LEFT | RIGHT) != 0 => Some(if held & LEFT != 0 {
                MOT_ATTACK_RIGHT2_LEFT
            } else {
                MOT_ATTACK_RIGHT2
            }),
            MOT_ATTACK_LEFT1 if held & (LEFT | RIGHT) != 0 => Some(if held & RIGHT != 0 {
                MOT_ATTACK_LEFT2_RIGHT
            } else {
                MOT_ATTACK_LEFT2
            }),
            _ => None,
        };
        if let Some(next) = combo
            && self.pad.trigger & SQUARE != 0
            && (i32::from(battle.combo) - 3..=i32::from(battle.combo) + 3)
                .contains(&i32::from(self.motion.count))
        {
            self.pad.time = 0;
            self.clear_attack(ctx, 3)?;
            if self.enter_unless_scripted(ctx, next, true) {
                self.snap_attack(ctx);
            }
        }
        if self.motion.loops < 0 {
            self.motion.loops = self.motion.loops.wrapping_add(1);
            if self.motion.loops == 0 {
                self.motion_velocity(ctx);
            }
            return Ok(pending);
        }
        if self.motion.count == 0 && self.motion.loops == 1 {
            self.clear_attack(ctx, 3)?;
            self.enter_unless_scripted(ctx, MOT_ENGAGE_STANCE, true);
            let base = ctx.mission.clips[self.motion.clip.unwrap()].bones[0][0].value[1];
            self.rotation[1] =
                self.rotation[1].wrapping_add(self.motion.pose.rotation[0][1].wrapping_sub(base));
            self.motion.pose.rotation[0][1] = base;
            if self.id == world.player {
                ctx.camera.normal(self, ctx.math)?;
                if action == MOT_ATTACK_LUNGE_BACK {
                    ctx.camera.snap_pending = true;
                }
            }
            self.pad.time = 0;
            return Ok(None);
        }
        if battle.clip == 0 || battle.start < 1 {
            return Ok(pending);
        }
        let hands = self.hands(ctx)?;
        if self.motion.count == battle.start {
            for (hand, &model) in hands.iter().enumerate() {
                if let Some(index) = self.weapon_hitbox[hand] {
                    let hitbox = &ctx.mission.weapons.hitboxes[index];
                    let slot = ctx.collision.insert(ctx.scene, model)?;
                    ctx.collision.slots[slot].owner = Some(crate::collision::Owner::Actor(self.id));
                    ctx.collision.slots[slot].offset = hitbox.center;
                    // Copying WeaponDB.confp also overwrites offset.pad.
                    ctx.collision.slots[slot].count = hitbox.radius;
                    ctx.collision.slots[slot].size = [hitbox.radius; 3];
                    ctx.collision.slots[slot].class = collision::HIT;
                }
            }
            world.intelligence.voices.push((self.id, 2));
            ctx.effects.rumble(self.id, RumbleAction::Attack, 255, 5, 0);
        } else if self.motion.count == battle.end {
            self.clear_attack(ctx, 1)?;
        }
        if self.motion.count < battle.end && self.kind & 0xf0 != 0xa0 {
            for model in hands {
                if ctx.scene.models[model].conflict.is_some() {
                    self.weapon_clash(ctx, world, model)?;
                }
            }
        }
        if battle.trail_start < 1 {
            return Ok(pending);
        }
        if self.motion.count == battle.trail_start {
            for (hand, &parent) in hands.iter().enumerate() {
                if let Some(index) = self.weapon_hitbox[hand] {
                    self.trails[hand] = Some(trail::Trail::new(
                        parent,
                        ctx.mission.weapons.hitboxes[index].trail,
                    ));
                }
            }
        } else if self.motion.count == battle.trail_end {
            self.clear_attack(ctx, 2)?;
        }
        Ok(pending)
    }

    pub(crate) fn play_pose(&mut self, ctx: &mut Context) -> Result<()> {
        let animate = self.motion.loops >= 0;
        self.motion
            .play(&ctx.mission.clips, &ctx.math.tables.hermite, true)?;
        if animate {
            let count = ctx.mission.clips[self.motion.clip.unwrap()].bones.len();
            self.motion
                .pose
                .publish(ctx.scene, &self.parts, ctx.math, self.motion.mask, count);
        }
        Ok(())
    }
}
