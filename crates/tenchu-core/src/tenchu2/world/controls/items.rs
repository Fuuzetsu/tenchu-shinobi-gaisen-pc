use super::*;

impl<E: Events> Driver<'_, E> {
    pub(super) fn item_control(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let mut item = ItemControlInput {
            held: input.held,
            player: input.player,
            camera_actor: input.camera_actor,
            camera: input.camera,
            previous_camera: c.controls.previous_camera,
            camera_yaw: c.camera.direction[1],
            aim: c.items.aim.direction,
            target_position: actor
                .target
                .and_then(|id| c.others.get(id, actor))
                .map(|a| a.position),
            grapple: GrappleStatus::Failed,
            grapple_position: Position::default(),
            difficulty: i16::from(c.difficulty),
            map: c.map as i16,
            story_triggered: c.controls.story_triggered,
        };
        actor.update_item(
            request,
            &mut item,
            resources,
            self.random,
            |actor, request, input, random, cue| match cue {
                ItemCue::Recover => c.recover(actor, request, input.held),
                ItemCue::Voice(id) => c.emit(actor, Event::Voice(id), random),
                ItemCue::Sound { id, positional } => {
                    c.emit(actor, Event::SoundAt { id, positional }, random)
                }
                ItemCue::PublishRoot => {
                    c.publish_root(actor, resources);
                    Ok(())
                }
                ItemCue::HeldModel(id) => c.emit(actor, Event::HeldItem(id), random),
                ItemCue::LaunchGrapple(direction) => c.launch_grapple(actor, direction, random),
                ItemCue::UpdateGrapple => {
                    input.grapple = c.grapple_status();
                    Ok(())
                }
                ItemCue::ReleaseGrapple => {
                    if let Some(grapple) = &mut c.items.grapple {
                        grapple.release();
                    }
                    Ok(())
                }
                ItemCue::GrapplePosition => {
                    input.grapple_position = c
                        .items
                        .grapple
                        .as_ref()
                        .ok_or("Missing Tenchu 2 grapple")?
                        .origin;
                    Ok(())
                }
                ItemCue::Use { item, direction } => {
                    c.emit(actor, Event::UseItem { item, direction }, random)
                }
                ItemCue::Medicine(effect) => c.emit(actor, Event::Medicine(effect), random),
                ItemCue::ShowHealth => c.emit(actor, Event::ShowHealth, random),
            },
        )?;
        input.camera = item.camera;
        c.controls.previous_camera = item.previous_camera;
        c.camera.direction[1] = item.camera_yaw;
        c.items.aim.direction = item.aim;
        c.controls.story_triggered = item.story_triggered;
        Ok(())
    }

    pub(super) fn swimming(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        let mut swimming = SwimInput {
            held: input.held,
            tick: input.tick,
            camera_actor: input.camera_actor,
            camera: input.camera,
            previous_camera: c.controls.previous_camera,
            aim: c.items.aim.direction,
            grapple: GrappleStatus::Failed,
            model_height: input.model_height,
        };
        actor.update_swimming(
            request,
            &mut swimming,
            &mut self.terrain,
            resources,
            self.random,
            |actor, request, input, random, cue| match cue {
                SwimCue::Recover => c.recover(actor, request, input.held),
                SwimCue::CommitRoot => {
                    actor.commit_root(c.root(actor)?);
                    Ok(())
                }
                SwimCue::Submerge => {
                    actor.position = c.root(actor)?;
                    Ok(())
                }
                SwimCue::PublishRoot => {
                    c.publish_root(actor, resources);
                    Ok(())
                }
                SwimCue::Jump => c.jump(actor, request, input.held, resources, random),
                SwimCue::Attack => c.start_attack(actor, request, input.held, resources),
                SwimCue::Voice(id) => c.emit(actor, Event::Voice(id), random),
                SwimCue::Splash(id) => c.emit(actor, Event::Splash(id), random),
                SwimCue::Weapon(mode) => c.weapon(actor, mode, resources),
                SwimCue::Grip { slot, spare, grip } => c.grip(actor, slot, spare, grip, random),
                SwimCue::EmptyItem => c.emit(actor, Event::Sound(6), random),
                SwimCue::LaunchGrapple => c.launch_grapple(actor, input.aim, random),
                SwimCue::UpdateGrapple => {
                    input.grapple = c.grapple_status();
                    Ok(())
                }
                SwimCue::BreathingTube(show) => {
                    c.emit(actor, Event::HeldItem(show.then_some(23)), random)
                }
                SwimCue::UseUnderwaterItem => c.emit(actor, Event::Bubbles, random),
            },
        )?;
        input.camera = swimming.camera;
        c.controls.previous_camera = swimming.previous_camera;
        c.items.aim.direction = swimming.aim;
        Ok(())
    }
}

impl<E: Events> Context<'_, E> {
    fn grapple_status(&self) -> GrappleStatus {
        self.items
            .grapple
            .as_ref()
            .map_or(GrappleStatus::Failed, |grapple| grapple.status())
    }

    fn launch_grapple(
        &mut self,
        actor: &Actor,
        direction: Position,
        random: &mut Random,
    ) -> Result<()> {
        if self.items.grapple.is_some() {
            return Ok(());
        }
        let joint = *self.instances[self.index]
            .rig
            .joints
            .get(1)
            .ok_or("Missing Tenchu 2 grapple joint")?;
        let position = self.scene.world(joint)?.translation;
        self.items.grapple = crate::tenchu2::items::grapple::Grapple::launch(
            self.index,
            position,
            direction,
            &mut self.items.aim,
            &self.math,
        );
        if self.items.grapple.is_none() {
            self.emit(actor, Event::SoundPosition { id: 6, position }, random)?;
        }
        Ok(())
    }
}
