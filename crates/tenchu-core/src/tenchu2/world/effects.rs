use super::Instance;
use crate::{
    Result,
    math::Math,
    random::Random,
    scene::Scene,
    tenchu2::{
        collision::Collision,
        effects::Effects,
        environment::{Event, Events, Shot},
        items::{Items, State, cloud::Cloud},
    },
};

pub(super) struct EnvironmentEvents<'a, E> {
    pub effects: &'a mut Effects,
    pub math: &'a Math,
    pub rules: &'a tenchu_assets::tenchu2::effects::Rules,
    pub instances: &'a [Instance],
    pub events: &'a mut E,
    pub spawned: Vec<State>,
}

impl<E> EnvironmentEvents<'_, E> {
    pub fn publish_items(
        &mut self,
        items: &mut Items,
        scene: &mut Scene,
        collision: &mut Collision,
    ) -> Result<()> {
        for state in self.spawned.drain(..) {
            items.insert(state, scene, collision)?;
        }
        Ok(())
    }
}

impl<E: Events> Events for EnvironmentEvents<'_, E> {
    fn emit(&mut self, event: Event, random: &mut Random) -> Result<()> {
        match event {
            Event::Shoot {
                kind: Shot::Smoke,
                position,
                velocity,
            } => {
                self.spawned.push(State::Cloud(Cloud {
                    owner: None,
                    position,
                    direction: velocity,
                    age: 0,
                }));
                Ok(())
            }
            Event::Explosion { position, small } => {
                self.effects
                    .explosion(position, u8::from(small), self.math, random)?;
                self.events.emit(Event::Sound { position, id: 51 }, random)
            }
            Event::Burn {
                actor,
                interval,
                duration,
            } => self.effects.burn(
                &self
                    .instances
                    .get(actor)
                    .ok_or("Missing Tenchu 2 burning actor")?
                    .rig,
                interval,
                duration,
                self.rules,
            ),
            _ => self.events.emit(event, random),
        }
    }
}
