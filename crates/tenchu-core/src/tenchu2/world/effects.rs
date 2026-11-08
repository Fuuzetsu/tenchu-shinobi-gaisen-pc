use super::Instance;
use crate::{
    Result,
    math::Math,
    random::Random,
    tenchu2::{
        effects::Effects,
        environment::{Event, Events},
    },
};

pub(super) struct EnvironmentEvents<'a, E> {
    pub effects: &'a mut Effects,
    pub math: &'a Math,
    pub rules: &'a tenchu_assets::tenchu2::effects::Rules,
    pub instances: &'a [Instance],
    pub events: &'a mut E,
}

impl<E: Events> Events for EnvironmentEvents<'_, E> {
    fn emit(&mut self, event: Event, random: &mut Random) -> Result<()> {
        match event {
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
