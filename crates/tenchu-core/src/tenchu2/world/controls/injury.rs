use super::*;
use crate::{math::Vector3, tenchu2::collision::OBJECT};

impl<E: Events> Driver<'_, E> {
    pub(super) fn hurt(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let hurt = HurtInput {
            held: input.held,
            player: input.player,
            difficulty: i16::from(c.difficulty),
            map: c.map as i16,
            water_surface: input.water.surface,
        };
        actor.update_hurt(
            request,
            &hurt,
            resources,
            self.random,
            |actor, request, random, cue| match cue {
                HurtCue::Recover => c.recover(actor, request, input.held),
                HurtCue::HideHeldItem => c.emit(actor, Event::HeldItem(None), random),
                HurtCue::Voice(id) => c.emit(actor, Event::Voice(id), random),
                HurtCue::Bubbles => c.emit(actor, Event::Bubbles, random),
                HurtCue::Blood { joint, angle } => c.blood(actor, joint, angle, random),
                HurtCue::WaterRipple { position, size } => {
                    c.emit(actor, Event::WaterRipple { position, size }, random)
                }
                HurtCue::Splash(id) => c.emit(actor, Event::Splash(id), random),
                HurtCue::LandingRumble => c.emit(
                    actor,
                    Event::Rumble {
                        kind: 2,
                        strength: 1,
                    },
                    random,
                ),
                HurtCue::Alert => c.alert.reset(c.difficulty, c.rules),
                HurtCue::Dust => c.emit(actor, Event::Dust, random),
                HurtCue::RemoveWeapons => c.weapon(actor, WeaponMode::Remove, resources),
                HurtCue::AlignToFloor => actor
                    .align_to_floor(&mut self.terrain, &resources.math, |a| c.root(a))
                    .map(|_| ()),
                HurtCue::Dead => {
                    c.dead(actor, request, input, &mut self.terrain, resources, random)
                }
            },
        )
    }

    pub(super) fn damage(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let hit_index = actor.hit.ok_or("Tenchu 2 hit has no collision slot")?;
        let slot = c
            .collision
            .slot(hit_index)
            .ok_or("Invalid Tenchu 2 hit slot")?;
        let owner = slot.owner.filter(|&id| id != c.index);
        let attacker = owner.and_then(|id| {
            c.others.get(id, actor).map(|a| DamageAttacker {
                actor: id,
                player: id == c.player,
                position: a.position,
                power: a
                    .attack
                    .index
                    .and_then(|index| resources.weapons.attacks.get(index))
                    .map_or(0, |a| a.power),
                health: a.health,
                attacking: a.state == State::Attack,
                recoil: 0,
            })
        });
        let body_joint = c.instances[c.index].rig.joints[0];
        let body = c
            .collision
            .index(body_joint)
            .and_then(|id| c.collision.slot(id))
            .ok_or("Tenchu 2 hit has no body")?
            .position;
        let mut damage = DamageInput {
            actor: c.index,
            player: input.player,
            controls_enabled: c.enabled,
            held: input.held,
            map: c.map as i16,
            difficulty: i16::from(c.difficulty),
            armor: c.controls.armor,
            drain_attack: c.controls.drain_attack,
            drain_uses: c.controls.drain_uses,
            story_triggered: c.controls.story_triggered,
            story_damage: c.controls.story_damage,
            camera: input.camera,
            focus: c.controls.focus,
            death_music: c.controls.death_music,
            hit: if slot.flags & OBJECT == 0 {
                Hit::Weapon
            } else {
                Hit::Object((slot.flags & 0x3f) as u8)
            },
            contact: slot.position,
            body,
            contact_delta: Vector3::default(),
            hit_disabled: false,
            attacker,
        };
        let mut resurrect = false;
        actor.apply_damage(
            request,
            &mut damage,
            &mut self.terrain,
            resources,
            self.random,
            |actor, _, damage, random, cue| match cue {
                DamageCue::DropHeldItem => c.emit(actor, Event::DropHeldItem, random),
                DamageCue::HideHeldItem => c.emit(actor, Event::HeldItem(None), random),
                DamageCue::ReleaseGrip { first, second } => {
                    c.grip(actor, first, second, Grip::Release, random)
                }
                DamageCue::FinishContacts => actor.set_attack_contacts(
                    &c.instances[c.index].rig,
                    c.collision,
                    resources,
                    Contacts::Finish,
                ),
                DamageCue::Root => {
                    actor.commit_root(c.root(actor)?);
                    Ok(())
                }
                DamageCue::ContactDelta => {
                    if let Some(contact) = c.collision.contact(body_joint, hit_index) {
                        damage.contact_delta = contact.distance;
                    }
                    Ok(())
                }
                DamageCue::AdvanceMotion => {
                    let playing = actor.motion.loops >= 0;
                    actor.motion.advance(&resources.clips, c.spline, true)?;
                    if playing {
                        actor.motion.publish(
                            c.scene,
                            &c.instances[c.index].rig.joints,
                            &resources.math,
                        );
                    }
                    Ok(())
                }
                DamageCue::Sparks { position, parried } => {
                    c.emit(actor, Event::Sparks { position, parried }, random)
                }
                DamageCue::Voice { target, id } => {
                    c.damage_event(actor, damage, target, Event::Voice(id), random)
                }
                DamageCue::Rumble {
                    target,
                    kind,
                    strength,
                } => c.damage_event(
                    actor,
                    damage,
                    target,
                    Event::Rumble {
                        kind,
                        strength: i16::from(strength),
                    },
                    random,
                ),
                DamageCue::Burn => c.emit(actor, Event::Burn, random),
                DamageCue::Sound(id) => c.emit(actor, Event::Sound(id), random),
                DamageCue::Alert => c.alert.reset(c.difficulty, c.rules),
                DamageCue::KillCredit => {
                    credit(
                        c.stats,
                        c.script,
                        actor,
                        c.index == c.player,
                        damage.attacker.as_ref().map(|a| a.actor) == Some(c.player),
                        c.hero == 2 && c.map >= 12,
                    );
                    Ok(())
                }
                DamageCue::Heal => c.emit(actor, Event::Heal, random),
                DamageCue::Resurrection => {
                    resurrect = true;
                    Ok(())
                }
                DamageCue::ShowHealth => c.emit(actor, Event::ShowHealth, random),
                DamageCue::DeathMusic => c.emit(actor, Event::DeathMusic, random),
                DamageCue::Blood { joint, angle } => c.blood(actor, joint, Some(angle), random),
            },
        )?;
        if resurrect {
            use crate::tenchu2::items::{self, resurrection};
            let markers = c
                .items
                .markers
                .iter()
                .flatten()
                .map(|&model| c.scene.models[model].local.translation);
            let position = resurrection::destination(actor.position, c.home, markers);
            let mut context = items::Context {
                map: self.terrain.map,
                cursor: self.terrain.cursor,
                math: &resources.math,
                scene: c.scene,
                collision: c.collision,
                player: c.player,
            };
            let resurrection = resurrection::Resurrection::new(
                c.index,
                damage.attacker.as_ref().map(|a| a.actor),
                actor,
                &mut context,
                self.random,
            )?;
            let log = resurrection.transform.translation;
            c.items.insert(
                items::State::Resurrection(resurrection),
                c.scene,
                c.collision,
            )?;
            actor.health = actor.health.max(actor.max_health);
            actor.flags &= !0xf00;
            c.items.return_point.position = position;
            actor.position = position;
            actor.previous = position;
            c.environment.background_only = true;
            c.instances[c.index].translucent = true;
            c.emit(
                actor,
                Event::SoundPosition {
                    position: log,
                    id: 52,
                },
                self.random,
            )?;
            damage.camera = 22;
            *c.script = 3;
            input.script = 3;
            if c.index == c.player {
                actor.change_motion(0, Velocity::Motion, resources)?;
                *request = Request {
                    action: 0,
                    velocity: Velocity::Keep,
                };
            } else {
                c.others
                    .get_mut(c.player)?
                    .actor
                    .change_motion(0, Velocity::Motion, resources)?;
            }
        }
        if damage.hit_disabled {
            c.collision
                .slot_mut(hit_index)
                .ok_or("Missing Tenchu 2 hit slot")?
                .flags |= DISABLED;
        }
        c.sync_attacker(&damage)?;
        input.camera = damage.camera;
        c.controls.focus = damage.focus;
        c.controls.drain_uses = damage.drain_uses;
        c.controls.story_triggered = damage.story_triggered;
        c.controls.story_damage = damage.story_damage;
        Ok(())
    }
}

