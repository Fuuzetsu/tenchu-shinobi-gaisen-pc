use super::*;

impl<E: Events> Context<'_, E> {
    pub(super) fn start_attack(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        held: u16,
        resources: &Resources,
    ) -> Result<()> {
        let victim = (self.index == self.player)
            .then(|| self.nearest(actor, i32::from(actor.height), false, resources))
            .flatten();
        let attack = actor.attack_input(
            *request,
            held,
            victim.and_then(|id| self.others.get(id, actor)),
            self.map as i16,
            &resources.math,
        )?;
        if attack.draw_weapon {
            self.weapon(actor, WeaponMode::Draw, resources)?;
        }
        if let AttackStart::Stealth(style) = attack.start {
            let id = victim.ok_or("Missing Tenchu 2 stealth victim")?;
            let victim = &mut self.others.get_mut(id)?.actor;
            let mut input = StealthInput {
                hero: self.hero,
                victim: id,
                style,
                penalties: *self.detections,
                story_damage: self.controls.story_damage,
            };
            let rig = &mut self.instances[id].rig;
            self.controls.stealth = Some(actor.start_stealth(
                victim,
                request,
                &mut input,
                resources,
                |_, victim, cue| {
                    match cue {
                        StealthCue::VictimWeapons => victim.set_weapon(
                            rig,
                            id,
                            self.collision,
                            resources,
                            WeaponMode::Remove,
                        )?,
                        StealthCue::VictimCollision => {
                            victim.set_body_collision(rig.joints[0], id, self.collision, false)?
                        }
                        StealthCue::KillCredit => credit(
                            self.stats,
                            self.script,
                            victim,
                            id == self.player,
                            Some(self.index) == Some(self.player),
                            self.hero == 2 && self.map >= 12,
                        ),
                        _ => return Err("Unexpected Tenchu 2 stealth-entry event".into()),
                    }
                    Ok(())
                },
            )?);
            *self.detections = input.penalties;
            self.controls.story_damage = input.story_damage;
        } else {
            actor.start_strike(attack.start, request, resources, |actor| self.root(actor))?;
        }
        self.sync_contacts(actor);
        Ok(())
    }
}

impl<E: Events> Driver<'_, E> {
    pub(super) fn attack(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        if (0x920..=0x926).contains(&actor.motion.action) {
            let victim = &c
                .others
                .get_mut(actor.target.ok_or("Missing Tenchu 2 stealth victim")?)?
                .actor;
            let session = c
                .controls
                .stealth
                .as_ref()
                .ok_or("Missing Tenchu 2 stealth session")?;
            let rig = &mut c.instances[c.index].rig;
            return actor.update_stealth(
                victim,
                request,
                session,
                resources,
                |actor, request, cue| {
                    let event = match cue {
                        StealthCue::Root => {
                            actor.commit_root(rig.root_position(actor, c.scene)?);
                            None
                        }
                        StealthCue::Recover => {
                            request.action = actor
                                .normal_motion(input.held, |a| rig.root_position(a, c.scene))?;
                            request.velocity = Velocity::Motion;
                            None
                        }
                        StealthCue::ReleaseGrip => actor
                            .swap_attachments(rig, 0, 3, Grip::Release)?
                            .map(Event::Voice),
                        StealthCue::Voice(id) => Some(Event::Voice(id)),
                        StealthCue::Tint => Some(Event::StealthTint),
                        StealthCue::FinishContacts => {
                            actor.set_attack_contacts(
                                rig,
                                c.collision,
                                resources,
                                Contacts::Finish,
                            )?;
                            None
                        }
                        _ => return Err("Unexpected Tenchu 2 stealth-update event".into()),
                    };
                    if let Some(event) = event {
                        c.events.emit(
                            EventSource {
                                index: c.index,
                                actor,
                                rig,
                                scene: c.scene,
                            },
                            event,
                            self.random,
                        )?;
                    }
                    Ok(())
                },
            );
        }
        let preparation =
            actor.prepare_attack(request, resources, self.random, input.player, c.enabled)?;
        if preparation.status != StatusResult::default() {
            c.status(actor, preparation.status, self.random)?;
        }
        if preparation.interrupted {
            return Ok(());
        }
        if preparation.retarget {
            actor.target = c.nearest(actor, 3000, true, resources);
            *c.tracking = actor.target;
        }
        actor.steer_attack(
            *request,
            actor
                .target
                .and_then(|id| c.others.get(id, actor))
                .map(|a| a.position),
            resources,
        )?;
        let mut falling_dead = false;
        if actor
            .attack_sequence(request, input.held, resources, |actor, _, cue| {
                match cue {
                    AttackCue::Grip { slot, spare, grip } => {
                        c.grip(actor, slot, spare, grip, self.random)?
                    }
                    AttackCue::NockArrow => c.emit(actor, Event::NockArrow, self.random)?,
                    AttackCue::Fire(kind) => c.emit(actor, Event::Fire(kind), self.random)?,
                    AttackCue::Land => {
                        c.emit(
                            actor,
                            Event::Rumble {
                                kind: 2,
                                strength: 1,
                            },
                            self.random,
                        )?;
                        if actor.terrain.material & 0xf00 == 0x200 {
                            c.emit(
                                actor,
                                Event::WaterRipple {
                                    position: actor.position,
                                    size: 0x400,
                                },
                                self.random,
                            )?;
                            c.emit(actor, Event::Voice(28), self.random)?;
                            if input.player {
                                c.alert.raise(
                                    crate::tenchu2::ai::status::AlertCause::Suspicion,
                                    c.difficulty,
                                    c.rules,
                                )?;
                            }
                        } else {
                            c.emit(actor, Event::Dust, self.random)?;
                            c.emit(actor, Event::Voice(23), self.random)?;
                        }
                    }
                    AttackCue::FallDead => {
                        actor.motion.action = 0x702;
                        falling_dead = true;
                    }
                }
                Ok(())
            })?
            .is_break()
        {
            return if falling_dead {
                self.special(actor, request, input, resources)
            } else {
                Ok(())
            };
        }
        actor.finish_attack(
            request,
            input.held,
            resources,
            &mut AttackContext {
                rig: &c.instances[c.index].rig,
                scene: c.scene,
                collision: c.collision,
                random: self.random,
            },
            |actor, context, cue| {
                if let CombatCue::Clash { opponent, .. } = cue {
                    c.others.get_mut(opponent)?.actor.motion.loops = -5;
                }
                c.events.emit(
                    EventSource {
                        index: c.index,
                        actor,
                        rig: context.rig,
                        scene: context.scene,
                    },
                    Event::Combat(cue),
                    context.random,
                )
            },
        )
    }
}
