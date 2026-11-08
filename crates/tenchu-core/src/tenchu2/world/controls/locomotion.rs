use super::*;

impl<E: Events> Driver<'_, E> {
    pub(super) fn standing(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let mut ledge = c.ledge_input(input);
        let standing = StandingInput {
            held: input.held,
            player: input.player,
            controls_enabled: c.enabled,
            water: input.water.depth.is_some(),
            map: c.map as i16,
            sequence: c.sequence as i16,
            story_triggered: c.controls.story_triggered,
            carried_action: actor
                .target
                .and_then(|id| c.others.get(id, actor))
                .map(|a| a.motion.action),
        };
        actor.update_standing(
            request,
            &standing,
            resources,
            self.random,
            |actor, request, random, cue| {
                match cue {
                    StandingCue::Status(status) => c.status(actor, status, random)?,
                    StandingCue::Voice(id) => c.emit(actor, Event::Voice(id), random)?,
                    StandingCue::Splash(id) => c.emit(actor, Event::Splash(id), random)?,
                    StandingCue::Dust => c.emit(actor, Event::DustTrail, random)?,
                    StandingCue::Footstep { check_surface } => {
                        let kind = if check_surface {
                            use crate::tenchu2::environment::{SurfaceActor, SurfaceContext};
                            c.environment.surface(
                                SurfaceActor {
                                    index: c.index,
                                    position: actor.position,
                                    height: actor.height,
                                    health: actor.health,
                                    visible: actor.flags & 0x100000 != 0,
                                    focused: c.index == c.controls.camera_actor,
                                },
                                &actor.terrain,
                                c.hidden,
                                &mut SurfaceContext {
                                    map: self.terrain.map,
                                    cursor: self.terrain.cursor,
                                    math: &resources.math,
                                    random,
                                    events: c.events,
                                },
                            )?
                        } else {
                            -1
                        };
                        if kind == 18 {
                            c.alert.raise(
                                crate::tenchu2::ai::status::AlertCause::Combat,
                                c.difficulty,
                                c.rules,
                            )?;
                            c.alert.position = Some(actor.position);
                            c.emit(
                                actor,
                                Event::SoundPosition {
                                    id: 3,
                                    position: actor.position,
                                },
                                random,
                            )?;
                        } else {
                            c.emit(
                                actor,
                                Event::SoundVolume {
                                    id: 0x12,
                                    volume: 42,
                                },
                                random,
                            )?;
                        }
                    }
                    StandingCue::Jump => c.jump(actor, request, input.held, resources, random)?,
                    StandingCue::Attack => c.start_attack(actor, request, input.held, resources)?,
                    StandingCue::Item => c.item(
                        actor,
                        request,
                        input.held,
                        input.script != 0,
                        resources,
                        random,
                    )?,
                    StandingCue::Recover => c.recover(actor, request, input.held)?,
                    StandingCue::Ledge => {
                        return actor.enter_ledge(
                            request,
                            &mut ledge,
                            &mut self.terrain,
                            resources,
                            |a, r, held, cue| c.ledge_cue(a, r, held, cue, random),
                        );
                    }
                    StandingCue::HiddenMission => {
                        c.controls.story_triggered = true;
                        c.emit(actor, Event::HiddenMission, random)?;
                    }
                    StandingCue::HiddenCarry => {
                        c.controls.story_triggered = true;
                        c.emit(actor, Event::HiddenCarry, random)?;
                    }
                }
                Ok(false)
            },
        )?;
        c.save_ledge(ledge, input);
        Ok(())
    }

    pub(super) fn crouched(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let body_id = c.nearest(actor, 1500, false, resources);
        let searching = actor.motion.action == 0x200
            && actor
                .motion
                .clip
                .and_then(|id| resources.clips.get(&id))
                .is_some_and(|clip| actor.motion.frame == 1i16.wrapping_sub(clip.blend));
        let mut body = body_id
            .filter(|_| searching)
            .and_then(|id| c.others.get(id, actor))
            .filter(|a| {
                a.health == 0 && a.motion.loops == i16::MIN && a.inventory.resurrections == 0
            })
            .cloned();
        let unlocked = c.controls.unlocked_items;
        actor.update_crouched(
            request,
            &mut CrouchInput {
                held: input.held,
                body: body.as_mut(),
                unlocked: &unlocked,
                wall_allowed: c.controls.wall_allowed,
            },
            &mut self.terrain,
            resources,
            |actor, request, held, cue| {
                match cue {
                    CrouchCue::Root => actor.commit_root(c.root(actor)?),
                    CrouchCue::Recover => c.recover(actor, request, held)?,
                    CrouchCue::Attack => c.start_attack(actor, request, held, resources)?,
                    CrouchCue::Carry => c.carry(actor, request, resources, self.random)?,
                    CrouchCue::Item => c.item(
                        actor,
                        request,
                        held,
                        input.script != 0,
                        resources,
                        self.random,
                    )?,
                    CrouchCue::Voice(id) => c.emit(actor, Event::Voice(id), self.random)?,
                    CrouchCue::Sound(id) => c.emit(actor, Event::Sound(id), self.random)?,
                    CrouchCue::Splash(id) => c.emit(actor, Event::Splash(id), self.random)?,
                    CrouchCue::Pickup(id) => c.emit(actor, Event::Pickup(id), self.random)?,
                }
                Ok(())
            },
        )?;
        if let (Some(id), Some(body)) = (body_id, body) {
            c.others.get_mut(id)?.actor.inventory = body.inventory;
        }
        Ok(())
    }

    pub(super) fn wall(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &mut Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let mut wall = WallInput {
            held: input.held,
            player: input.player,
            allowed: c.controls.wall_allowed,
            camera: input.camera,
        };
        let mut thrown = None;
        actor.update_wall(
            request,
            &mut wall,
            &mut self.terrain,
            resources,
            |actor, request, held, resources, cue| {
                match cue {
                    WallCue::PublishRoot => c.publish_root(actor, resources),
                    WallCue::Recover => c.recover(actor, request, held)?,
                    WallCue::Voice(id) => c.emit(actor, Event::Voice(id), self.random)?,
                    WallCue::Sound(sound) => c.emit(actor, Event::ItemSound(sound), self.random)?,
                    WallCue::Throw { item, direction } => thrown = Some((item, direction)),
                }
                Ok(())
            },
        )?;
        if let Some((item, direction)) = thrown {
            c.use_item(actor, item, Some(direction), &mut self.terrain, self.random)?;
        }
        input.held = wall.held;
        input.camera = wall.camera;
        Ok(())
    }

    pub(super) fn ledge(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
        enter: bool,
    ) -> Result<bool> {
        let c = &mut self.context;
        let mut ledge = c.ledge_input(input);
        let mut callback =
            |a: &mut Actor, r: &mut Request, held, cue| c.ledge_cue(a, r, held, cue, self.random);
        let grabbed = if enter {
            actor.enter_ledge(
                request,
                &mut ledge,
                &mut self.terrain,
                resources,
                &mut callback,
            )?
        } else {
            actor.update_ledge(
                request,
                &mut ledge,
                &mut self.terrain,
                resources,
                &mut callback,
            )?;
            false
        };
        c.save_ledge(ledge, input);
        Ok(grabbed)
    }

    pub(super) fn jumping(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let effects = actor.update_jump(
            request,
            input.held,
            &mut self.terrain,
            resources,
            |actor, request| c.start_attack(actor, request, input.held, resources),
        )?;
        c.jump_effects(actor, effects, self.random)
    }

    pub(super) fn special(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let mut special = SpecialInput {
            held: input.held,
            player: input.player,
            player_dead: c.others.get(c.player, actor).is_none_or(|a| a.health == 0),
            difficulty: i16::from(c.difficulty),
            water: input.water.depth,
            camera_actor: input.camera_actor,
            camera: input.camera,
        };
        actor.update_special(
            request,
            &mut special,
            self.terrain.rules,
            resources,
            self.random,
            |actor, request, random, cue| {
                match cue {
                    SpecialCue::Weapon(mode) => c.weapon(actor, mode, resources)?,
                    SpecialCue::Voice(id) => c.emit(actor, Event::Voice(id), random)?,
                    SpecialCue::Attack => c.start_attack(actor, request, input.held, resources)?,
                    SpecialCue::Recover => c.recover(actor, request, input.held)?,
                    SpecialCue::AlignToFloor => {
                        actor.align_to_floor(&mut self.terrain, &resources.math, |actor| {
                            c.root(actor)
                        })?;
                    }
                    SpecialCue::ShowHealth => c.emit(actor, Event::ShowHealth, random)?,
                    SpecialCue::Landing => c.emit(actor, Event::Landing, random)?,
                    SpecialCue::Dust => c.emit(actor, Event::Dust, random)?,
                    SpecialCue::Splash => c.emit(actor, Event::Splash(1), random)?,
                    SpecialCue::Throw {
                        bone,
                        center,
                        direction,
                    } => c.emit(
                        actor,
                        Event::Throw {
                            bone,
                            center,
                            direction,
                        },
                        random,
                    )?,
                }
                Ok(())
            },
        )?;
        input.camera = special.camera;
        Ok(())
    }
}

