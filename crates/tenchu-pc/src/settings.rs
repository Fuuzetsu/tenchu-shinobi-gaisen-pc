pub mod menu;

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
        fs::write(&store.path, b"{").unwrap();
        assert!(store.load().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
