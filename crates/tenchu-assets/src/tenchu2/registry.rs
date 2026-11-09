use super::{Edition, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, i16_le, slice, u32_le};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod stealth;
pub use stealth::{StealthEffect, StealthEvent};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registration {
    pub action: i16,
    pub clip: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub kind: i16,
    pub model: String,
    pub health: i16,
    pub weapon: i16,
    pub turn: i16,
    pub sight_penalty: i16,
    pub skill: i16,
    pub motions: Vec<Registration>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registry {
    pub edition: Edition,
    pub actors: Vec<Actor>,
    pub common: Vec<Registration>,
    cast: Vec<[Vec<i16>; 3]>,
    weapon_frames: Vec<(i16, i16)>,
    stealth: stealth::Events,
}

impl Registry {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        Self::read(&Volume::open(disc)?)
    }

    pub(super) fn read(volume: &Volume<'_, impl ReadAt>) -> Result<Self> {
        let data = volume.game_module()?;
        Self::parse(&data, volume.edition)
    }

    fn parse(data: &[u8], edition: Edition) -> Result<Self> {
        let addresses = edition.addresses();
        let pointer = |at| -> Result<usize> {
            u32_le(data, at)?
                .checked_sub(addresses.module_base)
                .map(|v| v as usize)
                .ok_or_else(|| "Invalid Tenchu 2 character pointer".into())
        };
        let common = motions(data, addresses.common_motions)?;
        let mut weapon_frames = Vec::new();
        for index in 0..64 {
            let at = addresses.characters - 0x60 + index * 4;
            let clip = i16_le(data, at)?;
            if clip == -1 {
                break;
            }
            let frame = i16_le(data, at + 2)?;
            if clip < 0 || frame < 0 || weapon_frames.iter().any(|&(id, _)| id == clip) {
                return Err("Invalid Tenchu 2 weapon animation timing".into());
            }
            weapon_frames.push((clip, frame));
        }
        if weapon_frames.len() == 64 {
            return Err("Unterminated Tenchu 2 weapon animation timing".into());
        }
        let mut cast = Vec::new();
        for map in 0..24 {
            let mut heroes = [Vec::new(), Vec::new(), Vec::new()];
            for (hero, actors) in heroes.iter_mut().enumerate() {
                let at = addresses.cast + map * 96 + hero * 32;
                for kind in slice(data, at, 32)?.chunks_exact(2) {
                    let kind = i16_le(kind, 0)?;
                    if kind == -1 {
                        break;
                    }
                    actors.push(kind);
                }
                if actors.len() == 16 {
                    return Err("Unterminated Tenchu 2 map cast".into());
                }
            }
            cast.push(heroes);
        }
        let mut actors = Vec::new();
        let mut kinds = BTreeSet::new();
        for index in 0..512 {
            let at = addresses.characters + index * 24;
            let kind = i16_le(data, at)?;
            if kind == -1 {
                if cast.iter().flatten().flatten().any(|k| !kinds.contains(k)) {
                    return Err("Unknown actor in Tenchu 2 map cast".into());
                }
                return Ok(Self {
                    edition,
                    actors,
                    common,
                    cast,
                    weapon_frames,
                    stealth: stealth::read(data, edition)?,
                });
            }
            if !(0..=255).contains(&kind) || !kinds.insert(kind) {
                return Err("Invalid or duplicate Tenchu 2 actor kind".into());
            }
            let bytes = slice(data, pointer(at + 4)?, 32)?;
            let end = bytes
                .iter()
                .position(|&v| v == 0)
                .ok_or("Unterminated Tenchu 2 model name")?;
            let name = &bytes[..end];
            if name.is_empty() || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') {
                return Err("Invalid Tenchu 2 model name".into());
            }
            actors.push(Actor {
                kind,
                model: String::from_utf8(name.to_vec()).unwrap(),
                health: i16_le(data, at + 2)?,
                weapon: i16_le(data, at + 12)?,
                turn: i16_le(data, at + 14)?,
                sight_penalty: i16_le(data, at + 16)?,
                skill: i16_le(data, at + 18)?,
                motions: motions(data, pointer(at + 8)?)?,
            });
        }
        Err("Unterminated Tenchu 2 actor registry".into())
    }

    pub fn actor(&self, kind: i16) -> Option<&Actor> {
        self.actors.iter().find(|actor| actor.kind == kind)
    }

    pub fn cast(&self, hero: usize, map: usize) -> Option<&[i16]> {
        Some(self.cast.get(map)?.get(hero)?.as_slice())
    }

    pub fn clip(&self, kind: i16, action: i16) -> Option<i16> {
        self.actor(kind)?
            .motions
            .iter()
            .chain(&self.common)
            .find(|entry| entry.action == action)
            .map(|entry| entry.clip)
    }

    pub fn stealth_events(&self, hero: usize, style: usize) -> Option<&[StealthEvent]> {
        Some(self.stealth.get(hero)?.get(style)?.as_slice())
    }

    pub fn weapon_frame(&self, clip: i16, duration: i16) -> i16 {
        self.weapon_frames
            .iter()
            .find(|&&(id, _)| id == clip)
            .map_or(duration / 3, |&(_, frame)| match self.edition {
                Edition::Europe => (i32::from(frame) * 5 / 6) as i16,
                Edition::Japan => frame,
            })
    }
}

fn motions(data: &[u8], at: usize) -> Result<Vec<Registration>> {
    let mut entries = Vec::new();
    for index in 0..2048 {
        let row = slice(data, at + index * 8, 8)?;
        let action = i16_le(row, 0)?;
        if action == -1 {
            return Ok(entries);
        }
        let clip = i16_le(row, 2)?;
        if action < 0 || clip < 0 {
            return Err("Invalid Tenchu 2 motion registration".into());
        }
        entries.push(Registration { action, clip });
    }
    Err("Unterminated Tenchu 2 motion registry".into())
}
