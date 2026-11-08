use super::{BONES, cab_member, volume::Volume};
use crate::{
    ReadAt, Result,
    disc::Disc,
    i16_le,
    motion::{self, Key},
    slice, u32_le,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clip {
    pub id: i16,
    pub duration: i16,
    pub blend: i16,
    pub forward_speed: i16,
    pub side_speed: i16,
    pub root: Vec<Key>,
    pub bones: Vec<Vec<Key>>,
}

pub fn spline_basis(disc: &Disc<impl ReadAt>) -> Result<[[i16; 4]; 33]> {
    let (edition, data) = super::executable(disc)?;
    let executable = crate::executable::Executable::new(&data)?;
    let bytes = executable.bytes(edition.addresses().spline_basis, 33 * 8)?;
    let mut basis = [[0; 4]; 33];
    for (out, row) in basis.iter_mut().zip(bytes.chunks_exact(8)) {
        for (i, value) in out.iter_mut().enumerate() {
            *value = i16_le(row, i * 2)?;
        }
    }
    Ok(basis)
}

impl Clip {
    pub fn gaisen_pose(&self) -> motion::Clip {
        motion::Clip {
            id: self.id,
            duration: self.duration,
            sweep: 0,
            forward_speed: 0,
            side_speed: 0,
            root: self.root.clone(),
            bones: BONES
                .iter()
                .map(|&joint| {
                    self.bones.get(joint).cloned().unwrap_or_else(|| {
                        vec![
                            Key {
                                value: [0; 3],
                                time: 0,
                            },
                            Key {
                                value: [0; 3],
                                time: self.duration,
                            },
                        ]
                    })
                })
                .collect(),
        }
    }
}

pub fn parse(data: &[u8]) -> Result<BTreeMap<i16, Clip>> {
    let mut clips = BTreeMap::new();
    for at in offsets(data)? {
        let clip = clip(data, at)?;
        if clips.insert(clip.id, clip).is_some() {
            return Err("Duplicate Tenchu 2 motion ID".into());
        }
    }
    Ok(clips)
}

pub fn for_map(disc: &Disc<impl ReadAt>, hero: usize, map: usize) -> Result<BTreeMap<i16, Clip>> {
    let hero = *['r', 'a', 't'].get(hero).ok_or("Invalid Tenchu 2 hero")?;
    if map >= 24 {
        return Err("Invalid Tenchu 2 map".into());
    }
    let volume = Volume::open(disc)?;
    let cab = volume.read(r"o:\cdimage\game\cab\game", &format!("{hero}_{map:02}.cab"))?;
    parse(cab_member(
        &cab,
        r"o:\graphic\motion\bin\mot",
        &format!("{hero}_{map:02}.mot"),
    )?)
}

pub fn collect(disc: &Disc<impl ReadAt>, wanted: BTreeSet<i16>) -> Result<BTreeMap<i16, Clip>> {
    collect_from(&Volume::open(disc)?, wanted)
}

pub(super) fn collect_from(
    volume: &Volume<'_, impl ReadAt>,
    mut wanted: BTreeSet<i16>,
) -> Result<BTreeMap<i16, Clip>> {
    let mut clips = BTreeMap::new();
    for hero in ['r', 'a', 't'] {
        for map in 0..24 {
            if wanted.is_empty() {
                return Ok(clips);
            }
            let Some(cab) =
                volume.optional(r"o:\cdimage\game\cab\game", &format!("{hero}_{map:02}.cab"))?
            else {
                continue;
            };
            let data = cab_member(
                &cab,
                r"o:\graphic\motion\bin\mot",
                &format!("{hero}_{map:02}.mot"),
            )?;
            for at in offsets(data)? {
                if wanted.remove(&i16_le(data, at)?) {
                    let clip = clip(data, at)?;
                    clips.insert(clip.id, clip);
                }
            }
        }
    }
    Ok(clips)
}

fn offsets(data: &[u8]) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for index in 0..4096 {
        let at = u32_le(data, index * 4)? as usize;
        if at == 0 {
            return Ok(out);
        }
        slice(data, at, 16)?;
        out.push(at);
    }
    Err("Unterminated Tenchu 2 motion bundle".into())
}

fn clip(data: &[u8], at: usize) -> Result<Clip> {
    let duration = i16_le(data, at + 2)?;
    let count = i16_le(data, at + 4)?;
    if !(1..=64).contains(&count) || !(0..=4096).contains(&duration) {
        return Err("Invalid Tenchu 2 motion".into());
    }
    let track = |field| motion::track(data, u32_le(data, field)? as usize, duration);
    Ok(Clip {
        id: i16_le(data, at)?,
        duration,
        blend: i16_le(data, at + 6)?,
        forward_speed: i16_le(data, at + 8)?,
        side_speed: i16_le(data, at + 10)?,
        root: track(at + 12)?,
        bones: (0..count as usize)
            .map(|bone| track(at + 16 + bone * 4))
            .collect::<Result<_>>()?,
    })
}
