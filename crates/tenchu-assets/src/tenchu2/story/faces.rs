use crate::{ReadAt, Result, disc::Disc, slice, texture::Vram, u16_le, u32_le};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Patch {
    rect: [u16; 4],
    words: Vec<u16>,
}

impl Patch {
    pub fn for_characters(
        disc: &Disc<impl ReadAt>,
        names: &[&str],
    ) -> Result<BTreeMap<String, Vec<Self>>> {
        let volume = super::Volume::open(disc)?;
        let module = volume.read(r"o:\cdimage\game\module", "event.bin")?;
        let origin = match volume.edition {
            super::Edition::Japan => 0x6920,
            super::Edition::Europe => 0x6000,
        };
        let npc_offset = [u16_le(&module, origin)?, u16_le(&module, origin + 2)?];
        let offset = |name| {
            if super::super::characters::hero(name).is_some() {
                [0; 2]
            } else {
                npc_offset
            }
        };
        let mut found = BTreeMap::new();
        let mut models = BTreeMap::new();
        let directory = r"o:\cdimage\game\event";
        let actor_directory = r"o:\cdimage\game\actor";
        let wanted: BTreeMap<_, _> = names
            .iter()
            .map(|&name| (super::super::hash(&format!("{name}.arc")), name))
            .collect();
        for &name in names {
            if let Some(bytes) = volume.optional(directory, &format!("{name}.arc"))? {
                found.insert(name.to_owned(), Self::archive(&bytes, offset(name))?);
            }
        }
        for bytes in volume.directory(r"o:\cdimage\game\cab\event") {
            let bytes = bytes?;
            let count = u32_le(&bytes, 0)? as usize;
            // The companion ARC starts with texture-group flags, not CAB offsets.
            if count == 0 || count > 4096 || (u32_le(&bytes, 4)? as usize) < count * 12 {
                continue;
            }
            let rows = slice(&bytes, 4, count * 12)?;
            for row in rows.chunks_exact(12) {
                if u32_le(row, 4)? != super::super::hash(directory) {
                    continue;
                }
                let Some(&name) = wanted.get(&u32_le(row, 8)?) else {
                    continue;
                };
                let model_name = format!("{name}.mdl");
                if rows.chunks_exact(12).any(|row| {
                    u32_le(row, 4) == Ok(super::super::hash(actor_directory))
                        && u32_le(row, 8) == Ok(super::super::hash(&model_name))
                }) {
                    if !models.contains_key(name) {
                        models.insert(name.to_owned(), volume.read(actor_directory, &model_name)?);
                    }
                    // Some scenes use a younger or otherwise different body with the same name.
                    if super::cab_member(&bytes, actor_directory, &model_name)? != models[name] {
                        continue;
                    }
                }
                let patches = Self::archive(
                    super::cab_member(&bytes, directory, &format!("{name}.arc"))?,
                    offset(name),
                )?;
                if let Some(previous) = found.get(name) {
                    if previous != &patches {
                        return Err(format!("Conflicting compatible Tenchu 2 faces for {name}"));
                    }
                } else {
                    found.insert(name.to_owned(), patches);
                }
            }
        }
        Ok(found)
    }

    fn archive(bytes: &[u8], offset: [u16; 2]) -> Result<Vec<Self>> {
        let count = u32_le(bytes, 0)? as usize;
        if !(26..=256).contains(&count) {
            return Err("Invalid Tenchu 2 facial texture count".into());
        }
        (0..count)
            .map(|i| Self::parse(crate::model::arc_entry(bytes, i)?, offset))
            .collect()
    }

    pub fn valid(&self) -> bool {
        let [x, y, w, h] = self.rect.map(usize::from);
        x + w <= 1024 && y + h <= 512 && self.words.len() == w * h
    }

    pub(super) fn parse(data: &[u8], offset: [u16; 2]) -> Result<Self> {
        if u32_le(data, 0)? != 0x10 {
            return Err("Invalid Tenchu 2 facial texture".into());
        }
        let at = 8 + if u32_le(data, 4)? & 8 != 0 {
            u32_le(data, 8)? as usize
        } else {
            0
        };
        let image = slice(data, at, u32_le(data, at)? as usize)?;
        let [x, y, w, h] = [
            u16_le(image, 4)?,
            u16_le(image, 6)?,
            u16_le(image, 8)?,
            u16_le(image, 10)?,
        ];
        let x = usize::from(x) + usize::from(offset[0]);
        let y = usize::from(y) + usize::from(offset[1]);
        if x + usize::from(w) > 1024 || y + usize::from(h) > 512 {
            return Err("Tenchu 2 facial texture outside atlas".into());
        }
        let words = slice(image, 12, usize::from(w) * usize::from(h) * 2)?
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap()))
            .collect();
        Ok(Self {
            rect: [x as u16, y as u16, w, h],
            words,
        })
    }

    pub fn apply(&self, vram: &mut Vram) {
        let [x, y, w, h] = self.rect.map(usize::from);
        for row in 0..h {
            let start = (y + row) * 1024 + x;
            vram.words[start..start + w].copy_from_slice(&self.words[row * w..(row + 1) * w]);
        }
        vram.revision = vram.revision.wrapping_add(1);
    }

    pub fn rect(&self) -> [u16; 4] {
        self.rect
    }
}