impl<E: Events> Context<'_, E> {
    pub(super) fn jump(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        held: u16,
        resources: &Resources,
        random: &mut Random,
    ) -> Result<()> {
        let effects = actor.jump_input(
            request,
            held,
            resources,
            self.index == self.player,
            |actor| self.root(actor),
        )?;
        self.jump_effects(actor, effects, random)
    }

    fn jump_effects(
        &mut self,
        actor: &Actor,
        effects: JumpEffects,
        random: &mut Random,
    ) -> Result<()> {
        for id in effects.voices {
            self.emit(actor, Event::Voice(id), random)?;
        }
        if effects.splash {
            self.emit(actor, Event::Splash(1), random)?;
        }
        if effects.wall_jump {
            self.emit(actor, Event::WallJump, random)?;
        }
        Ok(())
    }

    fn ledge_input(&self, input: &ControlInput) -> LedgeInput {
        LedgeInput {
            held: input.held,
            player: input.player,
            tick: input.tick,
            check: self.controls.ledge_check,
            grapple_camera: input.camera == 19,
            map: self.map as i16,
            story_triggered: self.controls.story_triggered,
        }
    }

    fn save_ledge(&mut self, ledge: LedgeInput, input: &mut ControlInput) {
        input.held = ledge.held;
        self.controls.ledge_check = ledge.check;
        self.controls.story_triggered = ledge.story_triggered;
        if !ledge.grapple_camera && input.camera == 19 {
            input.camera = 0;
        }
    }

    fn ledge_cue(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        held: u16,
        cue: LedgeCue,
        random: &mut Random,
    ) -> Result<()> {
        match cue {
            LedgeCue::Voice(id) => self.emit(actor, Event::Voice(id), random),
            LedgeCue::Recover => self.recover(actor, request, held),
            LedgeCue::HiddenMission => self.emit(actor, Event::HiddenMission, random),
            LedgeCue::Grabbed => self.emit(
                actor,
                Event::Rumble {
                    kind: 2,
                    strength: 1,
                },
                random,
            ),
            LedgeCue::ReleaseCamera => Ok(()), // The controller updates grapple_camera synchronously.
        }
    }

    fn carry(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        resources: &Resources,
        random: &mut Random,
    ) -> Result<()> {
        if self.index != self.player {
            return Ok(());
        }
        let Some(id) = self.nearest(actor, 1500, false, resources) else {
            return Ok(());
        };
        let body = &mut self.others.get_mut(id)?.actor;
        if actor.can_carry(body, &self.instances[id].rig, self.scene, true) {
            actor.target = Some(id);
            body.target = Some(self.index);
            request.action = 0x120;
            request.velocity = Velocity::Motion;
            self.emit(
                actor,
                Event::Rumble {
                    kind: 2,
                    strength: 1,
                },
                random,
            )?;
        }
        Ok(())
    }
}
