mod menu;
pub use menu::Manager;

use crate::storage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tenchu_assets::Result;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Game {
    #[default]
    Tenchu1,
    #[cfg(not(target_arch = "wasm32"))]
    Tenchu2,
}

impl Game {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tenchu1 => "Tenchu · Shinobi Gaisen",
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tenchu2 => "Tenchu 2 · Japan",
        }
    }

    pub fn path(self) -> Result<PathBuf> {
        match self {
            Self::Tenchu1 => storage::UserFile::Progress.path(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tenchu2 => storage::UserFile::Tenchu2Progress.path(),
        }
    }

    fn summary(self, bytes: &[u8]) -> Result<Vec<String>> {
        match self {
            Self::Tenchu1 => {
                let save = tenchu_core::progress::save::Save::decode(bytes)?;
                let names = ["Rikimaru", "Ayame"];
                let mut lines = vec![format!(
                    "Continue as {}",
                    names[usize::from(save.selection.character)]
                )];
                lines.extend(names.into_iter().zip(save.progress.highest).map(
                    |(name, highest)| format!("{name}: furthest mission cleared {highest} / 10"),
                ));
                Ok(lines)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Self::Tenchu2 => {
                let progress = crate::tenchu2::native::save::SaveFile::decode(bytes)?;
                let mut lines: Vec<_> = ["Rikimaru", "Ayame", "Tatsumaru"]
                    .into_iter()
                    .zip(&progress.records)
                    .map(|(name, records)| {
                        let completed = records
                            .iter()
                            .filter(|r| {
                                **r != Default::default()
                                    && **r
                                        != tenchu_core::tenchu2::progress::Record {
                                            time_tenths: 59999,
                                            ..Default::default()
                                        }
                            })
                            .count();
                        let masters = records.iter().filter(|r| r.rank == 6).count();
                        format!("{name}: {completed} recorded results · {masters} Grand Master")
                    })
                    .collect();
                if progress.tatsumaru_unlocked {
                    lines.push("Tatsumaru unlocked in this campaign".into());
                }
                Ok(lines)
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Slot {
    pub id: u64,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Index {
    version: u32,
    active: u64,
    next: u64,
    slots: Vec<Slot>,
}

impl Default for Index {
    fn default() -> Self {
        Self {
            version: 1,
            active: 0,
            next: 1,
            slots: vec![Slot {
                id: 0,
                name: "Campaign 1".into(),
            }],
        }
    }
}

pub struct Catalog {
    base: PathBuf,
    directory: PathBuf,
    index: Index,
}

impl Catalog {
    pub fn load(base: PathBuf) -> Result<Self> {
        let mut directory = base.as_os_str().to_owned();
        directory.push(".campaigns");
        let directory = PathBuf::from(directory);
        let path = directory.join("index.json");
        let index = match storage::read(&path) {
            Ok(bytes) => {
                Self::decode_index(&bytes).map_err(|e| format!("{}: {e}", path.display()))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Index::default(),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        Ok(Self {
            base,
            directory,
            index,
        })
    }

    fn decode_index(bytes: &[u8]) -> Result<Index> {
        let index: Index = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if index.version != 1
            || index.slots.is_empty()
            || !index.slots.iter().any(|s| s.id == index.active)
            || !index.slots.iter().any(|s| s.id == 0)
            || index.slots.iter().enumerate().any(|(i, s)| {
                s.id >= index.next
                    || Self::name(&index, &s.name, Some(s.id)).is_err()
                    || index.slots[..i].iter().any(|other| other.id == s.id)
            })
        {
            return Err("Invalid campaign catalog".into());
        }
        Ok(index)
    }

    pub fn active(base: PathBuf) -> Result<u64> {
        Ok(Self::load(base)?.index.active)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn backup_files(bytes: &[u8]) -> Result<Vec<String>> {
        Ok(Self::decode_index(bytes)?
            .slots
            .iter()
            .filter(|s| s.id != 0)
            .map(|s| format!("{}.sav", s.id))
            .collect())
    }

    pub fn active_path(base: PathBuf) -> Result<PathBuf> {
        let catalog = Self::load(base)?;
        catalog.path(catalog.index.active)
    }

    pub fn path(&self, id: u64) -> Result<PathBuf> {
        if !self.index.slots.iter().any(|s| s.id == id) {
            return Err("Campaign no longer exists".into());
        }
        Ok(self.slot_path(id))
    }

    fn slot_path(&self, id: u64) -> PathBuf {
        if id == 0 {
            self.base.clone()
        } else {
            self.directory.join(format!("{id}.sav"))
        }
    }

    fn name(index: &Index, name: &str, replacing: Option<u64>) -> Result<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err("Use a name of 1–64 visible characters".into());
        }
        if index
            .slots
            .iter()
            .any(|s| Some(s.id) != replacing && s.name.eq_ignore_ascii_case(name))
        {
            return Err("A campaign already has that name".into());
        }
        Ok(name.into())
    }

    fn store(&mut self, index: Index) -> Result<()> {
        storage::atomic_write(
            &self.directory.join("index.json"),
            &serde_json::to_vec_pretty(&index).map_err(|e| e.to_string())?,
        )?;
        self.index = index;
        Ok(())
    }

    pub fn select(&mut self, id: u64) -> Result<()> {
        self.path(id)?;
        let mut index = self.index.clone();
        index.active = id;
        self.store(index)
    }

    pub(crate) fn create(&mut self, name: &str, copy: Option<u64>) -> Result<u64> {
        let name = Self::name(&self.index, name, None)?;
        let mut index = self.index.clone();
        // A failed catalog write can leave an unlisted copy. Never reuse its file.
        loop {
            match storage::read(&self.slot_path(index.next)) {
                Ok(_) => {
                    index.next = index
                        .next
                        .checked_add(1)
                        .ok_or("Campaign ID limit reached")?
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                Err(e) => return Err(e.to_string()),
            }
        }
        let id = index.next;
        index.next = id.checked_add(1).ok_or("Campaign ID limit reached")?;
        if let Some(source) = copy {
            match storage::read(&self.path(source)?) {
                Ok(bytes) => storage::atomic_write(&self.slot_path(id), &bytes)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        index.slots.push(Slot { id, name });
        self.store(index)?;
        Ok(id)
    }

    fn rename(&mut self, id: u64, name: &str) -> Result<()> {
        let name = Self::name(&self.index, name, Some(id))?;
        let mut index = self.index.clone();
        index
            .slots
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Campaign no longer exists")?
            .name = name;
        self.store(index)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn state_directory(game: Game) -> Result<PathBuf> {
    let directory = storage::UserFile::States.path()?;
    let active = Catalog::active(game.path()?)?;
    Ok(if active == 0 {
        directory
    } else {
        let game = match game {
            Game::Tenchu1 => "t1",
            Game::Tenchu2 => "t2",
        };
        directory.join(format!("{game}-campaign-{active}"))
    })
}

#[cfg(test)]
mod tests;
