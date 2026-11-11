use super::{
    Assets, World,
    controls::{self, Event, EventSource},
};
use crate::{
    Result,
    tenchu2::{actor::Actor, ai::behavior, physics::FallDeath},
};

#[derive(Default)]
struct Voices(Vec<u8>);

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    pub scenery: crate::tenchu2::environment::scenery::Frame,
    pub effects: crate::tenchu2::effects::Frame,
    pub reticle: Option<crate::tenchu2::items::aim::Reticle>,
    pub fade: Option<u8>,
    pub transition_fade: Option<u8>,
    pub ambience: Option<crate::tenchu2::audio::Sound>,
}

impl behavior::Events for Voices {
    fn voice(&mut self, _: &Actor, voice: u8) -> Result<()> {
        self.0.push(voice);
        Ok(())
    }
}

impl World {
    pub fn step(
        &mut self,
        controllers: [u16; 2],
        assets: &mut Assets,
        events: &mut impl controls::Events,
    ) -> Result<Frame> {
        let frame = self.begin_recorded_update(controllers, assets, events)?;
        self.finish_tick(assets, 6)?;
        Ok(frame)
    }

    /// Stops at GAME.EXE's display boundary for recorded retail comparisons.
    pub fn begin_recorded_update(
        &mut self,
        controllers: [u16; 2],
        assets: &mut Assets,
        events: &mut impl controls::Events,
    ) -> Result<Frame> {
        self.bleeding_tick();
        let ambience = self.audio.ambient(
            self.map,
            assets.ambient_interval,
            self.tick,
            &mut self.random,
            &assets.resources.math,
        );
        let controllers = if self.mission.input_allowed() {
            controllers
        } else {
            [0; 2]
        };
        self.update_camera(assets, false)?;
        self.update_actors(controllers, assets, events)?;
        self.sample_weapon_ribbons(assets)?;
        let scenery = self.update_environment(assets, events)?;
        let mut items = if self.mission.items_running() {
            self.update_items(assets, events)?
        } else {
            Default::default()
        };
        if self.mission.items_running() {
            self.sync_bandages();
            let player = &self.actors[self.player].actor;
            self.controls.hud.step(
                player.into(),
                player.breath,
                &player.inventory,
                self.ai.global.radar,
                |index| self.actors.get(index).map(|a| (&a.actor).into()),
            )?;
            self.controls.hud.aim(items.reticle);
        } else {
            self.controls.hud.frame.clear();
        }
        let mut effects = self.update_effects(assets)?;
        items.quads.append(&mut effects.quads);
        effects.quads = items.quads;
        Ok(Frame {
            scenery,
            effects,
            reticle: items.reticle,
            fade: items.fade,
            transition_fade: self.mission.fade,
            ambience,
        })
    }

    pub fn listener(&self) -> crate::tenchu2::audio::Listener {
        let actor = &self.actors[self.controls.camera_actor].actor;
        crate::tenchu2::audio::Listener {
            actor: self.controls.camera_actor,
            position: actor.position,
            yaw: actor.rotation.y,
            distance: self.view_distance >> 4,
        }
    }

    pub fn update_items(
        &mut self,
        assets: &Assets,
        events: &mut impl controls::Events,
    ) -> Result<crate::tenchu2::items::Frame> {
        use crate::tenchu2::items::aim::Input;
        let math = &assets.resources.math;
        let mut frame = crate::tenchu2::items::Frame::default();
        self.advance_items(assets, events, &mut frame)?;
        let root = self
            .scene
            .world(self.instances[self.player].rig.joints[0])?
            .translation;
        frame.reticle = self.items.aim.update(
            Input {
                eye: self.camera.eye,
                look: self.camera.look,
                root,
                camera: self.camera.mode,
                grapple: !self.controls.hud.bandage_selected()
                    && self.actors[self.player].actor.inventory.selected
                        == Some(crate::tenchu2::inventory::Item::GRAPPLE),
            },
            &self.terrain,
            &mut self.cursor,
            math,
        )?;
        Ok(frame)
    }

    pub fn update_effects(&mut self, assets: &Assets) -> Result<crate::tenchu2::effects::Frame> {
        let view = self.view();
        let frame = self.effects.update(
            &mut self.scene,
            view,
            &self.terrain,
            &mut self.cursor,
            &assets.resources.math,
        )?;
        if self.random.next15() & 15 == 0 {
            self.effects.ambient_ripple(
                self.actors[self.player].actor.position,
                view,
                &self.terrain,
                &mut self.cursor,
                &mut self.random,
            )?;
        }
        Ok(frame)
    }

