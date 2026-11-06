use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Off,
    Original,
    OriginalHd,
    Modern,
}

impl Style {
    pub const ALL: [Self; 4] = [Self::Off, Self::Original, Self::OriginalHd, Self::Modern];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Original => "Original",
            Self::OriginalHd => "Original HD",
            Self::Modern => "Modern",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let index = Self::ALL.iter().position(|&s| s == self).unwrap() as i32;
        Self::ALL[(index + delta).rem_euclid(Self::ALL.len() as i32) as usize]
    }

    pub fn uses_original(self) -> bool {
        matches!(self, Self::Original | Self::OriginalHd)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "SavedBlood")]
pub struct Blood {
    pub hit: Style,
    pub drips: Style,
    pub death: Style,
    pub ground: Style,
    pub player_death: bool,
    pub retain_original: bool,
    pub conform_retained: bool,
    pub model_stains: bool,
    pub weapons: bool,
    pub weapon_metal_only: bool,
    pub weapon_coverage: u16,
    pub weapon_flow: u16,
    pub runoff: bool,
    pub amount: u16,
    pub death_size: u16,
    pub seconds: u16,
    pub spurts: u16,
    pub death_gush: u16,
    pub gush_seconds: u16,
    pub severed: u16,
    pub severed_seconds: u16,
    pub marks: u16,
}

impl Default for Blood {
    fn default() -> Self {
        Self {
            hit: Style::Original,
            drips: Style::Off,
            death: Style::Original,
            ground: Style::Original,
            player_death: false,
            retain_original: false,
            conform_retained: true,
            model_stains: false,
            weapons: false,
            weapon_metal_only: false,
            weapon_coverage: 100,
            weapon_flow: 0,
            runoff: false,
            amount: 100,
            death_size: 100,
            seconds: 3,
            spurts: 0,
            death_gush: 0,
            gush_seconds: 6,
            severed: 0,
            severed_seconds: 3,
            marks: 8192,
        }
    }
}

impl Blood {
    pub fn modern() -> Self {
        Self {
            hit: Style::Modern,
            drips: Style::Modern,
            death: Style::Modern,
            ground: Style::Modern,
            player_death: true,
            model_stains: true,
            weapons: true,
            runoff: true,
            severed: 100,
            ..Self::default()
        }
    }

    pub fn retained_ground(self) -> bool {
        self.amount != 0
            && (self.ground == Style::Modern || self.ground.uses_original() && self.retain_original)
    }

    pub fn custom_death(self, player: bool) -> bool {
        self.amount != 0
            && self.death != Style::Off
            && if player {
                self.player_death
            } else {
                self.death == Style::Modern || self.retain_original
            }
    }

    pub fn gush(self, player: bool) -> bool {
        self.amount != 0
            && self.death_gush != 0
            && self.gush_seconds != 0
            && self.death != Style::Off
            && (!player || self.player_death)
    }

    pub fn active(self) -> bool {
        self.amount != 0
            && (self.retained_ground()
                || self.custom_death(false)
                || self.custom_death(true)
                || self.gush(false)
                || self.severed != 0 && self.severed_seconds != 0
                || self.weapons
                || self.model_stains
                || self.drips != Style::Off && self.seconds != 0)
    }

