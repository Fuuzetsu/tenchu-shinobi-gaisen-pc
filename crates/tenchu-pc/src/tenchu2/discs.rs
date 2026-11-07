use std::path::{Path, PathBuf};
use tenchu_assets::{Result, tenchu2::Edition};

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Discs {
    selected: Option<PathBuf>,
    entries: Vec<(Edition, PathBuf)>,
}

impl Discs {
    fn read() -> Self {
        let mut discs = crate::storage::UserFile::Tenchu2
            .path()
            .ok()
            .and_then(|p| crate::storage::read(&p).ok())
            .map(|bytes| {
                serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                    let mut legacy = Self::default();
                    if let Ok(path) = serde_json::from_slice::<PathBuf>(&bytes) {
                        legacy.include(path);
                    }
                    legacy
                })
            })
            .unwrap_or_default();
        #[cfg(target_os = "android")]
        if let Some(path) = crate::android::data_dir()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("tenchu2-path.txt")).ok())
            && !discs
                .entries
                .iter()
                .any(|(_, p)| p == Path::new(path.trim()))
        {
            discs.include(path.trim().into());
        }
        // A moved/missing disc stays remembered, so reconnecting it restores the library.
        discs.entries.truncate(2);
        discs
    }

    fn include(&mut self, path: PathBuf) {
        if !self.entries.iter().any(|(_, p)| *p == path)
            && let Ok(edition) = crate::source::open_disc(&path).and_then(|d| Edition::detect(&d))
        {
            self.entries.retain(|(old, _)| *old != edition);
            self.entries.push((edition, path.clone()));
        }
        self.selected = Some(path);
    }
}

pub fn disc_path() -> Option<PathBuf> {
    Discs::read().selected
}

pub fn known_discs() -> Vec<(Edition, PathBuf)> {
    Discs::read().entries
}

pub fn remember(path: &Path) -> Result<()> {
    let edition = Edition::detect(&crate::source::open_disc(path)?)?;
    let mut discs = Discs::read();
    discs.entries.retain(|(old, _)| *old != edition);
    discs.entries.push((edition, path.into()));
    discs.selected = Some(path.into());
    crate::storage::atomic_write(
        &crate::storage::UserFile::Tenchu2.path()?,
        &serde_json::to_vec(&discs).map_err(|e| e.to_string())?,
    )
}