    pub fn update_actors(
        &mut self,
        controllers: [u16; 2],
        assets: &mut Assets,
        events: &mut impl controls::Events,
    ) -> Result<()> {
        let mut voices = Voices::default();
        for index in 0..self.actors.len() {
            if self.actors[index].actor.flags & 8 != 0 {
                continue;
            }
            let movement = self.move_actor(index, assets, events)?;
            self.terrain_options.avoid_water = false;
            if let Some(death) = movement.death {
                if index == self.player && death == FallDeath::LongFall {
                    self.camera.mode = 0x13;
                }
                controls::credit(
                    &mut self.statistics,
                    &mut self.script,
                    &self.actors[index].actor,
                    index == self.player,
                    false,
                    assets.hero == 2 && self.map >= 12,
                );
                let event = if index == self.player && self.controls.death_music {
                    Event::DeathMusic
                } else {
                    Event::Voice(15)
                };
                self.emit(index, event, events)?;
            }
            for voice in movement.voices {
                self.emit(index, Event::Voice(voice), events)?;
            }
            self.decide_actor(index, controllers, assets, &mut voices)?;
            for voice in voices.0.drain(..) {
                self.emit(index, Event::Voice(voice), events)?;
            }
            self.control_actor(index, assets, events)?;
            self.animate_actor(index, assets)?;
        }
        Ok(())
    }

    pub(super) fn emit(
        &mut self,
        index: usize,
        event: Event,
        events: &mut impl controls::Events,
    ) -> Result<()> {
        self.controls
            .observe(index, &self.actors[index].actor, event);
        controls::Events::emit(
            events,
            EventSource {
                listener: self.listener(),
                voice: self.instances[index].voice,
                index,
                actor: &self.actors[index].actor,
                rig: &self.instances[index].rig,
                scene: &mut self.scene,
            },
            event,
            &mut self.random,
        )
    }
}

impl World {
    pub fn update_environment(
        &mut self,
        assets: &Assets,
        events: &mut impl crate::tenchu2::environment::Events,
    ) -> Result<crate::tenchu2::environment::scenery::Frame> {
        use crate::tenchu2::ai::status::{Alert, AlertCause};
        use crate::tenchu2::environment::{Context, Event};
        struct Sink<'a, E> {
            alert: &'a mut Alert,
            difficulty: u8,
            rules: &'a tenchu_assets::tenchu2::combat::Rules,
            events: &'a mut E,
        }
        impl<E: crate::tenchu2::environment::Events> crate::tenchu2::environment::Events for Sink<'_, E> {
            fn emit(&mut self, event: Event, random: &mut crate::random::Random) -> Result<()> {
                if let Event::Alert { position, kind } = event {
                    let cause = match kind {
                        1 => AlertCause::Suspicion,
                        2 => AlertCause::Corpse,
                        _ => AlertCause::Combat,
                    };
                    self.alert.raise(cause, self.difficulty, self.rules)?;
                    self.alert.position = Some(position);
                    Ok(())
                } else {
                    self.events.emit(event, random)
                }
            }
        }
        let mut view = self.view();
        view.distance = assets.draw_distance;
        let mut events = super::effects::EnvironmentEvents {
            effects: &mut self.effects,
            math: &assets.resources.math,
            rules: &assets.effect_rules,
            instances: &self.instances,
            events,
            spawned: Vec::new(),
        };
        let mut sink = Sink {
            alert: &mut self.ai.global.perception.alert,
            difficulty: self.difficulty,
            rules: &assets.combat_rules,
            events: &mut events,
        };
        let mut context = Context {
            map: &mut self.terrain,
            cursor: &mut self.cursor,
            collision: &mut self.collision,
            scene: &mut self.scene,
            math: &assets.resources.math,
            random: &mut self.random,
            events: &mut sink,
        };
        let frame = self.environment.update_scenery(
            &crate::tenchu2::environment::scenery::Input {
                level: &assets.level,
                sprites: &assets.sprites,
                view,
                sequence: self.sequence,
                vblank: self.tick.wrapping_mul(2) as u16,
                actors: &self.actors,
            },
            &mut context,
        )?;
        if !self.environment.background_only {
            self.environment.update_contacts(
                &assets.level.scene.bounds,
                (self.player, self.actors[self.player].actor.position),
                |object| assets.level.grid.contains(object.cell),
                &mut context,
            )?;
            self.environment.update_triggers(&mut context)?;
        }
        events.publish_items(&mut self.items, &mut self.scene, &mut self.collision)?;
        Ok(frame)
    }
}
