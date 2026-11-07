use super::{BONES, cab_member, model, volume::Volume};
use crate::{
    ReadAt, Result,
    disc::Disc,
    i16_le,
    model::Part,
    motion::{Clip, Key},
    slice,
    texture::Vram,
    u16_le, u32_le,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub struct Character {
    pub name: String,
    pub kinds: Vec<i16>,
    pub parts: Vec<Part>,
    pub vram: Vram,
    pub idle: Option<Clip>,
}

struct Definition {
    name: String,
    kinds: Vec<i16>,
    idle: Option<i16>,
}

fn definitions(volume: &Volume<'_, impl ReadAt>) -> Result<Vec<Definition>> {
    let data = volume.read(r"o:\cdimage\game\module", "datamng.bin")?;
    let addresses = volume.edition.addresses();
    if format!("{:x}", Sha256::digest(&data)) != addresses.module_hash {
        return Err(format!(
            "Unsupported {} character module revision",
            volume.edition.name()
        ));
    }
    let pointer = |at| -> Result<usize> {
        u32_le(&data, at)?
            .checked_sub(addresses.module_base)
            .map(|v| v as usize)
            .ok_or_else(|| "Invalid Tenchu 2 character pointer".into())
    };
    let mut out = BTreeMap::<String, Definition>::new();
    for at in (addresses.characters..data.len()).step_by(24) {
        let kind = i16_le(&data, at)?;
        if kind < 0 {
            break;
        }
        let name_at = pointer(at + 4)?;
        let name = slice(&data, name_at, 32)?
            .split(|&b| b == 0)
            .next()
            .unwrap();
        if name.is_empty() || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') {
            return Err("Invalid Tenchu 2 model name".into());
        }
        let name = String::from_utf8(name.to_vec()).unwrap();
        if let Some(existing) = out.get_mut(&name) {
            existing.kinds.push(kind);
            continue;
        }
        let mut idle = None;
        for row in data
            .get(pointer(at + 8)?..)
            .ok_or("Missing Tenchu 2 motions")?
            .chunks_exact(8)
        {
            let action = u16_le(row, 0)?;
            if action == u16::MAX {
                break;
            }
            if action == 0 || (action == 0x100 && idle.is_none()) {
                idle = Some(i16_le(row, 2)?);
            }
        }
        out.insert(
            name.clone(),
            Definition {
                name,
                kinds: vec![kind],
                idle,
            },
        );
    }
    for (name, base) in [
        ("rikia", "rikimaru"),
        ("ayamea", "ayame"),
        ("tatua", "tatumaru"),
        ("tatua_m", "tatumaru"),
        ("tatu2_m", "tatu2"),
    ] {
        if volume
            .optional(r"o:\cdimage\game\actor", &format!("{name}.mdl"))?
            .is_some()
        {
            let idle = out[base].idle;
            out.insert(
                name.into(),
                Definition {
                    name: name.into(),
                    kinds: Vec::new(),
                    idle,
                },
            );
        }
    }
    Ok(out.into_values().collect())
}

/// Read only the selected source actor kinds, or the whole catalogue for the model picker.
pub fn load(disc: &Disc<impl ReadAt>, kinds: Option<&BTreeSet<i16>>) -> Result<Vec<Character>> {
    let volume = Volume::open(disc)?;
    let definitions: Vec<_> = definitions(&volume)?
        .into_iter()
        .filter(|d| kinds.is_none_or(|k| d.kinds.iter().any(|i| k.contains(i))))
        .collect();
    let mut wanted: BTreeSet<_> = definitions.iter().filter_map(|d| d.idle).collect();
    let mut idle_clips = BTreeMap::new();
    for hero in ['r', 'a', 't'] {
        for map in 0..24 {
            if wanted.is_empty() {
                break;
            }
            let Some(cab) =
                volume.optional(r"o:\cdimage\game\cab\game", &format!("{hero}_{map:02}.cab"))?
            else {
                continue;
            };
            let motion = cab_member(
                &cab,
                r"o:\graphic\motion\bin\mot",
                &format!("{hero}_{map:02}.mot"),
            )?;
            for at in offsets(motion)? {
                let id = i16_le(motion, at)?;
                if wanted.remove(&id) {
                    idle_clips.insert(id, clip(motion, at)?);
                }
            }
        }
    }
    definitions
        .into_iter()
        .map(|definition| {
            let directory = r"o:\cdimage\game\actor";
            let mut parts =
                model::character(&volume.read(directory, &format!("{}.mdl", definition.name))?)?;
            let textures = volume.read(directory, &format!("{}.arc", definition.name))?;
            let mut vram = Vram::default();
            let count = usize::from(u16_le(&textures, 0)?);
            if count > 256 {
                return Err("Excessive Tenchu 2 character textures".into());
            }
            for i in 0..count {
                vram.upload_tim(crate::model::arc_entry(&textures, i)?)?;
            }
            model::resolve_transparency(parts.iter_mut(), &vram);
            let idle = definition.idle.and_then(|id| idle_clips.get(&id)).cloned();
            Ok(Character {
                name: definition.name,
                kinds: definition.kinds,
                parts,
                vram,
                idle,
            })
        })
        .collect()
}

fn offsets(data: &[u8]) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for i in 0..4096 {
        let at = u32_le(data, i * 4)? as usize;
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
        return Err("Invalid Tenchu 2 idle motion".into());
    }
    let track = |field| crate::motion::track(data, u32_le(data, field)? as usize, duration);
    Ok(Clip {
        id: i16_le(data, at)?,
        duration,
        sweep: 0,
        forward_speed: 0,
        side_speed: 0,
        root: track(at + 12)?,
        bones: BONES
            .iter()
            .map(|&joint| {
                if joint < count as usize {
                    track(at + 16 + joint * 4)
                } else {
                    Ok(vec![
                        Key {
                            value: [0; 3],
                            time: 0,
                        },
                        Key {
                            value: [0; 3],
                            time: duration,
                        },
                    ])
                }
            })
            .collect::<Result<_>>()?,
    })
}
