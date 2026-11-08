use super::{
    Assets, World,
    controls::{self, Event, EventSource},
};
use crate::{
    Result,
    tenchu2::{actor::Actor, ai::behavior, physics::FallDeath, terrain::Probe},
};

pub trait ActorEvents: controls::Events {
    fn surface(&mut self, actor: usize, probe: &Probe) -> i16;
    fn visible(&mut self, source: EventSource<'_>) -> Result<bool>;
}

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
        events: &mut impl ActorEvents,
    ) -> Result<()> {
        let mut voices = Voices::default();
        for index in 0..self.actors.len() {
            if self.actors[index].actor.flags & 8 != 0 {
                continue;
            }
            let movement = self.move_actor(index, assets, |probe| events.surface(index, probe))?;
            self.terrain_options.avoid_water = false;
            if let Some(death) = movement.death {
                if index == self.player && death == FallDeath::LongFall {
                    self.controls.camera = 0x13;
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
            let visible = index == self.player || events.visible(self.event_source(index))?;
            self.animate_actor(index, visible, assets)?;
        }
        Ok(())
    }

    fn event_source(&mut self, index: usize) -> EventSource<'_> {
        EventSource {
            index,
            actor: &self.actors[index].actor,
            rig: &self.instances[index].rig,
            scene: &mut self.scene,
        }
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
