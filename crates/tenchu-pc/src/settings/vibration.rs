use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_core::effects::RumbleAction;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Vibration {
    pub intensity: u16,
    pub actions: BTreeMap<RumbleAction, u16>,
}

impl Default for Vibration {
    fn default() -> Self {
        Self {
            intensity: 100,
            actions: BTreeMap::new(),
        }
    }
}

impl Vibration {
    pub fn validate(&mut self) {
        self.intensity = self.intensity.min(200);
        for value in self.actions.values_mut() {
            *value = (*value).min(200);
        }
    }

    pub fn intensity(&self, action: RumbleAction) -> u16 {
        self.actions.get(&action).copied().unwrap_or(100)
    }

    pub fn gain(&self, action: RumbleAction) -> f32 {
        f32::from(self.intensity) * f32::from(self.intensity(action)) / 10_000.0
    }
}

pub fn description(action: RumbleAction) -> (&'static str, &'static str) {
    match action {
        RumbleAction::Attack => ("Weapon swing", "Start of an attack's active swing."),
        RumbleAction::WeaponClash => ("Weapon clash", "Your weapon collides with another weapon."),
        RumbleAction::Hit => ("Landing a hit", "Your attack connects with a person."),
        RumbleAction::HitBlocked => ("Attack blocked", "An opponent blocks your attack."),
        RumbleAction::Block => ("Blocking an attack", "You block an incoming attack."),
        RumbleAction::Damage => ("Melee damage", "You are hit by another actor's attack."),
        RumbleAction::ItemDamage => (
            "Item / trap damage",
            "You are hurt by an item, projectile, fire or trap.",
        ),
        RumbleAction::TargetHit => (
            "Animal / target hit",
            "Your attack connects with an animal or training target.",
        ),
        RumbleAction::Knockdown => (
            "Body slam / hard fall",
            "Ground impact after being knocked airborne or a damaging fall.",
        ),
        RumbleAction::StealthKill => (
            "Stealth kill",
            "Authored impact pulses during a stealth kill.",
        ),
        RumbleAction::StealthInterrupted => (
            "Uneven-ground stealth kill",
            "Impact when a stealth kill ends early because the actors are at different heights.",
        ),
        RumbleAction::JumpCollision => (
            "Jump collision",
            "A jump is interrupted by terrain contact above the floor.",
        ),
        RumbleAction::Landing => (
            "Normal landing",
            "Landing from a normal jump or short fall.",
        ),
        RumbleAction::HeavyLanding => ("Heavy landing", "Landing hard without taking fall damage."),
        RumbleAction::FlipLanding => ("Flip landing", "Landing from a flip."),
        RumbleAction::LedgeGrab => ("Ledge grab", "Catching a ledge."),
        RumbleAction::WaterEntry => ("Entering water", "Entering swimming or drowning depth."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, menu::Item, presets};

    #[test]
    fn vibration_preferences_migrate_scale_and_reset_independently() {
        let mut settings: Settings =
            serde_json::from_str(r#"{"frontend":{"rumble":false}}"#).unwrap();
        assert!(!settings.frontend.rumble);
        for action in RumbleAction::ALL {
            assert_eq!(settings.vibration.gain(action), 1.0);
            assert!(Item::VibrationAction(action).change_setting(&mut settings, -10));
            assert_eq!(settings.vibration.gain(action), 0.5);
        }
        Item::VibrationIntensity.change_setting(&mut settings, -10);
        assert_eq!(settings.vibration.gain(RumbleAction::Attack), 0.25);
        Item::VibrationAction(RumbleAction::Landing).change_setting(&mut settings, -100);
        assert_eq!(settings.vibration.gain(RumbleAction::Landing), 0.0);
        Item::VibrationAction(RumbleAction::Attack).reset_setting(&mut settings);
        assert_eq!(settings.vibration.gain(RumbleAction::Attack), 0.5);
        let encoded = serde_json::to_string(&settings).unwrap();
        let mut restored: Settings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, settings);
        restored.set_visual_preset(presets::MODERN_ULTRA);
        restored.retro = true;
        assert_eq!(restored.rendering().vibration, settings.vibration);
        assert!(!restored.frontend.rumble);
        restored.vibration.intensity = u16::MAX;
        restored
            .vibration
            .actions
            .insert(RumbleAction::Attack, u16::MAX);
        restored.validate();
        assert_eq!(restored.vibration.gain(RumbleAction::Attack), 4.0);
        assert_eq!(restored.vibration.gain(RumbleAction::Landing), 0.0);
    }
}
