use crate::{actor::Actor, random::Random};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub chance: u16,
    pub seconds: u16,
    pub damage: u16,
    pub bandages: u16,
    pub medicine: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            chance: 25,
            seconds: 10,
            damage: 1,
            bandages: 3,
            medicine: true,
        }
    }
}

impl Settings {
    pub fn validate(&mut self) {
        self.chance = self.chance.min(100);
        self.seconds = self.seconds.min(120);
        self.damage = self.damage.min(20);
        self.bandages = self.bandages.min(99);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub settings: Settings,
    pub bandages: u16,
    pub active: bool,
    pub remaining: u32,
    phase: u8,
    random: Random,
    held: bool,
    requested: bool,
}

impl State {
    pub(crate) fn configure(state: &mut Option<Self>, mut settings: Settings, seed: u32) {
        settings.validate();
        if !settings.enabled {
            *state = None;
        } else if let Some(state) = state {
            if settings.seconds != state.settings.seconds && state.active {
                state.remaining = u32::from(settings.seconds) * 30;
            }
            state.settings = settings;
        } else {
            *state = Some(Self::new(settings, seed));
        }
    }

    fn new(settings: Settings, seed: u32) -> Self {
        Self {
            settings,
            bandages: settings.bandages,
            active: false,
            remaining: 0,
            phase: 0,
            random: Random::new(seed ^ 0x626c_6564),
            held: false,
            requested: false,
        }
    }

    pub fn valid(&self) -> bool {
        let mut settings = self.settings;
        settings.validate();
        settings == self.settings
            && settings.enabled
            && self.bandages <= 99
            && self.phase < 30
            && self.remaining <= 3600
            && (self.active || self.remaining == 0 && self.phase == 0)
    }

    pub fn input(&mut self, held: bool) {
        self.requested |= held && !self.held;
        self.held = held;
    }

    pub fn request_bandage(&mut self) {
        self.requested = true;
    }

    pub fn bandage_requested(&self) -> bool {
        self.requested
    }

    pub(crate) fn use_bandage(&mut self) -> bool {
        if !self.active || self.bandages == 0 {
            return false;
        }
        self.bandages -= 1;
        self.clear();
        true
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.remaining = 0;
        self.phase = 0;
    }

    pub(crate) fn hit(&mut self) {
        if self.settings.chance == 0
            || self.random.next15() % 100 >= i32::from(self.settings.chance)
        {
            return;
        }
        if !self.active {
            self.phase = 0;
        }
        self.active = true;
        self.remaining = u32::from(self.settings.seconds) * 30;
    }

    pub(crate) fn tick(
        &mut self,
        health: &mut i16,
        can_treat: bool,
        paused: bool,
        invulnerable: bool,
    ) {
        let requested = std::mem::take(&mut self.requested);
        if *health <= 0 {
            self.clear();
            return;
        }
        if paused {
            return;
        }
        if requested && can_treat {
            self.use_bandage();
        }
        if !self.active {
            return;
        }
        self.phase += 1;
        if self.phase == 30 {
            self.phase = 0;
            if !invulnerable {
                *health = (*health - self.settings.damage as i16).max(1);
            }
        }
        if self.settings.seconds != 0 {
            self.remaining = self.remaining.saturating_sub(1);
            if self.remaining == 0 {
                self.clear();
            }
        }
    }
}

impl Actor {
    fn can_bandage(&self) -> bool {
        self.health > 0
            && self.free_stance()
            && self.map.height <= 0
            && !self.scripted
            && !self.aiming
    }

    pub(crate) fn use_bandage(&mut self) -> bool {
        self.can_bandage() && self.bleeding.as_mut().is_some_and(State::use_bandage)
    }

    pub(crate) fn bleeding_tick(&mut self, paused: bool) {
        let can_treat = self.can_bandage();
        let Some(state) = &mut self.bleeding else {
            return;
        };
        state.tick(
            &mut self.health,
            can_treat,
            paused || self.scripted,
            self.invulnerable,
        );
    }
}

impl super::Game {
    pub fn set_bleeding(&mut self, settings: Settings) {
        State::configure(
            &mut self.roster.get_mut(self.player_id).bleeding,
            settings,
            self.random.seed,
        );
    }
}
