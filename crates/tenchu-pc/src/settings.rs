mod blood;
mod edit;
pub mod menu;
pub mod overlay;
pub use blood::{Blood, Style as BloodStyle};
mod sky;
pub use sky::Sky;
mod weather;
pub use weather::{Weather, WeatherMode};
mod water;
pub use water::Water;
mod models;
pub use models::Models;
mod effects;
pub use effects::Effects;
mod motion;
pub use motion::Motion;
pub mod gameplay;
pub mod library;
mod post;
pub mod presets;
pub use post::Post;
mod vibration;
pub use vibration::Vibration;
mod navigation;
pub use navigation::Navigation;

use serde::{Deserialize, Serialize};
mod store;
pub use store::Store;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub show_fps: bool,
    pub show_performance: bool,
    pub pause_on_defocus: bool,
    pub pause_in_menu: bool,
    pub wait_after_death: bool,
    pub keep_enemies_active: bool,
    pub reinforcement_limit: u16,
    pub sight_percent: u16,
    pub retro: bool,
    pub fullscreen: bool,
    pub vsync: bool,
    pub show_hud: bool,
    pub sharp_text: bool,
    pub volume: u16,
    pub vibration: Vibration,
    pub navigation: Navigation,
    pub graphics: Graphics,
    pub cutscene_speed: u16,
    pub player_model: String,
    pub model_moveset: bool,
    pub model_moveset_options: tenchu_core::game::moveset::Options,
    pub model_directory: String,
    pub frontend: crate::frontend::Preferences,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_fps: false,
            show_performance: false,
            pause_on_defocus: true,
            pause_in_menu: true,
            wait_after_death: false,
            keep_enemies_active: false,
            reinforcement_limit: 0,
            sight_percent: 100,
            retro: false,
            fullscreen: false,
            vsync: true,
            show_hud: true,
            sharp_text: false,
            volume: 100,
            vibration: Vibration::default(),
            navigation: Navigation::default(),
            graphics: Graphics::default(),
            cutscene_speed: 1,
            player_model: String::new(),
            model_moveset: false,
            model_moveset_options: Default::default(),
            model_directory: String::new(),
            frontend: crate::frontend::Preferences::default(),
        }
    }
}

impl Settings {
    fn validate(&mut self) {
        self.volume = self.volume.min(100);
        self.vibration.validate();
        self.navigation.validate();
        self.sight_percent = self.sight_percent.min(400);
        self.reinforcement_limit = self.reinforcement_limit.min(1000);
        if ![0, 1, 2, 4, 8, 16].contains(&self.cutscene_speed) {
            self.cutscene_speed = 1;
        }
        self.graphics.validate();
    }

    pub fn rendering(&self) -> std::borrow::Cow<'_, Self> {
        if !self.retro {
            return std::borrow::Cow::Borrowed(self);
        }
        let mut original = self.clone();
        original.retro = false;
        original.graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        original.sharp_text = false;
        original.player_model.clear();
        std::borrow::Cow::Owned(original)
    }

    pub fn apply_activation(&self, roster: &mut tenchu_core::roster::Roster) {
        roster.keep_all_active = self.keep_enemies_active;
        roster.reinforcement_limit =
            (self.reinforcement_limit != 0).then_some(usize::from(self.reinforcement_limit));
    }

    pub fn set_visual_preset(&mut self, index: u16) {
        self.retro = false;
        self.graphics.set_preset(index);
        self.sharp_text = presets::sharp_text(index);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Graphics {
    pub blood: Blood,
    pub shop_lighting: bool,
    pub sky: Sky,
    pub weather: Weather,
    pub water: Water,
    pub models: Models,
    pub motion: Motion,
    pub effects: Effects,
    pub interpolation: bool,
    pub fog: bool,
    pub fog_distance: u16,
    pub textures: bool,
    pub shading: bool,
    pub exposure: u16,
    pub bilinear: bool,
    pub mipmaps: bool,
    pub anisotropy: u16,
    pub magnification: u16,
    pub surface_detail: u16,
    pub surface_strength: u16,
    pub asset_pack: bool,
    pub upscale: u16,
    pub smooth_models: bool,
    pub model_detail: u16,
    pub samples: u32,
    pub lighting: Lighting,
    pub blob_shadows: bool,
    pub post: Post,
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            blood: Blood::default(),
            shop_lighting: false,
            sky: Sky::default(),
            weather: Weather::default(),
            water: Water::default(),
            models: Models::default(),
            motion: Motion::default(),
            effects: Effects::default(),
            interpolation: false,
            fog: true,
            fog_distance: 100,
            textures: true,
            shading: true,
            exposure: 100,
            bilinear: false,
            mipmaps: false,
            anisotropy: 1,
            magnification: 0,
            surface_detail: 0,
            surface_strength: 50,
            asset_pack: true,
            upscale: 1,
            smooth_models: false,
            model_detail: 0,
            samples: 1,
            lighting: Lighting::default(),
            blob_shadows: true,
            post: Post::default(),
        }
    }
}

