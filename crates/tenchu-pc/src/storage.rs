use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};
use tenchu_assets::Result;
use tenchu_core::progress::save::Save;

pub struct Storage {
    path: PathBuf,
}

pub enum UserFile {
    Progress,
    Settings,
    Presets,
    Controls,
    #[cfg(not(target_arch = "wasm32"))]
    LastDisc,
}

impl UserFile {
    pub fn path(self) -> Result<PathBuf> {
        #[cfg(target_arch = "wasm32")]
        return Ok(match self {
            Self::Progress => "progress.sav",
            Self::Settings => "settings.json",
            Self::Presets => "presets.json",
            Self::Controls => "controls.json",
        }
        .into());
        #[cfg(not(target_arch = "wasm32"))]
        self.native_path()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn native_path(self) -> Result<PathBuf> {
        let (name, override_env, xdg, fallback) = match self {
            Self::Progress => (
                "progress.sav",
                "TENCHU_SAVE",
                "XDG_DATA_HOME",
                ".local/share",
            ),
            Self::Settings => (
                "settings.json",
                "TENCHU_SETTINGS",
                "XDG_CONFIG_HOME",
                ".config",
            ),
            Self::Presets => (
                "presets.json",
                "TENCHU_PRESETS",
                "XDG_CONFIG_HOME",
                ".config",
            ),
            Self::Controls => (
                "controls.json",
                "TENCHU_CONTROLS",
                "XDG_CONFIG_HOME",
                ".config",
            ),
            Self::LastDisc => (
                "last-disc.json",
                "TENCHU_LAST_DISC",
                "XDG_CONFIG_HOME",
                ".config",
            ),
        };
        if let Some(path) = std::env::var_os(override_env) {
            return Ok(path.into());
        }
        let root = if let Some(path) = std::env::var_os(xdg) {
            PathBuf::from(path)
        } else if cfg!(target_os = "windows") {
            PathBuf::from(
                std::env::var_os("APPDATA")
                    .ok_or_else(|| format!("Set APPDATA or {override_env}"))?,
            )
        } else {
            PathBuf::from(
                std::env::var_os("HOME").ok_or_else(|| format!("Set HOME or {override_env}"))?,
            )
            .join(if cfg!(target_os = "macos") {
                "Library/Application Support"
            } else {
                fallback
            })
        };
        Ok(root.join("tenchu-shinobi-gaisen-pc").join(name))
    }
}

impl Storage {
    pub fn new() -> Result<Self> {
        Ok(Self {
            path: UserFile::Progress.path()?,
        })
    }

    pub fn load(&self) -> Result<Option<Save>> {
        match read(&self.path) {
            Ok(bytes) => Save::decode(&bytes)
                .map(Some)
                .map_err(|e| format!("{}: {e}", self.path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{}: {e}", self.path.display())),
        }
    }

    pub fn write(&self, save: &Save) -> Result<()> {
        atomic_write(&self.path, &save.encode())?;
        println!("Saved {}", self.path.display());
        Ok(())
    }
}

pub fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    return crate::web::read_saved(path);
    #[cfg(not(target_arch = "wasm32"))]
    fs::read(path)
}

#[cfg(target_arch = "wasm32")]
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    crate::web::write_saved(path, bytes)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let (temporary, mut file) = loop {
        let sequence = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let temporary = directory.join(format!(".tenchu-{}-{sequence}.tmp", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Save {}: {error}", path.display())),
        }
    };
    let result = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = result.and_then(|()| replace_file(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("Save {}: {e}", path.display()))
}

#[cfg(not(target_arch = "wasm32"))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    // Windows can briefly deny replacement while another writer finishes renaming.
    #[cfg(target_os = "windows")]
    for delay in [1, 2, 4, 8, 16] {
        match fs::rename(source, destination) {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            result => return result,
        }
    }
    fs::rename(source, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_a_complete_save_and_reports_corruption() {
        let directory = std::env::temp_dir().join(format!(
            "tenchu-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = Storage {
            path: directory.join("nested/progress.sav"),
        };
        assert!(storage.load().unwrap().is_none());
        let mut save = Save::default();
        storage.write(&save).unwrap();
        let sibling = storage.path.with_extension("tmp");
        fs::write(&sibling, b"unrelated file").unwrap();
        save.progress.stock[0][1] = 42;
        storage.write(&save).unwrap();
        assert_eq!(storage.load().unwrap().unwrap(), save);
        assert_eq!(fs::read(&sibling).unwrap(), b"unrelated file");

        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            for count in 0..8 {
                let barrier = &barrier;
                let path = &storage.path;
                let mut candidate = save.clone();
                candidate.progress.stock[0][1] = count;
                scope.spawn(move || {
                    barrier.wait();
                    atomic_write(path, &candidate.encode()).unwrap();
                });
            }
        });
        let loaded = storage.load().unwrap().unwrap();
        assert!(loaded.progress.stock[0][1] < 8);
        save.progress.stock[0][1] = loaded.progress.stock[0][1];
        assert_eq!(loaded, save);
        assert_eq!(fs::read(&sibling).unwrap(), b"unrelated file");

        let parent = storage.path.parent().unwrap();
        assert!(atomic_write(parent, b"cannot replace a directory").is_err());
        assert_eq!(
            fs::read_dir(parent).unwrap().count(),
            2,
            "successful writes must consume their temporary files"
        );
        assert_eq!(
            fs::read_dir(&directory).unwrap().count(),
            1,
            "failed writes must remove their temporary file"
        );
        fs::write(&storage.path, b"truncated").unwrap();
        assert!(storage.load().unwrap_err().contains("Truncated"));
        fs::remove_dir_all(directory).unwrap();
    }
}
