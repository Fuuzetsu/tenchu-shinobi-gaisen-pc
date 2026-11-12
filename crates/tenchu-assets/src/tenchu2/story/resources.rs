use super::{Edition, Patch, Script, Volume, cab_member};
use crate::{
    ReadAt, Result,
    disc::Disc,
    i16_le,
    model::{Part, arc_entry},
    texture::Vram,
    u16_le, u32_le,
};
use std::collections::BTreeMap;

pub struct Model {
    pub name: String,
    pub parts: Vec<Part>,
    pub vram: Vram,
}

pub struct Resources {
    pub area: usize,
    pub results_fade: i16,
    pub script: Script,
    pub models: BTreeMap<i16, Model>,
    pub effects: BTreeMap<i16, Option<Sprite>>,
    pub weapons: BTreeMap<String, Model>,
    pub attachments: Vec<Attachment>,
    pub faces: BTreeMap<i16, Vec<Patch>>,
    pub audio: Option<crate::audio::stream::Track>,
    pub portraits: Vec<Picture>,
    pub pictures: Vec<Picture>,
}

pub struct Sprite {
    pub name: String,
    pub size: i16,
    pub texture: Option<(Vram, crate::texture::Image)>,
}

pub struct Picture {
    pub size: [u16; 2],
    pub rgba: Vec<u8>,
}

impl Picture {
    fn parse(bytes: &[u8]) -> Result<Self> {
        let mut vram = Vram::default();
        let image = super::super::texture::upload(&mut vram, bytes)?;
        let mut rgba =
            Vec::with_capacity(usize::from(image.size[0]) * usize::from(image.size[1]) * 4);
        for y in 0..image.size[1] {
            for x in 0..image.size[0] {
                rgba.extend(crate::texture::rgba8(vram.texel(
                    image.page,
                    image.palette,
                    [u16::from(image.uv[0]) + x, u16::from(image.uv[1]) + y],
                )));
            }
        }
        Ok(Self {
            size: image.size,
            rgba,
        })
    }
}

pub struct Attachment {
    pub actor: usize,
    pub slot: usize,
    pub bone: usize,
    pub model: String,
}

struct Files<'a, S> {
    volume: Volume<'a, S>,
    cab: Vec<u8>,
}

impl<S: ReadAt> Files<'_, S> {
    fn read(&self, directory: &str, name: &str) -> Result<Vec<u8>> {
        self.optional(directory, name)?
            .ok_or_else(|| format!("Missing Tenchu 2 story resource {directory}/{name}"))
    }

    fn optional(&self, directory: &str, name: &str) -> Result<Option<Vec<u8>>> {
        // The active CAB overrides the same resource in VOLUME.DAT.
        let wanted = (super::super::hash(directory), super::super::hash(name));
        let count = u32_le(&self.cab, 0)? as usize;
        if count > 4096 {
            return Err("Excessive Tenchu 2 event bundle entries".into());
        }
        for row in crate::slice(&self.cab, 4, count * 12)?.chunks_exact(12) {
            if (u32_le(row, 4)?, u32_le(row, 8)?) == wanted {
                return Ok(Some(cab_member(&self.cab, directory, name)?.to_vec()));
            }
        }
        self.volume.optional(directory, name)
    }
}

