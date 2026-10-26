use super::Settings;
use std::path::PathBuf;
use tenchu_assets::Result;

pub struct Store {
    pub path: PathBuf,
}

impl Store {
    pub fn new() -> Result<Self> {
        Ok(Self {
            path: crate::storage::UserFile::Settings.path()?,
        })
    }

    pub fn load(&self) -> Result<Settings> {
        match crate::storage::read(&self.path) {
            Ok(bytes) => {
                let mut settings: Settings = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("{}: {e}", self.path.display()))?;
                settings.validate();
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
