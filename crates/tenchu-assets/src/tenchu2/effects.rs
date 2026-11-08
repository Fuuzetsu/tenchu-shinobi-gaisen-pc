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

#[derive(Clone, Debug)]
pub struct Rules {
    pub burn_bones: [u8; 14],
    pub healing_bones: [u8; 6],
}

impl Rules {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, bytes) = super::executable(disc)?;
        if edition != super::Edition::Japan {
            return Err("Native Tenchu 2 effects require the Japanese disc".into());
        }
        let executable = crate::executable::Executable::new(&bytes)?;
        Ok(Self {
            burn_bones: executable.bytes(0x80012a08, 14)?.try_into().unwrap(),
            healing_bones: executable.bytes(0x8008e928, 6)?.try_into().unwrap(),
        })
    }
}
