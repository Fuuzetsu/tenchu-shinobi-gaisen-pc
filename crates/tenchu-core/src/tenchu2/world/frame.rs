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

impl behavior::Events for Voices {
    fn voice(&mut self, _: &Actor, voice: u8) -> Result<()> {
        self.0.push(voice);
        Ok(())
    }
}

impl World {
    pub fn update_actors(
        &mut self,
        controllers: [u16; 2],
        assets: &mut Assets,
        events: &mut (impl controls::Events + crate::tenchu2::environment::Events),
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

    fn emit(
        &mut self,
        index: usize,
        event: Event,
        events: &mut impl controls::Events,
    ) -> Result<()> {
        events.emit(
            EventSource {
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
        let mut sink = Sink {
            alert: &mut self.ai.global.perception.alert,
            difficulty: self.difficulty,
            rules: &assets.combat_rules,
            events,
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
        if self.environment.background_only {
            return Ok(frame);
        }
        self.environment.update_contacts(
            &assets.level.scene.bounds,
            (self.player, self.actors[self.player].actor.position),
            |object| assets.level.grid.contains(object.cell),
            &mut context,
        )?;
        self.environment.update_triggers(&mut context)?;
        Ok(frame)
    }
}
