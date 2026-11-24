use super::Settings;
use tenchu_core::game::sheathing::{Settings as Sheathing, UnarmedStyle};

impl Settings {
    pub fn sheathing_for(&self, hero: usize) -> Sheathing {
        Sheathing {
            unarmed_style: self
                .hero_unarmed_styles
                .get(hero)
                .copied()
                .flatten()
                .unwrap_or(self.sheathing.unarmed_style),
            ..self.sheathing
        }
    }
}

pub(crate) fn cycle(style: Option<UnarmedStyle>, delta: i32) -> Option<UnarmedStyle> {
    let index = style.map_or(0, |style| {
        UnarmedStyle::ALL.iter().position(|&s| s == style).unwrap() + 1
    });
    let index = (index as i32 + delta).rem_euclid(UnarmedStyle::ALL.len() as i32 + 1);
    (index != 0).then(|| UnarmedStyle::ALL[index as usize - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_style_is_the_default_until_each_hero_is_overridden() {
        let mut saved = serde_json::to_value(Settings::default()).unwrap();
        saved.as_object_mut().unwrap().remove("hero_unarmed_styles");
        saved["sheathing"]["unarmed_style"] =
            serde_json::to_value(UnarmedStyle::Tatsumaru).unwrap();
        let mut settings: Settings = serde_json::from_value(saved).unwrap();
        assert_eq!(settings.hero_unarmed_styles, [None; 3]);
        for hero in 0..3 {
            assert_eq!(
                settings.sheathing_for(hero).unarmed_style,
                UnarmedStyle::Tatsumaru
            );
        }
        settings.hero_unarmed_styles[1] = Some(UnarmedStyle::Onikage);
        settings.sheathing.unarmed_style = UnarmedStyle::WangXiaohai;
        let restored: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(restored, settings);
        assert_eq!(
            restored.sheathing_for(0).unarmed_style,
            UnarmedStyle::WangXiaohai
        );
        assert_eq!(
            restored.sheathing_for(1).unarmed_style,
            UnarmedStyle::Onikage
        );
        assert_eq!(
            restored.sheathing_for(2).unarmed_style,
            UnarmedStyle::WangXiaohai
        );
        let mut current = Settings::default();
        current.restore_playback_preferences(&restored);
        assert_eq!(current.hero_unarmed_styles, restored.hero_unarmed_styles);
    }
}
