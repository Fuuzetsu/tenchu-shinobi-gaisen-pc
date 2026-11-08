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
            aim: c.controls.aim,
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
                ItemCue::LaunchGrapple(direction) => {
                    c.emit(actor, Event::LaunchGrapple(direction), random)
                }
                ItemCue::UpdateGrapple => {
                    c.emit(actor, Event::UpdateGrapple, random)?;
                    input.grapple = c.events.grapple()?.0;
                    Ok(())
                }
                ItemCue::ReleaseGrapple => c.emit(actor, Event::ReleaseGrapple, random),
                ItemCue::GrapplePosition => {
                    input.grapple_position = c.events.grapple()?.1;
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
        c.controls.aim = item.aim;
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
            aim: c.controls.aim,
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
                SwimCue::LaunchGrapple => c.emit(actor, Event::LaunchGrapple(input.aim), random),
                SwimCue::UpdateGrapple => {
                    c.emit(actor, Event::UpdateGrapple, random)?;
                    input.grapple = c.events.grapple()?.0;
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
        c.controls.aim = swimming.aim;
        Ok(())
    }
}
