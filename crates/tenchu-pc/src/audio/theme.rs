use crate::level_editor::custom::{Asset, FILE_LIMIT, Kind};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use tenchu_assets::Result;

const VERSION: u32 = 1;
const STORE_LIMIT: usize = FILE_LIMIT.div_ceil(3) * 4 + 4096;

#[cfg(test)]
mod tests;

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Theme {
    pub enabled: bool,
    pub asset: Option<Arc<Asset>>,
}

#[derive(Serialize, Deserialize)]
struct Stored<T> {
    version: u32,
    fallback: Option<usize>,
    enabled: bool,
    asset: Option<T>,
}

impl Theme {
    pub fn selected(&self) -> Option<Arc<Asset>> {
        self.enabled.then(|| self.asset.clone()).flatten()
    }

    #[cfg(any(test, not(target_arch = "wasm32")))]
    pub fn imported(name: String, bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            enabled: true,
            asset: Some(Arc::new(Asset::import(name, Kind::Audio, bytes)?)),
        })
    }

    pub fn load(path: &Path) -> (Self, Option<usize>, Option<String>) {
        let bytes = match read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return (Self::default(), None, None);
            }
            Err(error) => {
                return (
                    Self::default(),
                    None,
                    Some(format!(
                        "Custom theme unavailable; using stage ambience: {error}"
                    )),
                );
            }
        };
        let stored: Stored<serde_json::Value> = match serde_json::from_slice(&bytes) {
            Ok(stored) => stored,
            Err(error) => {
                return (
                    Self::default(),
                    None,
                    Some(format!(
                        "Custom theme preferences are invalid; using stage ambience: {error}"
                    )),
                );
            }
        };
        let fallback = stored
            .fallback
            .filter(|&i| i < tenchu_assets::tenchu2::music::THEMES.len());
        let load = || -> Result<Self> {
            if stored.version != VERSION || stored.fallback != fallback {
                return Err("Unsupported custom theme preferences".into());
            }
            let asset = stored
                .asset
                .map(serde_json::from_value::<Asset>)
                .transpose()
                .map_err(|e| e.to_string())?
                .map(Arc::new);
            let theme = Self {
                enabled: stored.enabled,
                asset,
            };
            theme.validate()?;
            Ok(theme)
        };
        match load() {
            Ok(theme) => (theme, fallback, None),
            Err(error) => (
                Self::default(),
                fallback,
                Some(format!(
                    "Custom theme unavailable; using the disc selection: {error}"
                )),
            ),
        }
    }

    pub fn save(&self, path: &Path, fallback: Option<usize>) -> Result<()> {
        self.validate()?;
        if fallback.is_some_and(|i| i >= tenchu_assets::tenchu2::music::THEMES.len()) {
            return Err("Unknown sandbox disc theme".into());
        }
        let bytes = serde_json::to_vec(&Stored {
            version: VERSION,
            fallback,
            enabled: self.enabled,
            asset: self.asset.as_ref(),
        })
        .map_err(|e| e.to_string())?;
        if bytes.len() > STORE_LIMIT {
            return Err("Custom theme preferences are too large".into());
        }
        crate::storage::atomic_write(path, &bytes)
    }

    fn validate(&self) -> Result<()> {
        if self.enabled && self.asset.is_none() {
            return Err("The selected custom theme is missing".into());
        }
        if let Some(asset) = &self.asset {
            validate(asset)?;
        }
        Ok(())
    }
}

pub(crate) fn validate(asset: &Asset) -> Result<()> {
    if asset.kind != Kind::Audio || asset.name.is_empty() || asset.name.len() > 512 {
        return Err("Invalid custom audio asset".into());
    }
    asset.clip().map(|_| ())
}

#[cfg(not(target_arch = "wasm32"))]
fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(STORE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > STORE_LIMIT {
        return Err(std::io::Error::other(
            "Custom theme preferences are too large",
        ));
    }
    Ok(bytes)
}

#[cfg(target_arch = "wasm32")]
fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    let bytes = crate::storage::read(path)?;
    if bytes.len() > STORE_LIMIT {
        return Err(std::io::Error::other(
            "Custom theme preferences are too large",
        ));
    }
    Ok(bytes)
}
