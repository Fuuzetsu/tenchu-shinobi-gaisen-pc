use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Library {
    version: u8,
    pub missions: Vec<Vec<u8>>,
    pub records: Vec<Vec<(i32, String)>>,
    pub draft: Option<Vec<u8>>,
    pub record_name: String,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            missions: vec![],
            records: vec![],
            draft: None,
            record_name: "Player".into(),
        }
    }
}
impl Library {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let bytes = match storage::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(format!("Read trial library: {e}")),
        };
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Trial library is too large".into());
        }
        let library: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Read trial library: {e}"))?;
        if library.version != 1
            || library.missions.len() > 128
            || library.records.len() > 16
            || library.records.iter().any(|r| {
                r.len() > 10
                    || r.iter()
                        .any(|(t, n)| *t < 0 || *t >= 108000 || n.len() > 36)
            })
        {
            return Err("Invalid trial library".into());
        }
        for mission in library.missions.iter().chain(library.draft.iter()) {
            Mission::decode(mission)?;
        }
        Ok(library)
    }
    pub fn write(&self, path: &std::path::Path) -> Result<()> {
        storage::atomic_write(path, &serde_json::to_vec(self).map_err(|e| e.to_string())?)
    }
}
pub fn time(ticks: i32) -> String {
    format!(
        "{}:{:02}.{:02}",
        ticks / 1800,
        ticks / 30 % 60,
        ticks % 30 * 100 / 30
    )
}

pub(super) fn share(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::from("tenchu-trial-v1:");
    for b in bytes {
        let _ = write!(text, "{b:02x}");
    }
    text
}
pub(super) fn unshare(text: &str) -> Result<Vec<u8>> {
    let text = text
        .trim()
        .strip_prefix("tenchu-trial-v1:")
        .unwrap_or(text.trim());
    if ![7484 * 2, 8192 * 2].contains(&text.len()) || !text.is_ascii() {
        return Err("Paste a complete Tenchu mission share code".into());
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|p| {
            let digit = |b: u8| (b as char).to_digit(16).ok_or("Invalid mission share code");
            Ok((digit(p[0])? * 16 + digit(p[1])?) as u8)
        })
        .collect()
}
impl Editor {
    pub(super) fn save_copy(&mut self, copy: bool) -> Result<()> {
        let bytes = self.mission.as_ref().unwrap().encode()?;
        if !copy && let Origin::Saved(index) = self.origin {
            self.library.missions[index] = bytes;
        } else {
            if self.library.missions.len() >= 128 {
                return Err(
                    "Library is full (128 missions). Export or remove a mission first.".into(),
                );
            }
            self.origin = Origin::Saved(self.library.missions.len());
            self.library.missions.push(bytes);
        }
        self.persist()?;
        self.notice = "Mission saved in your local library".into();
        Ok(())
    }
    pub(super) fn import(&mut self, bytes: &[u8], assets: &GameAssets<DiscSource>) -> Result<()> {
        self.select(Mission::decode(bytes)?, Origin::Draft, assets)?;
        self.tab = Tab::Missions;
        self.notice = "Imported mission. Use Save to keep a copy in your library.".into();
        Ok(())
    }
    pub(super) fn file_import(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Tenchu mission", &["msn", "mcs"])
            .pick_file()
        {
            if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 8192 {
                return Err("Select a single .msn mission or .mcs memory-card block".into());
            }
            self.import(&std::fs::read(path).map_err(|e| e.to_string())?, assets)?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = assets;
            crate::web::import_trial();
        }
        Ok(())
    }
    pub(super) fn file_export(&self) -> Result<()> {
        let bytes = self.mission.as_ref().unwrap().encode()?;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Tenchu mission", &["msn"])
            .set_file_name("mission.msn")
            .save_file()
        {
            storage::atomic_write(&path, &bytes)?;
        }
        #[cfg(target_arch = "wasm32")]
        crate::web::export_trial(&js_sys::Uint8Array::from(bytes.as_slice()))
            .map_err(|e| format!("Download: {e:?}"))?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_missions_survive_storage_and_sharing() {
        let mut mission = Mission::new(0, Objective::ReachLord, 140).unwrap();
        mission.record(901, "Ninja".into());
        let bytes = mission.encode().unwrap();
        assert_eq!(
            Mission::decode(&unshare(&share(&bytes)).unwrap()).unwrap(),
            mission
        );
        assert!(unshare("tenchu-trial-v1:abcd").is_err());
        let library = Library {
            missions: vec![bytes.clone()],
            draft: Some(bytes),
            ..Default::default()
        };
        let path = std::env::temp_dir().join(format!("trial-library-{}.json", std::process::id()));
        library.write(&path).unwrap();
        let restored = Library::load(&path).unwrap();
        assert_eq!(Mission::decode(&restored.missions[0]).unwrap(), mission);
        assert_eq!(restored.draft, library.draft);
        std::fs::remove_file(path).unwrap();
    }
}
