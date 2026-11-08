use crate::storage::{self, UserFile};
use std::path::PathBuf;
use tenchu_assets::Result;
use tenchu_core::tenchu2::progress::Progress;

pub struct SaveFile {
    path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Save {
    version: u32,
    progress: Progress,
}

impl SaveFile {
    pub fn new() -> Result<Self> {
        Ok(Self {
            path: UserFile::Tenchu2Progress.path()?,
        })
    }

    #[cfg(test)]
    pub(super) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Progress> {
        let bytes = match storage::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Progress::default());
            }
            Err(error) => return Err(format!("{}: {error}", self.path.display())),
        };
        Self::decode(&bytes).map_err(|error| format!("{}: {error}", self.path.display()))
    }

    fn decode(bytes: &[u8]) -> Result<Progress> {
        let save: Save = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if save.version != 1 {
            return Err(format!(
                "Unsupported Japanese Tenchu 2 save version {}",
                save.version
            ));
        }
        if save.progress.records.iter().flatten().any(|r| r.rank > 6)
            || save.progress.stock.iter().any(|stock| {
                stock.items.iter().flatten().any(|&amount| amount > 99)
                    || stock.items[1..].iter().all(Option::is_none)
            })
        {
            return Err("Invalid Japanese Tenchu 2 records or supplies".into());
        }
        Ok(save.progress)
    }

    pub fn write(&self, progress: &Progress) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&Save {
            version: 1,
            progress: progress.clone(),
        })
        .map_err(|e| e.to_string())?;
        storage::atomic_write(&self.path, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_progress_round_trip_and_invalid_saves() {
        let root = std::env::temp_dir().join(format!(
            "tenchu-jp-progress-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = SaveFile::at(root.join("tenchu2-jp-progress.json"));
        assert_eq!(file.load().unwrap().stock[0].items[4], Some(10));
        let mut progress = Progress::default();
        progress.stock[1].items[7] = Some(13);
        progress.records[2][4].rank = 6;
        progress.records[2][4].time_tenths = 1234;
        progress.tatsumaru_unlocked = true;
        file.write(&progress).unwrap();
        let bytes = std::fs::read(&file.path).unwrap();
        assert_eq!(
            serde_json::to_value(file.load().unwrap()).unwrap(),
            serde_json::to_value(&progress).unwrap()
        );
        for value in [2, 0] {
            let mut data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            data["version"] = value.into();
            assert!(SaveFile::decode(&serde_json::to_vec(&data).unwrap()).is_err());
        }
        let mut data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        data["progress"]["records"][0][0]["rank"] = 7.into();
        assert!(SaveFile::decode(&serde_json::to_vec(&data).unwrap()).is_err());
        std::fs::write(&file.path, b"truncated").unwrap();
        assert!(
            file.load()
                .unwrap_err()
                .contains("tenchu2-jp-progress.json")
        );
        assert_eq!(std::fs::read(&file.path).unwrap(), b"truncated");
        std::fs::remove_dir_all(root).unwrap();
    }
}