impl Resources {
    pub fn load(disc: &Disc<impl ReadAt>, event: u16) -> Result<Self> {
        let volume = Volume::open(disc)?;
        if volume.edition != Edition::Japan {
            return Err("Native Tenchu 2 story requires the Japanese disc".into());
        }
        let cab = volume.read(r"o:\cdimage\game\cab\event", &format!("E{event}.CAB"))?;
        let script = Script::parse(cab_member(
            &cab,
            r"o:\cdimage\game\event",
            &format!("E{event}.EVT"),
        )?)?;
        let module = volume.read(r"o:\cdimage\game\module", "event.bin")?;
        let audio = if u16_le(&script.audio, 6)? == 0 {
            None
        } else {
            let path = module_string(&module, 0x94c)?
                .trim_start_matches('\\')
                .split(';')
                .next()
                .unwrap()
                .replace('\\', "/");
            let file = disc.find(&path)?;
            let start = super::super::music::time(&script.audio[4..6])?;
            let end = super::super::music::time(&script.audio[6..8])?;
            if script.audio[3] > 31 || start >= end || end > file.size.div_ceil(2048) {
                return Err("Invalid Tenchu 2 story audio cue".into());
            }
            Some(crate::audio::stream::Track {
                path,
                start,
                end,
                channel: script.audio[3],
                repeat: script.audio[2] != 0,
            })
        };
        let area = match script.area {
            Some(area) => area,
            None => crate::slice(&module, 0x6f98, 142 * 4)?
                .chunks_exact(4)
                .find(|row| u16_le(row, 0) == Ok(event))
                .and_then(|row| usize::try_from(i16_le(row, 2).ok()?).ok())
                .filter(|&area| area < 24)
                .ok_or("Missing Tenchu 2 story area")?,
        };
        let textures = volume.read(r"o:\cdimage\game\cab\event", &format!("E{event}.ARC"))?;
        let count = u32_le(&textures, 0)? as usize;
        if count > 32 {
            return Err("Excessive Tenchu 2 story texture groups".into());
        }
        let archive = textures
            .get((count + 1) * 4..)
            .ok_or("Truncated Tenchu 2 story textures")?;
        if u32_le(archive, 0)? as usize != count {
            return Err("Mismatched Tenchu 2 story texture groups".into());
        }
        let mut base = super::super::weapons::textures(disc)?;
        let mut characters: Vec<Vram> = Vec::new();
        for index in 0..count {
            let at = u32_le(archive, 4 + index * 4)? as usize;
            let end = if index + 1 < count {
                u32_le(archive, 8 + index * 4)? as usize
            } else {
                archive.len()
            };
            let group = crate::slice(
                archive,
                at,
                end.checked_sub(at)
                    .ok_or("Reversed Tenchu 2 story texture groups")?,
            )?;
            if u32_le(&textures, 4 + index * 4)? == 0 {
                upload(&mut base, group)?;
            } else {
                let mut vram = Vram::default();
                upload(&mut vram, group)?;
                characters.push(vram);
            }
        }
        let files = Files { volume, cab };
        let portraits = script
            .portraits
            .iter()
            .map(|&id| {
                Picture::parse(&files.read(
                    r"o:\cdimage\game\event",
                    &format!("{}.tim", model_name(&module, id)?),
                )?)
            })
            .collect::<Result<_>>()?;
        let pictures = script
            .pictures
            .iter()
            .map(|id| Picture::parse(&files.read(r"o:\cdimage\game\event", &format!("P{id}.tim"))?))
            .collect::<Result<_>>()?;
        let mut models = BTreeMap::new();
        let mut effects = BTreeMap::new();
        let mut faces = BTreeMap::new();
        let mut next_character = 0;
        for animation in &script.actors {
            let id = animation.model;
            if models.contains_key(&id) || effects.contains_key(&id) {
                continue;
            }
            let name = model_name(&module, id)?;
            if id < 106
                && let Some(archive) =
                    files.optional(r"o:\cdimage\game\event", &format!("{name}.arc"))?
            {
                let count = u32_le(&archive, 0)? as usize;
                if !(26..=256).contains(&count) {
                    return Err("Invalid Tenchu 2 facial texture count".into());
                }
                let offset = if id < 11 {
                    [0; 2]
                } else {
                    [u16_le(&module, 0x6920)?, u16_le(&module, 0x6922)?]
                };
                faces.insert(
                    id,
                    (0..count)
                        .map(|i| Patch::parse(arc_entry(&archive, i)?, offset))
                        .collect::<Result<_>>()?,
                );
            }
            if (285..=289).contains(&id) {
                let sprite = match id {
                    286 => Some(Sprite {
                        name,
                        size: 32,
                        texture: None,
                    }),
                    287 | 288 => {
                        let bytes = files.read(r"o:\cdimage\game\event", &format!("{name}.spr"))?;
                        let mut vram = Vram::default();
                        let image = super::super::texture::upload(&mut vram, &bytes)?;
                        Some(Sprite {
                            name,
                            size: if id == 287 { 8 } else { 192 },
                            texture: Some((vram, image)),
                        })
                    }
                    _ => None,
                };
                effects.insert(id, sprite);
                continue;
            }
            let directory = match id {
                0..=105 | 290..=293 => r"o:\cdimage\game\actor",
                106..=231 => r"o:\cdimage\game\weapon",
                _ => r"o:\cdimage\game\event",
            };
            let parts = super::super::model::rig(&files.read(directory, &format!("{name}.mdl"))?)
                .map_err(|e| format!("{name}: {e}"))?;
            let vram = match id {
                11..=105 => {
                    let vram = characters
                        .get(next_character)
                        .ok_or("Missing Tenchu 2 story character textures")?;
                    next_character += 1;
                    vram.clone()
                }
                232..=284 => {
                    let mut vram = Vram::default();
                    upload(&mut vram, &files.read(directory, &format!("{name}.arc"))?)?;
                    vram
                }
                _ => base.clone(),
            };
            models.insert(id, Model { name, parts, vram });
        }
        for animation in &script.actors {
            if animation.model > 290 {
                let texture = &models
                    .get(&animation.texture_model)
                    .ok_or("Missing Tenchu 2 story clothing owner")?
                    .vram;
                let texture = texture.clone();
                models.get_mut(&animation.model).unwrap().vram = texture;
            }
        }
        for model in models.values_mut() {
            super::super::model::resolve_transparency(model.parts.iter_mut(), &model.vram);
        }
        let registry = super::super::weapons::Weapons::load(disc)?;
        let mut weapons = BTreeMap::new();
        let mut attachments = Vec::new();
        for weapon in &script.weapons {
            let part = registry
                .parts
                .get(weapon.part)
                .ok_or("Missing Tenchu 2 story weapon")?;
            let Some(name) = &part.model else { continue };
            let actor = models
                .get(&script.actors[weapon.actor].model)
                .ok_or("Missing Tenchu 2 story weapon owner")?;
            if part.bone >= actor.parts.len() {
                return Err("Missing Tenchu 2 story weapon joint".into());
            }
            if !weapons.contains_key(name) {
                let mut parts = super::super::model::rig(
                    &files.read(r"o:\cdimage\game\weapon", &format!("{name}.mdl"))?,
                )?;
                super::super::model::resolve_transparency(parts.iter_mut(), &base);
                weapons.insert(
                    name.clone(),
                    Model {
                        name: name.clone(),
                        parts,
                        vram: base.clone(),
                    },
                );
            }
            attachments.push(Attachment {
                actor: weapon.actor,
                slot: part.slot,
                bone: part.bone,
                model: name.clone(),
            });
        }
        Ok(Self {
            area,
            results_fade: i16_le(&module, 0x68e4 + area * 2)?,
            script,
            models,
            effects,
            weapons,
            attachments,
            faces,
            audio,
            portraits,
            pictures,
        })
    }
}

