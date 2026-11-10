use super::{Edition, executable};
use crate::{ReadAt, Result, disc::Disc, executable::Executable, i16_le, short3, u16_le};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub center: [i16; 3],
    pub radius: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Objective {
    Reach { region: Region, unseen: bool },
    Weaken { targets: [i16; 3] },
    Defeat { targets: [i16; 3] },
    EliminateUnseen { count: i16 },
    EliminateAll,
    Scripted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Introduction,
    Event,
    Movie,
    Gameplay(Objective),
    Results,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sequence {
    pub kind: Kind,
    pub resource: u16,
    pub next: Option<usize>,
    pub failure: Option<usize>,
}

#[derive(Clone)]
pub struct Campaign {
    pub sequences: Vec<Sequence>,
    pub starts: [[Option<usize>; 11]; 3],
    pub rewards: [Reward; 7],
}

#[derive(Clone, Copy)]
pub struct Reward {
    pub grand_masters: u8,
    pub item: usize,
}

impl Campaign {
    pub fn mission(&self, hero: usize, sequence: usize) -> Option<usize> {
        for (mission, &start) in self.starts.get(hero)?.iter().enumerate() {
            let Some(start) = start else { continue };
            let mut pending = vec![start];
            let mut visited = vec![false; self.sequences.len()];
            while let Some(index) = pending.pop() {
                if index == sequence {
                    return Some(mission);
                }
                let row = self.sequences.get(index)?;
                if visited[index] || row.kind == Kind::Results {
                    continue;
                }
                visited[index] = true;
                pending.extend(row.next);
                pending.extend(row.failure);
            }
        }
        None
    }

    pub fn first_gameplay(&self, hero: usize, mission: usize) -> Option<usize> {
        let start = (*self.starts.get(hero)?.get(mission)?)?;
        (start..self.sequences.len()).find(|&i| matches!(self.sequences[i].kind, Kind::Gameplay(_)))
    }

    pub fn result_mission(&self, hero: usize, result: usize) -> Option<usize> {
        for (mission, &start) in self.starts.get(hero)?.iter().enumerate() {
            let Some(mut index) = start else { continue };
            for _ in 0..self.sequences.len() {
                let sequence = self.sequences.get(index)?;
                if sequence.kind == Kind::Results {
                    if index == result {
                        return Some(mission);
                    }
                    break;
                }
                let Some(next) = sequence.next else { break };
                index = next;
            }
        }
        None
    }

    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, bytes) = executable(disc)?;
        if edition != Edition::Japan {
            return Err("Native Tenchu 2 campaign requires the Japanese disc".into());
        }
        Self::parse(&Executable::new(&bytes)?)
    }

    fn parse(exe: &Executable<'_>) -> Result<Self> {
        let records = exe.bytes(0x80084b2c, 0x1030)?;
        let mut sequences = Vec::with_capacity(records.len() / 16);
        let link = |row: &[u8], at| -> Result<Option<usize>> {
            match i16_le(row, at)? {
                0 => Ok(None),
                id if (1..records.len() as i16 / 16).contains(&id) => Ok(Some(id as usize)),
                _ => Err("Invalid Tenchu 2 sequence link".into()),
            }
        };
        for row in records.chunks_exact(16) {
            let flags = u16_le(row, 0)?;
            let targets = short3(row, 8)?;
            let kind = match flags {
                0 => Kind::Introduction,
                0x10 => Kind::Event,
                0x20 => Kind::Movie,
                0x31 | 0x32 => Kind::Gameplay(Objective::Reach {
                    region: Region {
                        center: targets,
                        radius: i16_le(row, 14)?,
                    },
                    unseen: flags == 0x32,
                }),
                0x33 => Kind::Gameplay(Objective::Weaken { targets }),
                0x34 => Kind::Gameplay(Objective::Defeat { targets }),
                0x35 => Kind::Gameplay(Objective::EliminateUnseen { count: targets[0] }),
                0x36 => Kind::Gameplay(Objective::EliminateAll),
                0x40 => Kind::Results,
                _ => return Err(format!("Unknown Tenchu 2 sequence kind {flags:#x}")),
            };
            sequences.push(Sequence {
                kind,
                resource: u16_le(row, 2)?,
                next: link(row, 4)?,
                failure: link(row, 6)?,
            });
        }
        let mut starts = [[None; 11]; 3];
        for (i, start) in starts.iter_mut().flatten().enumerate() {
            let id = i16_le(exe.bytes(0x80085b5c, 66)?, i * 2)?;
            if id != -1 {
                *start = Some(
                    usize::try_from(id)
                        .ok()
                        .filter(|&id| id < sequences.len())
                        .ok_or("Invalid Tenchu 2 mission start")?,
                );
            }
        }
        let bytes = exe.bytes(0x80085bd4, 14)?;
        let mut rewards = [Reward {
            grand_masters: 0,
            item: 0,
        }; 7];
        for (reward, row) in rewards.iter_mut().zip(bytes.chunks_exact(2)) {
            if !(1..=20).contains(&row[1]) {
                return Err("Invalid Tenchu 2 reward item".into());
            }
            *reward = Reward {
                grand_masters: row[0],
                item: usize::from(row[1]),
            };
        }
        Ok(Self {
            sequences,
            starts,
            rewards,
        })
    }
}
