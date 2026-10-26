mod blood;
mod edit;
pub mod menu;
pub mod overlay;
pub use blood::Blood;
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
mod post;
pub mod presets;
pub use post::Post;

use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::fs;
use std::path::PathBuf;
use tenchu_assets::Result;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub show_fps: bool,
    pub show_performance: bool,
    pub pause_on_defocus: bool,
    pub pause_in_menu: bool,
    pub keep_enemies_active: bool,
    pub sight_percent: u16,
    pub retro: bool,
    pub fullscreen: bool,
    pub vsync: bool,
    pub show_hud: bool,
    pub sharp_text: bool,
    pub volume: u16,
    pub graphics: Graphics,
    pub cutscene_speed: u16,
    pub player_model: String,
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
            keep_enemies_active: false,
            sight_percent: 100,
            retro: false,
            fullscreen: false,
            vsync: true,
            show_hud: true,
            sharp_text: false,
            volume: 100,
            graphics: Graphics::default(),
            cutscene_speed: 1,
            player_model: String::new(),
            model_directory: String::new(),
            frontend: crate::frontend::Preferences::default(),
        }
    }
}

impl Settings {
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

    pub fn set_visual_preset(&mut self, index: u16) {
        self.retro = false;
        self.graphics.set_preset(index);
        if (presets::MODERN_HIGH..presets::CUSTOM).contains(&index) {
            self.sharp_text = true;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Graphics {
    pub blood: Blood,
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

pub struct Store {
    pub path: PathBuf,
}

impl Store {
    pub fn new() -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        return Ok(Self {
            path: "settings.json".into(),
        });
        #[cfg(not(target_arch = "wasm32"))]
        Self::native()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native() -> Result<Self> {
        let path = if let Some(path) = std::env::var_os("TENCHU_SETTINGS") {
            PathBuf::from(path)
        } else {
            let root = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
                PathBuf::from(path)
            } else if cfg!(target_os = "windows") {
                PathBuf::from(std::env::var_os("APPDATA").ok_or("Set APPDATA or TENCHU_SETTINGS")?)
            } else {
                PathBuf::from(std::env::var_os("HOME").ok_or("Set HOME or TENCHU_SETTINGS")?).join(
                    if cfg!(target_os = "macos") {
                        "Library/Application Support"
                    } else {
                        ".config"
                    },
                )
            };
            root.join("tenchu-shinobi-gaisen-pc/settings.json")
        };
        Ok(Self { path })
    }

    pub fn load(&self) -> Result<Settings> {
        match crate::storage::read(&self.path) {
            Ok(bytes) => {
                let mut settings: Settings = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("{}: {e}", self.path.display()))?;
                settings.volume = settings.volume.min(100);
                settings.sight_percent = settings.sight_percent.min(400);
                if ![0, 1, 2, 4, 8, 16].contains(&settings.cutscene_speed) {
                    settings.cutscene_speed = 1;
                }
                settings.graphics.fog_distance = settings.graphics.fog_distance.clamp(25, 400);
                settings.graphics.exposure = settings.graphics.exposure.clamp(25, 300);
                if ![1, 2, 4, 8, 16].contains(&settings.graphics.anisotropy) {
                    settings.graphics.anisotropy = 1;
                }
                if ![1, 2, 4].contains(&settings.graphics.upscale) {
                    settings.graphics.upscale = 1;
                }
                settings.graphics.model_detail = settings.graphics.model_detail.min(2);
                settings.graphics.magnification = settings.graphics.magnification.min(3);
                settings.graphics.surface_detail = settings.graphics.surface_detail.min(2);
                settings.graphics.surface_strength = settings.graphics.surface_strength.min(100);
                settings.graphics.post.validate();
                settings.graphics.blood.validate();
                settings.graphics.sky.validate();
                settings.graphics.weather.validate();
                settings.graphics.water.validate();
                settings.graphics.models.validate();
                settings.graphics.motion.validate();
                settings.graphics.effects.validate();
                if ![1, 2, 4, 8, 16].contains(&settings.graphics.samples) {
                    settings.graphics.samples = 1;
                }
                let light = &mut settings.graphics.lighting;
                light.radius = light.radius.clamp(2, 50);
                light.intensity = light.intensity.min(400);
                light.ambient = light.ambient.min(100);
                light.softness = light.softness.min(8);
                light.source_size = light.source_size.min(400);
                light.sky_tint = light.sky_tint.min(100);
                let sun = &mut light.directional;
                sun.intensity = sun.intensity.min(200);
                sun.color = sun.color.min(2);
                sun.azimuth %= 360;
                sun.elevation = sun.elevation.clamp(10, 85);
                sun.radius = sun.radius.clamp(10, 60);
                if ![512, 1024, 2048, 4096].contains(&sun.resolution) {
                    sun.resolution = 2048;
                }
                light.count = light.count.min(crate::lighting::MAX_LIGHTS as u16);
                light.shadow_count = light
                    .shadow_count
                    .min(crate::lighting::MAX_SHADOW_LIGHTS as u16);
                if ![256, 512, 1024, 2048].contains(&light.resolution) {
                    light.resolution = 512;
                }
                Ok(settings)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(format!("{}: {error}", self.path.display())),
        }
    }

    pub fn write(&self, settings: &Settings) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
        crate::storage::atomic_write(&self.path, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retro_is_a_rendering_view_and_restores_all_preferences() {
        let mut settings = Settings {
            keep_enemies_active: true,
            sight_percent: 125,
            cutscene_speed: 4,
            volume: 25,
            player_model: "pack:custom".into(),
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
        assert_eq!(store.load().unwrap(), Settings::default());
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
