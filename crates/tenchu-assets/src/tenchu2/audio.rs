use super::{Edition, volume::Volume};
use crate::{ReadAt, Result, audio::bank::Bank, disc::Disc, executable::Executable, i16_le};

pub struct Audio {
    pub bank: Bank,
    pub interval: i16,
}

pub fn menu(disc: &Disc<impl ReadAt>) -> Result<Bank> {
    Bank::parse(&Volume::open(disc)?.read(r"o:\cdimage\menu\sound", "menu.vab")?)
}

pub fn interval(disc: &Disc<impl ReadAt>, map: usize) -> Result<i16> {
    if map >= 24 {
        return Err("Invalid Tenchu 2 ambience map".into());
    }
    let (edition, bytes) = super::executable(disc)?;
    let exe = Executable::new(&bytes)?;
    i16_le(
        exe.bytes(edition.addresses().ambience_intervals + map as u32 * 2, 2)?,
        0,
    )
}

impl Audio {
    pub fn load(disc: &Disc<impl ReadAt>, hero: usize, map: usize) -> Result<Self> {
        let (edition, bytes) = super::executable(disc)?;
        if edition != Edition::Japan || hero >= 3 || map >= 24 {
            return Err("Native Tenchu 2 audio requires a Japanese campaign cast".into());
        }
        let exe = Executable::new(&bytes)?;
        let prefix = exe.bytes(0x8008e4d0 + hero as u32, 1)?[0];
        if !prefix.is_ascii_alphabetic() {
            return Err("Invalid Tenchu 2 sound bank prefix".into());
        }
        let name = format!("{}_E_{map:02}.vab", char::from(prefix));
        Ok(Self {
            bank: Bank::parse(&Volume::open(disc)?.read(r"o:\cdimage\game\sound", &name)?)?,
            interval: i16_le(
                exe.bytes(edition.addresses().ambience_intervals + map as u32 * 2, 2)?,
                0,
            )?,
        })
    }
}
