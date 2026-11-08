use super::volume::Volume;
use crate::{
    ReadAt, Result,
    disc::Disc,
    model::arc_entry,
    texture::{Image, Vram},
    u32_le,
};

pub struct Sprites {
    pub vram: Vram,
    pub effects: Vec<Image>,
    pub scenery: Vec<Image>,
}

impl Sprites {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let volume = Volume::open(disc)?;
        let mut vram = Vram::default();
        let mut load = |directory, name| {
            let bytes = volume.read(directory, name)?;
            let count = u32_le(&bytes, 0)? as usize;
            if count > 256 {
                return Err("Excessive Tenchu 2 effect texture count".into());
            }
            (0..count)
                .map(|i| vram.upload_tim(arc_entry(&bytes, i)?))
                .collect::<Result<Vec<_>>>()
        };
        let effects = load(r"o:\cdimage\game\effect", "effect.arc")?;
        let scenery = load(r"o:\cdimage\game\stage", "objectef.arc")?;
        if effects.len() < 36 || scenery.len() < 8 {
            return Err("Missing Tenchu 2 effect textures".into());
        }
        Ok(Self {
            vram,
            effects,
            scenery,
        })
    }
}
