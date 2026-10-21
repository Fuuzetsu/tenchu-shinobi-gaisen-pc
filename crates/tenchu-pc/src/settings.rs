pub mod menu;
pub mod overlay;
mod post;
pub mod presets;
pub use post::Post;

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tenchu_assets::Result;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub show_fps: bool,
    pub pause_on_defocus: bool,
    pub fullscreen: bool,
    pub vsync: bool,
    pub show_hud: bool,
    pub volume: u16,
    pub graphics: Graphics,
    pub cutscene_speed: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_fps: false,
            pause_on_defocus: true,
            fullscreen: false,
            vsync: true,
            show_hud: true,
            volume: 100,
            graphics: Graphics::default(),
            cutscene_speed: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Graphics {
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
        match fs::read(&self.path) {
            Ok(bytes) => {
                let mut settings: Settings = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("{}: {e}", self.path.display()))?;
                settings.volume = settings.volume.min(100);
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
                settings.graphics.post.validate();
                if ![1, 2, 4, 8, 16].contains(&settings.graphics.samples) {
                    settings.graphics.samples = 1;
                }
                let light = &mut settings.graphics.lighting;
                light.radius = light.radius.clamp(2, 50);
                light.intensity = light.intensity.min(400);
                light.ambient = light.ambient.min(100);
                light.softness = light.softness.min(8);
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