    pub fn validate(&mut self) {
        self.amount = self.amount.min(200);
        self.death_size = self.death_size.clamp(10, 200);
        self.seconds = self.seconds.min(60);
        self.spurts = self.spurts.min(200);
        self.death_gush = self.death_gush.min(200);
        self.gush_seconds = self.gush_seconds.min(15);
        self.severed = self.severed.min(200);
        self.severed_seconds = self.severed_seconds.min(15);
        self.weapon_coverage = self.weapon_coverage.min(200);
        self.weapon_flow = self.weapon_flow.min(200);
        self.marks = self.marks.max(32);
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct SavedBlood {
    enabled: bool,
    spray: bool,
    hit: Option<Style>,
    drips: Option<Style>,
    death: Option<Style>,
    ground: Option<Style>,
    player_death: Option<bool>,
    retain_original: Option<bool>,
    conform_retained: Option<bool>,
    model_stains: Option<bool>,
    weapons: bool,
    weapon_metal_only: bool,
    weapon_coverage: Option<u16>,
    weapon_flow: Option<u16>,
    runoff: bool,
    amount: Option<u16>,
    death_size: Option<u16>,
    seconds: Option<u16>,
    spurts: Option<u16>,
    death_gush: Option<u16>,
    gush_seconds: Option<u16>,
    severed: Option<u16>,
    severed_seconds: Option<u16>,
    marks: Option<u16>,
}

impl From<SavedBlood> for Blood {
    fn from(saved: SavedBlood) -> Self {
        let default = if saved.enabled {
            Self::modern()
        } else {
            Self::default()
        };
        Self {
            hit: saved.hit.unwrap_or(if saved.spray {
                Style::Modern
            } else {
                Style::Original
            }),
            drips: saved.drips.unwrap_or(default.drips),
            death: saved.death.unwrap_or(default.death),
            ground: saved.ground.unwrap_or(default.ground),
            player_death: saved.player_death.unwrap_or(default.player_death),
            retain_original: saved.retain_original.unwrap_or(default.retain_original),
            conform_retained: saved.conform_retained.unwrap_or(default.conform_retained),
            model_stains: saved.model_stains.unwrap_or(default.model_stains),
            weapons: saved.weapons,
            weapon_metal_only: saved.weapon_metal_only,
            weapon_coverage: saved.weapon_coverage.unwrap_or(100),
            weapon_flow: saved.weapon_flow.unwrap_or_default(),
            runoff: saved.runoff,
            amount: saved.amount.unwrap_or(default.amount),
            death_size: saved.death_size.unwrap_or(default.death_size),
            seconds: saved.seconds.unwrap_or(default.seconds),
            spurts: saved.spurts.unwrap_or_default(),
            death_gush: saved.death_gush.unwrap_or_default(),
            gush_seconds: saved.gush_seconds.unwrap_or(default.gush_seconds),
            severed: saved.severed.unwrap_or(default.severed),
            severed_seconds: saved.severed_seconds.unwrap_or(default.severed_seconds),
            marks: saved.marks.unwrap_or(default.marks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_preferences_migrate_and_independent_choices_roundtrip() {
        for enabled in [false, true] {
            for spray in [false, true] {
                let old = serde_json::json!({"enabled": enabled, "spray": spray, "marks": 12000});
                let blood: Blood = serde_json::from_value(old).unwrap();
                assert_eq!(
                    blood.hit,
                    if spray {
                        Style::Modern
                    } else {
                        Style::Original
                    }
                );
                assert_eq!(blood.retained_ground(), enabled);
                assert_eq!(blood.player_death, enabled);
                assert_eq!(blood.model_stains, enabled);
                assert_eq!(blood.marks, 12000);
                assert!(!blood.runoff, "Existing saved preferences keep runoff off");
                assert_eq!(blood.spurts, 0);
                assert_eq!((blood.death_gush, blood.gush_seconds), (0, 6));
                assert_eq!((blood.weapon_coverage, blood.weapon_flow), (100, 0));
            }
        }
        let blood = Blood {
            hit: Style::Off,
            drips: Style::Original,
            ground: Style::Original,
            death: Style::OriginalHd,
            retain_original: true,
            player_death: false,
            death_size: 45,
            spurts: 120,
            death_gush: 140,
            gush_seconds: 9,
            weapon_coverage: 140,
            weapon_flow: 80,
            ..Blood::modern()
        };
        let encoded = serde_json::to_value(blood).unwrap();
        assert!(encoded.get("spray").is_none() && encoded.get("enabled").is_none());
        assert_eq!(serde_json::from_value::<Blood>(encoded).unwrap(), blood);
        assert!(blood.retained_ground() && blood.custom_death(false) && !blood.custom_death(true));
        assert_eq!(
            serde_json::from_str::<Blood>("{}").unwrap(),
            Blood::default()
        );
    }
}
