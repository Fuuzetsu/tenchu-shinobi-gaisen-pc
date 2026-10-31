use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Off,
    Original,
    Modern,
}

impl Style {
    pub const ALL: [Self; 3] = [Self::Off, Self::Original, Self::Modern];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Original => "Original",
            Self::Modern => "Modern",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let index = Self::ALL.iter().position(|&s| s == self).unwrap() as i32;
        Self::ALL[(index + delta).rem_euclid(3) as usize]
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
    pub model_stains: bool,
    pub amount: u16,
    pub seconds: u16,
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
            model_stains: false,
            amount: 100,
            seconds: 3,
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
            ..Self::default()
        }
    }

    pub fn retained_ground(self) -> bool {
        self.amount != 0
            && (self.ground == Style::Modern
                || self.ground == Style::Original && self.retain_original)
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

    pub fn active(self) -> bool {
        self.amount != 0
            && (self.retained_ground()
                || self.custom_death(false)
                || self.custom_death(true)
                || self.model_stains
                || self.drips != Style::Off && self.seconds != 0)
    }

    pub fn validate(&mut self) {
        self.amount = self.amount.min(200);
        self.seconds = self.seconds.min(60);
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
    model_stains: Option<bool>,
    amount: Option<u16>,
    seconds: Option<u16>,
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
            model_stains: saved.model_stains.unwrap_or(default.model_stains),
            amount: saved.amount.unwrap_or(default.amount),
            seconds: saved.seconds.unwrap_or(default.seconds),
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
            }
        }
        let blood = Blood {
            hit: Style::Off,
            drips: Style::Original,
            ground: Style::Original,
            death: Style::Modern,
            retain_original: true,
            player_death: false,
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