impl Graphics {
    fn validate(&mut self) {
        self.fog_distance = self.fog_distance.clamp(25, 400);
        self.exposure = self.exposure.clamp(25, 300);
        if ![1, 2, 4, 8, 16].contains(&self.anisotropy) {
            self.anisotropy = 1;
        }
        if ![1, 2, 4].contains(&self.upscale) {
            self.upscale = 1;
        }
        self.model_detail = self.model_detail.min(2);
        self.magnification = self.magnification.min(3);
        self.surface_detail = self.surface_detail.min(2);
        self.surface_strength = self.surface_strength.min(100);
        self.post.validate();
        self.blood.validate();
        self.sky.validate();
        self.weather.validate();
        self.water.validate();
        self.models.validate();
        self.motion.validate();
        self.effects.validate();
        if ![1, 2, 4, 8, 16].contains(&self.samples) {
            self.samples = 1;
        }
        self.lighting.validate();
    }

    pub fn animated_presentation(&self) -> bool {
        self.interpolation
            || self.sky.enabled && self.sky.speed != 0
            || self.weather.mode.precipitates() && self.weather.amount != 0
            || self.water.enabled && self.water.flow != 0
            || self.motion.actors()
            || self.motion.wind != 0
            || self.effects.active()
    }

    pub fn filtering(&self) -> u16 {
        if self.mipmaps {
            2 + self.anisotropy.ilog2() as u16
        } else {
            u16::from(self.bilinear)
        }
    }

