use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub increment: u16,
    pub maximum: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            increment: 0,
            maximum: 100,
        }
    }
}

impl Settings {
    pub fn enabled(self) -> bool {
        self.increment != 0 && self.maximum != 0
    }

    pub fn validate(&mut self) {
        self.increment = self.increment.min(100);
        self.maximum = self.maximum.min(400);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Encounters {
    pub detections: u16,
    ready: bool,
}

impl Default for Encounters {
    fn default() -> Self {
        Self {
            detections: 0,
            ready: true,
        }
    }
}

impl Encounters {
    pub(super) fn prepare(&mut self, enabled: bool, calm: bool) {
        if !enabled {
            *self = Self::default();
        } else if calm {
            self.ready = true;
        }
    }

    pub(super) fn see_player(&mut self) {
        if self.ready {
            self.detections = self.detections.saturating_add(1);
            self.ready = false;
        }
    }

    pub fn bonus(self, settings: Settings) -> u16 {
        self.detections
            .saturating_sub(1)
            .saturating_mul(settings.increment.min(100))
            .min(settings.maximum.min(400))
    }

    pub(crate) fn damage(self, damage: i16, settings: Settings) -> i16 {
        if damage <= 0 {
            return damage;
        }
        (i32::from(damage) * (100 + i32::from(self.bonus(settings))) / 100).min(i32::from(i16::MAX))
            as i16
    }
}

impl crate::game::Game {
    pub fn set_damage_escalation(&mut self, mut settings: Settings) {
        settings.validate();
        if self.intelligence.damage_escalation.enabled() != settings.enabled() {
            for actor in self.roster.actors.iter_mut().flatten() {
                actor.encounters = Encounters::default();
            }
        }
        self.intelligence.damage_escalation = settings;
    }
}
