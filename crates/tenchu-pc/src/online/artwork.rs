use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Artwork {
    Gaisen,
    Tenchu2,
}

#[derive(Deserialize)]
pub struct Release {
    pub tag: String,
    pub archive: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub sha256: String,
    pub bytes: u64,
}

impl Release {
    pub fn url(&self) -> String {
        format!(
            "{}/releases/download/{}/{}",
            super::REPOSITORY,
            self.tag,
            self.archive
        )
    }
}

impl Artwork {
    pub const ALL: [Self; 2] = [Self::Gaisen, Self::Tenchu2];

    pub fn name(self) -> &'static str {
        match self {
            Self::Gaisen => "Tenchu 1 HD artwork",
            Self::Tenchu2 => "Tenchu 2 HD artwork",
        }
    }

    pub fn release(self) -> &'static Release {
        static GAISEN: LazyLock<Release> = LazyLock::new(|| {
            serde_json::from_str(include_str!("../../../../artwork/release.json"))
                .expect("bundled artwork metadata")
        });
        static TENCHU2: LazyLock<Release> = LazyLock::new(|| {
            serde_json::from_str(include_str!("../../../../artwork/tenchu2/release.json"))
                .expect("bundled artwork metadata")
        });
        match self {
            Self::Gaisen => &GAISEN,
            Self::Tenchu2 => &TENCHU2,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn directory(self) -> tenchu_assets::Result<std::path::PathBuf> {
        Ok(crate::storage::UserFile::Artwork.path()?.join(match self {
            Self::Gaisen => "tenchu",
            Self::Tenchu2 => "tenchu2",
        }))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn installed(self) -> Option<std::path::PathBuf> {
        installed_at(&self.directory().ok()?)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn installed_at(directory: &std::path::Path) -> Option<std::path::PathBuf> {
    let bytes = std::fs::read(directory.join("current.json")).ok()?;
    let name: String = serde_json::from_slice(&bytes).ok()?;
    crate::visuals::pack::validate_path(&name).ok()?;
    let path = directory.join(name);
    path.is_file().then_some(path)
}
