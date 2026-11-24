use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Models {
    pub heroes: bool,
    pub rikimaru_sheath: bool,
    pub stow_unarmed_kills: bool,
    pub characters: u16,
    pub foliage: u16,
    pub variation: bool,
    pub grapple: bool,
}

impl Models {
    pub fn validate(&mut self) {
        self.characters = self.characters.min(2);
        self.foliage = self.foliage.min(4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_model_preferences_keep_source_weapon_presentation() {
        let old: Models =
            serde_json::from_str(r#"{"characters":2,"rikimaru_sheath":true}"#).unwrap();
        assert!(!old.stow_unarmed_kills);
        assert!(old.rikimaru_sheath);
        let settings = Models {
            stow_unarmed_kills: true,
            ..old
        };
        assert_eq!(
            serde_json::from_slice::<Models>(&serde_json::to_vec(&settings).unwrap()).unwrap(),
            settings
        );
        let mut graphics = crate::settings::Graphics::default();
        let original = crate::visuals::models::Options::from(&graphics);
        graphics.models.stow_unarmed_kills = true;
        assert!(original != crate::visuals::models::Options::from(&graphics));
        for preset in [
            0,
            crate::settings::presets::MODERN_HIGH,
            crate::settings::presets::MODERN_ULTRA,
        ] {
            graphics.set_preset(preset);
            assert!(!graphics.models.stow_unarmed_kills);
        }
    }
}
