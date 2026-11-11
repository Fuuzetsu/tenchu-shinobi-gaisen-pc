use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(in crate::tenchu2::native::render) struct State {
    weather: Weather,
    footprints: Footprints,
    tick: Option<u64>,
    thunder: Option<tenchu_audio::ThunderCue>,
    contact: crate::visuals::contact::Contact,
    blood: crate::visuals::blood::Blood,
    blood_pools_changed: bool,
    blood_serial: usize,
    player_pool_age: Option<u64>,
}

impl Environment {
    pub fn capture_state(&self) -> State {
        let Self {
            caster: _,
            stale: _,
            surfaces: _,
            weather,
            footprints,
            tick,
            thunder,
            spray: _,
            contact,
            blood,
            blood_pools_changed,
            blood_serial,
            player_pool_age,
        } = self;
        State {
            weather: weather.clone(),
            footprints: footprints.clone(),
            tick: *tick,
            thunder: *thunder,
            contact: contact.clone(),
            blood: blood.clone(),
            blood_pools_changed: *blood_pools_changed,
            blood_serial: *blood_serial,
            player_pool_age: *player_pool_age,
        }
    }

    pub fn restore_state(&mut self, state: State) {
        self.weather = state.weather;
        self.footprints = state.footprints;
        self.tick = state.tick;
        self.thunder = state.thunder;
        self.contact = state.contact;
        self.blood = state.blood;
        self.blood_pools_changed = state.blood_pools_changed;
        self.blood_serial = state.blood_serial;
        self.player_pool_age = state.player_pool_age;
    }
}
