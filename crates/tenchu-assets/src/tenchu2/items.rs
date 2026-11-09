use super::volume::Volume;
use crate::{
    ReadAt, Result,
    disc::Disc,
    model::{Part, arc_entry},
    texture::{Image, Vram},
};
use std::collections::BTreeMap;

pub struct Items {
    pub vram: Vram,
    pub images: Vec<Image>,
    models: BTreeMap<u8, Vec<Part>>,
}

impl Items {
    pub fn models(&self) -> impl Iterator<Item = (u8, &[Part])> {
        self.models
            .iter()
            .map(|(&id, parts)| (id, parts.as_slice()))
    }
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let volume = Volume::open(disc)?;
        let directory = r"o:\cdimage\game\item";
        let data = volume.read(directory, "item.arc")?;
        let count = crate::u32_le(&data, 0)? as usize;
        if !(65..=256).contains(&count) {
            return Err("Invalid Tenchu 2 item texture archive".into());
        }
        let mut vram = Vram::default();
        let images = (0..count)
            .map(|i| super::texture::upload(&mut vram, arc_entry(&data, i)?))
            .collect::<Result<_>>()?;
        let models = [
            (0, "kagihead"),
            (1, "jinmyo"),
            (2, "dokumyo"),
            (4, "syuriken"),
            (5, "makibisi"),
            (6, "kansyaku"),
            (7, "jirai"),
            (10, "enmaku"),
            (11, "dango"),
            (15, "nigiri"),
            (18, "nemuri"),
            (20, "utusemi"),
            (22, "ya"),
            (23, "tutu"),
            (24, "yari"),
            (30, "kunai"),
            (31, "tudura"),
            (32, "goshiki1"),
            (33, "goshiki2"),
            (34, "goshiki3"),
            (35, "goshiki4"),
            (36, "goshiki5"),
        ]
        .into_iter()
        .map(|(id, name)| {
            let bytes = volume.read(directory, &format!("{name}.mdl"))?;
            Ok((id, super::model::rig(&bytes)?))
        })
        .collect::<Result<_>>()?;
        Ok(Self {
            vram,
            images,
            models,
        })
    }

    pub fn model(&self, id: u8) -> Option<&[Part]> {
        if id > 36 {
            return None;
        }
        let id = match id {
            9 => 22,
            12 => 7,
            _ => id,
        };
        self.models
            .get(&id)
            .or_else(|| self.models.get(&4))
            .map(Vec::as_slice)
    }
}
