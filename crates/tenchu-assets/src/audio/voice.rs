use super::bank::Bank;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dialect {
    Gaisen,
    Tenchu2,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VoiceSet {
    pub bank: Arc<Bank>,
    pub dialect: Dialect,
    pub weapon_cue_program: Option<u8>,
}

#[derive(Clone)]
pub struct Sourced<T> {
    pub sound: T,
    pub bank: Option<Arc<Bank>>,
}

impl<T> From<T> for Sourced<T> {
    fn from(sound: T) -> Self {
        Self { sound, bank: None }
    }
}

impl VoiceSet {
    /// T1 hero packs retain global player vocals at their original program 4 IDs.
    pub fn gaisen_player_cue(&self, id: i16) -> bool {
        self.dialect == Dialect::Gaisen
            && matches!(id, 0x48 | 0x49 | 0x4c)
            && self.bank.programs.get(4).is_some_and(Option::is_some)
    }

    pub fn extract(source: &Bank, program: usize, dialect: Dialect) -> Result<Self> {
        let mut bank = Bank {
            volume: source.volume,
            pan: source.pan,
            programs: Vec::new(),
            samples: Vec::new(),
        };
        bank.import_program(source, program)?;
        Ok(Self {
            bank: Arc::new(bank),
            dialect,
            weapon_cue_program: None,
        })
    }

    pub fn cue(&self, host: Dialect, id: u8, dead: bool) -> i16 {
        if id <= 1
            && let Some(program) = self.weapon_cue_program
        {
            return i16::from(program) * 16 + i16::from(id);
        }
        let id = match (host, self.dialect) {
            (Dialect::Gaisen, Dialect::Tenchu2) => match id {
                6 | 7 => 13,
                8 => {
                    if dead {
                        15
                    } else {
                        14
                    }
                }
                9 => 10,
                10 => 11,
                11 => 12,
                12 => 7,
                13 => 8,
                14 => 9,
                15 => 6,
                id => id,
            },
            (Dialect::Tenchu2, Dialect::Gaisen) => match id {
                6 => 15,
                7 => 12,
                8 => 13,
                9 => 14,
                10 => 9,
                11 => 10,
                12 => 11,
                13 => 6,
                14 | 15 => 8,
                id => id,
            },
            _ => id,
        };
        i16::from(id)
    }
}