impl<E: Events> Context<'_, E> {
    fn blood(
        &mut self,
        actor: &Actor,
        joint: usize,
        angle: Option<i16>,
        random: &mut Random,
    ) -> Result<()> {
        let joint = *self.instances[self.index]
            .rig
            .joints
            .get(joint)
            .ok_or("Missing Tenchu 2 blood joint")?;
        let position = self.scene.world(joint)?.translation;
        let angle = angle.unwrap_or_else(|| random.next15() as i16);
        self.emit(actor, Event::Blood { position, angle }, random)
    }

    fn damage_event(
        &mut self,
        actor: &Actor,
        input: &DamageInput,
        target: DamageTarget,
        event: Event,
        random: &mut Random,
    ) -> Result<()> {
        match target {
            DamageTarget::Victim => self.emit(actor, event, random),
            DamageTarget::Attacker => match &input.attacker {
                Some(attacker) => self.emit_other(actor, attacker.actor, event, random),
                None => Ok(()),
            },
        }
    }

    fn emit_other(
        &mut self,
        actor: &Actor,
        id: usize,
        event: Event,
        random: &mut Random,
    ) -> Result<()> {
        let other = self
            .others
            .get(id, actor)
            .ok_or("Missing Tenchu 2 event actor")?;
        self.controls.observe(id, other, event);
        Events::emit(
            self.events,
            EventSource {
                listener: listener(
                    &self.others,
                    self.controls.camera_actor,
                    actor,
                    self.view_distance,
                )?,
                voice: self.instances[id].voice,
                index: id,
                actor: other,
                rig: &self.instances[id].rig,
                scene: self.scene,
            },
            event,
            random,
        )
    }

    pub(super) fn dead(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &ControlInput,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        random: &mut Random,
    ) -> Result<()> {
        let player = self
            .others
            .get(self.player, actor)
            .ok_or("Missing Tenchu 2 player")?;
        let mut death = DeathInput {
            actor: self.index,
            is_player: self.index == self.player,
            player: DeathPlayer {
                position: player.position,
                yaw: player.rotation.y,
                velocity: player.velocity,
                turn: player.turn,
                action: player.motion.action,
                target: player.target,
            },
            tick: input.tick,
            map: self.map as i16,
            stealth: self.controls.stealth.as_ref().map(|s| s.playback),
            focus: self.controls.focus,
            carry_point: Position::default(),
        };
        actor.update_dead(
            request,
            &mut death,
            terrain,
            resources,
            random,
            |actor, _, input, random, terrain, cue| match cue {
                DeathCue::CarryPoint => {
                    let joint = *self.instances[self.player]
                        .rig
                        .joints
                        .get(5)
                        .ok_or("Missing Tenchu 2 carry joint")?;
                    input.carry_point = self.scene.world(joint)?.translation;
                    Ok(())
                }
                DeathCue::BodyDisabled(disabled) => {
                    let joint = self.instances[self.index].rig.joints[0];
                    let slot = self
                        .collision
                        .get_mut(joint)
                        .ok_or("Missing Tenchu 2 corpse body")?;
                    if disabled {
                        slot.flags |= DISABLED;
                    } else {
                        slot.flags &= !DISABLED;
                    }
                    Ok(())
                }
                DeathCue::Voice { player, id } => {
                    if player {
                        self.emit_other(actor, self.player, Event::Voice(id), random)
                    } else {
                        self.emit(actor, Event::Voice(id), random)
                    }
                }
                DeathCue::Sound(id) => self.emit(actor, Event::Sound(id), random),
                DeathCue::Rumble { kind, strength } => self.emit(
                    actor,
                    Event::Rumble {
                        kind,
                        strength: i16::from(strength),
                    },
                    random,
                ),
                DeathCue::Blood { joint, angle } => self.blood(actor, joint, angle, random),
                DeathCue::Bubbles => self.emit(actor, Event::Bubbles, random),
                DeathCue::WaterRipple { position, size } => {
                    self.emit(actor, Event::WaterRipple { position, size }, random)
                }
                DeathCue::Splash(id) => self.emit(actor, Event::Splash(id), random),
                DeathCue::PlayerGrip(grip) => {
                    let listener = listener(
                        &self.others,
                        self.controls.camera_actor,
                        actor,
                        self.view_distance,
                    )?;
                    let player = if self.index == self.player {
                        actor
                    } else {
                        &mut self.others.get_mut(self.player)?.actor
                    };
                    if let Some(sound) =
                        player.swap_attachments(&mut self.instances[self.player].rig, 0, 3, grip)?
                    {
                        Events::emit(
                            self.events,
                            EventSource {
                                listener,
                                voice: self.instances[self.player].voice,
                                index: self.player,
                                actor: player,
                                rig: &self.instances[self.player].rig,
                                scene: self.scene,
                            },
                            Event::Voice(sound),
                            random,
                        )?;
                    }
                    Ok(())
                }
                DeathCue::HideHeldItem => self.emit(actor, Event::HeldItem(None), random),
                DeathCue::RemoveWeapons => self.weapon(actor, WeaponMode::Remove, resources),
                DeathCue::GroundPool => {
                    let bone = *self.instances[self.index]
                        .rig
                        .joints
                        .get(1)
                        .ok_or("Missing Tenchu 2 corpse joint")?;
                    let position = self
                        .scene
                        .world(bone)?
                        .translation
                        .wrapping_sub(Position::new(0, 1024, 0));
                    self.effects
                        .death_pool(position, terrain.map, terrain.cursor)
                }
                DeathCue::RemoveJoint(joint) => {
                    if let Some(&joint) = self.instances[self.index].rig.joints.get(joint) {
                        self.collision.remove(joint);
                    }
                    Ok(())
                }
                DeathCue::BodyCollision => actor.set_body_collision(
                    self.instances[self.index].rig.joints[0],
                    self.index,
                    self.collision,
                    true,
                ),
                DeathCue::ResizeBody => {
                    let slot = self
                        .collision
                        .get_mut(self.instances[self.index].rig.joints[0])
                        .ok_or("Missing Tenchu 2 corpse body")?;
                    slot.size = Vector3::new(actor.width, actor.height, actor.width);
                    Ok(())
                }
                DeathCue::FinishContacts => actor.set_attack_contacts(
                    &self.instances[self.index].rig,
                    self.collision,
                    resources,
                    Contacts::Finish,
                ),
                DeathCue::AlignToFloor => actor
                    .align_to_floor(terrain, &resources.math, |a| self.root(a))
                    .map(|_| ()),
            },
        )?;
        self.controls.focus = death.focus;
        if let (Some(session), Some(playback)) = (self.controls.stealth.as_mut(), death.stealth) {
            session.playback = playback;
        }
        Ok(())
    }
}

