use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tenchu_assets::Result;
use tenchu_core::progress::save::Save;

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let path = if let Some(path) = std::env::var_os("TENCHU_SAVE") {
            PathBuf::from(path)
        } else {
            let root = if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
                PathBuf::from(path)
            } else if cfg!(target_os = "windows") {
                PathBuf::from(std::env::var_os("APPDATA").ok_or("Set APPDATA or TENCHU_SAVE")?)
            } else {
                let root =
                    PathBuf::from(std::env::var_os("HOME").ok_or("Set HOME or TENCHU_SAVE")?);
                root.join(if cfg!(target_os = "macos") {
                    "Library/Application Support"
                } else {
                    ".local/share"
                })
            };
            root.join("tenchu-shinobi-gaisen-pc/progress.sav")
        };
        Ok(Self { path })
    }

    pub fn load(&self) -> Result<Option<Save>> {
        match fs::read(&self.path) {
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

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&temporary, path).map_err(|e| format!("Save {}: {e}", path.display()))?;
    Ok(())
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
        save.progress.stock[0][1] = 42;
        storage.write(&save).unwrap();
        assert_eq!(storage.load().unwrap().unwrap(), save);
        fs::write(&storage.path, b"truncated").unwrap();
        assert!(storage.load().unwrap_err().contains("Truncated"));
        fs::remove_dir_all(directory).unwrap();
    }
}