fn upload(vram: &mut Vram, archive: &[u8]) -> Result<()> {
    let count = u16_le(archive, 0)? as usize;
    if count > 256 {
        return Err("Excessive Tenchu 2 story texture count".into());
    }
    for index in 0..count {
        super::super::texture::upload(vram, arc_entry(archive, index)?)?;
    }
    Ok(())
}

fn model_name(module: &[u8], id: i16) -> Result<String> {
    let id = usize::try_from(id)
        .ok()
        .filter(|&id| id < 294)
        .ok_or("Invalid Tenchu 2 story model")?;
    let table = if id < 106 { 0x699c } else { 0x6b44 };
    let index = if id < 106 { id } else { id - 106 };
    let address = u32_le(module, table + index * 4)?;
    let offset = address
        .checked_sub(0x800d0610)
        .ok_or("Invalid Tenchu 2 story model name")? as usize;
    let name = module_string(module, offset)?;
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err("Invalid Tenchu 2 story model name".into());
    }
    Ok(name.to_ascii_lowercase())
}

fn module_string(module: &[u8], offset: usize) -> Result<&str> {
    let bytes = module
        .get(offset..)
        .ok_or("Tenchu 2 story string outside module")?;
    let length = bytes
        .iter()
        .position(|&c| c == 0)
        .ok_or("Unterminated Tenchu 2 story string")?;
    std::str::from_utf8(&bytes[..length]).map_err(|_| "Invalid Tenchu 2 story string".into())
}