    pub fn set_filtering(&mut self, level: u16) {
        self.bilinear = level != 0;
        self.mipmaps = level >= 2;
        self.anisotropy = 1 << level.saturating_sub(2).min(4);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lighting {
    pub enabled: bool,
    pub shadows: bool,
    pub fires: bool,
    pub radius: u16,
    pub intensity: u16,
    pub ambient: u16,
    pub resolution: u32,
    pub count: u16,
    pub shadow_count: u16,
    pub lamps: bool,
    pub markers: bool,
    pub softness: u16,
    pub source_size: u16,
    pub sky_tint: u16,
    pub directional: Directional,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            enabled: false,
            shadows: true,
            fires: true,
            radius: 12,
            intensity: 150,
            ambient: 60,
            resolution: 512,
            count: 2,
            shadow_count: 2,
            lamps: true,
            markers: false,
            softness: 0,
            source_size: 100,
            sky_tint: 0,
            directional: Directional::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Directional {
    pub intensity: u16,
    pub color: u16,
    pub azimuth: u16,
    pub elevation: u16,
    pub radius: u16,
    pub resolution: u32,
}

impl Default for Directional {
    fn default() -> Self {
        Self {
            intensity: 0,
            color: 1,
            azimuth: 315,
            elevation: 45,
            radius: 30,
            resolution: 2048,
        }
    }
}

impl Lighting {
    fn validate(&mut self) {
        self.radius = self.radius.clamp(2, 50);
        self.intensity = self.intensity.min(400);
        self.ambient = self.ambient.min(100);
        self.softness = self.softness.min(8);
        self.source_size = self.source_size.min(400);
        self.sky_tint = self.sky_tint.min(100);
        self.directional.validate();
        self.count = self.count.min(crate::lighting::MAX_LIGHTS as u16);
        self.shadow_count = self
            .shadow_count
            .min(crate::lighting::MAX_SHADOW_LIGHTS as u16);
        if ![256, 512, 1024, 2048].contains(&self.resolution) {
            self.resolution = 512;
        }
    }
}

impl Directional {
    fn validate(&mut self) {
        self.intensity = self.intensity.min(200);
        self.color = self.color.min(2);
        self.azimuth %= 360;
        self.elevation = self.elevation.clamp(10, 85);
        self.radius = self.radius.clamp(10, 60);
        if ![512, 1024, 2048, 4096].contains(&self.resolution) {
            self.resolution = 2048;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn retro_is_a_rendering_view_and_restores_all_preferences() {
        let mut settings = Settings {
            keep_enemies_active: true,
            sight_percent: 125,
            cutscene_speed: 4,
            volume: 25,
            player_model: "pack:custom".into(),
            model_moveset: true,
            model_moveset_options: tenchu_core::game::moveset::Options {
                player_movement: true,
                no_idle_taunts: true,
            },
            ..Default::default()
        };
        settings.set_visual_preset(presets::MODERN_HIGH);
        settings.graphics.weather.mode = WeatherMode::Snow;
        let selected = settings.clone();
        settings.retro = true;
        let original = settings.rendering();
        assert_eq!(
            original.graphics,
            Graphics {
                asset_pack: false,
                ..Default::default()
            }
        );
        assert_eq!(original.sight_percent, 125);
        assert!(original.keep_enemies_active);
        assert_eq!(original.cutscene_speed, 4);
        assert_eq!(original.volume, 25);
        assert!(original.player_model.is_empty());
        assert_eq!(
            original.model_moveset_options,
            settings.model_moveset_options
        );
        let stored = serde_json::to_vec(&settings).unwrap();
        settings = serde_json::from_slice(&stored).unwrap();
        settings.retro = false;
        assert_eq!(settings, selected);
        assert!(matches!(
            settings.rendering(),
            std::borrow::Cow::Borrowed(_)
        ));
        settings.set_visual_preset(presets::MODERN_ULTRA);
        assert_eq!(settings.graphics.weather.mode, WeatherMode::Snow);
        assert_eq!(settings.sight_percent, 125);
    }

    #[test]
    fn settings_upgrade_and_atomic_replacement_do_not_touch_campaign_saves() {
        let root = std::env::temp_dir().join(format!("tenchu-settings-{}", std::process::id()));
        let store = Store {
            path: root.join("config/settings.json"),
        };
        let mut initial = Settings::default();
        initial.set_visual_preset(presets::MODERN_LOW);
        assert_eq!(store.load().unwrap(), initial);
        let settings = Settings {
            show_fps: true,
            pause_on_defocus: false,
            volume: 25,
            ..Settings::default()
        };
        store.write(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
        fs::write(&store.path, br#"{"show_fps":true,"volume":200,"future":1}"#).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.show_fps && loaded.pause_on_defocus);
        assert_eq!(loaded.volume, 100);
        assert_eq!(loaded.graphics.lighting, Lighting::default());
        fs::write(&store.path, br#"{"cutscene_speed":7,"graphics":{"lighting":{"enabled":true,"count":99,"radius":0,"intensity":65535,"ambient":200,"resolution":99999}}}"#).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.cutscene_speed, 1);
        assert_eq!(
            loaded.graphics.lighting,
            Lighting {
                enabled: true,
                count: crate::lighting::MAX_LIGHTS as u16,
                radius: 2,
                intensity: 400,
                ambient: 100,
                ..Lighting::default()
            }
        );
        fs::write(&store.path, b"{").unwrap();
        assert!(store.load().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