impl<E: Events> Context<'_, E> {
    fn sync_attacker(&mut self, damage: &DamageInput) -> Result<()> {
        if let Some(attacker) = &damage.attacker {
            let other = &mut self.others.get_mut(attacker.actor)?.actor;
            other.health = attacker.health;
            if attacker.recoil != 0 {
                other.motion.loops = attacker.recoil;
            }
        }
        Ok(())
    }

    pub(super) fn status(
        &mut self,
        actor: &Actor,
        status: StatusResult,
        random: &mut Random,
    ) -> Result<()> {
        if status.died {
            credit(
                self.stats,
                self.script,
                actor,
                self.index == self.player,
                false,
                self.hero == 2 && self.map >= 12,
            );
            let event = if self.index == self.player && self.controls.death_music {
                Event::DeathMusic
            } else {
                Event::Voice(15)
            };
            self.emit(actor, event, random)?;
        }
        if status.show_health {
            self.emit(actor, Event::ShowHealth, random)?;
        }
        if status.staggered {
            self.emit(
                actor,
                Event::Rumble {
                    kind: 4,
                    strength: 1,
                },
                random,
            )?;
        }
        Ok(())
    }
}

pub(in crate::tenchu2::world) fn credit(
    stats: &mut super::super::Statistics,
    script: &mut u16,
    actor: &Actor,
    victim_is_player: bool,
    attacker_is_player: bool,
    tatsumaru_late: bool,
) {
    match actor.kind & 0xf0 {
        0x10 | 0x60 if attacker_is_player => {
            let count = if tatsumaru_late {
                &mut stats.combat_kills
            } else {
                &mut stats.innocents
            };
            *count = count.wrapping_add(1);
        }
        0x10 | 0x60 | 0x70 => {}
        _ if !victim_is_player => {
            stats.killed = stats.killed.wrapping_add(1);
            if attacker_is_player {
                if actor.weapon_drawn() {
                    stats.combat_kills = stats.combat_kills.wrapping_add(1);
                } else {
                    stats.stealth_kills = stats.stealth_kills.wrapping_add(1);
                    *script = 0;
                }
            }
        }
        _ => {}
    }
}
