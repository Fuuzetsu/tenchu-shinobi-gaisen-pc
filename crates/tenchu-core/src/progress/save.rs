use super::{LAYOUTS, Progress, STAGES, TrainingRecord};
use crate::{Result, ai::Difficulty, game::score::Record};
use sha2::{Digest, Sha256};
use tenchu_assets::script::Language;

const MAGIC: &[u8; 8] = b"TENCHUPC";
const VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub character: u8,
    pub stage: u8,
    pub layout: u8,
    pub difficulty: Difficulty,
    pub language: Language,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Save {
    pub progress: Progress,
    pub selection: Selection,
    pub seed: u32,
}

impl Default for Save {
    fn default() -> Self {
        Self {
            progress: Progress::default(),
            selection: Selection::default(),
            seed: 1,
        }
    }
}

impl Save {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(VERSION.to_le_bytes());
        let selection = self.selection;
        bytes.extend([
            selection.character,
            selection.stage,
            selection.layout,
            selection.difficulty as u8,
            match selection.language {
                Language::English => 0,
                Language::French => 1,
                Language::Italian => 2,
                Language::Japanese => 3,
            },
        ]);
        bytes.extend(self.seed.to_le_bytes());
        bytes.extend(self.progress.flags.to_le_bytes());
        for highest in self.progress.highest {
            bytes.extend(highest.to_le_bytes());
        }
        for stock in self.progress.stock {
            bytes.extend(stock);
        }
        for record in self.progress.records.iter().flatten().flatten() {
            bytes.extend([
                record.bosses,
                record.enemies,
                record.found,
                record.murders,
                record.criticals,
                record.friends,
            ]);
            bytes.extend(record.clock.to_le_bytes());
        }
        for record in self.progress.training {
            bytes.extend([record.character, record.grade]);
            bytes.extend(record.clock.to_le_bytes());
        }
        bytes.extend(Sha256::digest(&bytes));
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let end = bytes.len().checked_sub(32).ok_or("Truncated save")?;
        if Sha256::digest(&bytes[..end])[..] != bytes[end..] {
            return Err("Save checksum mismatch".into());
        }
        let mut reader = Reader(&bytes[..end]);
        if &reader.array::<8>()? != MAGIC || u32::from_le_bytes(reader.array()?) != VERSION {
            return Err("Unsupported save format".into());
        }
        let [character, stage, layout, difficulty, language] = reader.array()?;
        if character >= 2
            || usize::from(stage) >= STAGES
            || (usize::from(layout) >= LAYOUTS && layout != 255)
        {
            return Err("Invalid saved mission selection".into());
        }
        let difficulty = match difficulty {
            0 => Difficulty::Easy,
            1 => Difficulty::Normal,
            2 => Difficulty::Hard,
            _ => return Err("Invalid saved difficulty".into()),
        };
        let language = match language {
            0 => Language::English,
            1 => Language::French,
            2 => Language::Italian,
            3 => Language::Japanese,
            _ => return Err("Invalid saved language".into()),
        };
        let mut save = Self {
            selection: Selection {
                character,
                stage,
                layout,
                difficulty,
                language,
            },
            seed: u32::from_le_bytes(reader.array()?),
            ..Self::default()
        };
        save.progress.flags = u32::from_le_bytes(reader.array()?);
        for highest in &mut save.progress.highest {
            *highest = i16::from_le_bytes(reader.array()?);
            if !(0..STAGES as i16).contains(highest) {
                return Err("Invalid saved mission progress".into());
            }
        }
        for stock in &mut save.progress.stock {
            *stock = reader.array()?;
        }
        for record in save.progress.records.iter_mut().flatten().flatten() {
            let [bosses, enemies, found, murders, criticals, friends] = reader.array()?;
            *record = Record {
                bosses,
                enemies,
                found,
                murders,
                criticals,
                friends,
                clock: i32::from_le_bytes(reader.array()?),
            };
        }
        for record in &mut save.progress.training {
            let [character, grade] = reader.array()?;
            if character >= 2 || grade >= 5 {
                return Err("Invalid training record".into());
            }
            *record = TrainingRecord {
                character,
                grade,
                clock: i32::from_le_bytes(reader.array()?),
            };
        }
        if !reader.0.is_empty() {
            return Err("Unexpected data at end of save".into());
        }
        Ok(save)
    }
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.0.get(..N).ok_or("Truncated save")?;
        self.0 = &self.0[N..];
        Ok(bytes.try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_progress_rejects_corruption_and_retains_signed_records() {
        let mut save = Save::default();
        save.progress.stock[1][13] = 255;
        save.progress.flags = 0x800007ff;
        save.progress.records[1][10][2] = Record {
            bosses: 2,
            found: 255,
            enemies: 19,
            clock: -300,
            criticals: 244,
            murders: 13,
            friends: 12,
        };
        save.progress.training[0] = TrainingRecord {
            character: 1,
            grade: 4,
            clock: 3000,
        };
        save.selection.layout = 255;
        let bytes = save.encode();
        assert_eq!(Save::decode(&bytes).unwrap(), save);
        for length in 0..bytes.len() {
            assert!(Save::decode(&bytes[..length]).is_err());
        }
        for at in 0..bytes.len() {
            let mut corrupt = bytes.clone();
            corrupt[at] ^= 1;
            assert!(Save::decode(&corrupt).is_err());
        }
        for (at, value) in [(8, 2), (12, 2), (13, 11), (14, 3), (15, 3), (16, 4)] {
            let mut corrupt = bytes[..bytes.len() - 32].to_vec();
            corrupt[at] = value;
            corrupt.extend(Sha256::digest(&corrupt));
            assert!(Save::decode(&corrupt).is_err());
        }
    }
}
