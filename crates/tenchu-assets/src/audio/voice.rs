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
        })
    }

    pub fn cue(&self, host: Dialect, id: u8, dead: bool) -> i16 {
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
