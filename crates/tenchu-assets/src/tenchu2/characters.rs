use super::{model, motions, registry::Registry, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, model::Part, motion::Clip, texture::Vram, u16_le};
use std::collections::{BTreeMap, BTreeSet};

pub struct Character {
    pub name: String,
    pub kinds: Vec<i16>,
    pub parts: Vec<Part>,
    pub vram: Vram,
    pub idle: Option<Clip>,
}

pub struct Rig {
    pub parts: Vec<Part>,
    pub vram: Vram,
}

pub fn rig(disc: &Disc<impl ReadAt>, name: &str) -> Result<Rig> {
    if name.is_empty()
        || name.len() >= 32
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Invalid Tenchu 2 model name".into());
    }
    read_model(&Volume::open(disc)?, name, true)
}

fn read_model(volume: &Volume<'_, impl ReadAt>, name: &str, native: bool) -> Result<Rig> {
    let directory = r"o:\cdimage\game\actor";
    let data = volume.read(directory, &format!("{name}.mdl"))?;
    let mut parts = if native {
        model::rig(&data)?
    } else {
        model::character(&data)?
    };
    let textures = volume.read(directory, &format!("{name}.arc"))?;
    let mut vram = Vram::default();
    let count = usize::from(u16_le(&textures, 0)?);
    if count > 256 {
        return Err("Excessive Tenchu 2 character textures".into());
    }
    for i in 0..count {
        vram.upload_tim(crate::model::arc_entry(&textures, i)?)?;
    }
    model::resolve_transparency(parts.iter_mut(), &vram);
    Ok(Rig { parts, vram })
}

struct Definition {
    name: String,
    kinds: Vec<i16>,
    idle: Option<i16>,
}

fn definitions(volume: &Volume<'_, impl ReadAt>) -> Result<Vec<Definition>> {
    let registry = Registry::read(volume)?;
    let mut out = BTreeMap::<String, Definition>::new();
    for actor in &registry.actors {
        if let Some(existing) = out.get_mut(&actor.model) {
            existing.kinds.push(actor.kind);
            continue;
        }
        out.insert(
            actor.model.clone(),
            Definition {
                name: actor.model.clone(),
                kinds: vec![actor.kind],
                idle: registry.clip(actor.kind, 0),
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
    let wanted = definitions.iter().filter_map(|d| d.idle).collect();
    let idle_clips = motions::collect_from(&volume, wanted)?;
    definitions
        .into_iter()
        .map(|definition| {
            let Rig { parts, vram } = read_model(&volume, &definition.name, false)?;
            let idle = definition
                .idle
                .and_then(|id| idle_clips.get(&id))
                .map(motions::Clip::gaisen_pose);
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
