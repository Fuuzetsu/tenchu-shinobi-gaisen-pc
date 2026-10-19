use crate::{Result, executable::Executable, i16_le, u16_le, u32_le};

#[derive(Clone, Copy, Debug)]
pub struct MapPlacement {
    pub divisor: i32,
    pub rotation: i32,
    pub screen: [i32; 2],
}

impl MapPlacement {
    pub fn load(exe: &Executable, stage: usize) -> Result<Self> {
        if stage >= 11 {
            return Err("Invalid map placement stage".into());
        }
        let row = exe.bytes(0x8008e50c + stage as u32 * 16, 16)?;
        Ok(Self {
            divisor: u32_le(row, 0)? as i32,
            rotation: u32_le(row, 4)? as i32,
            screen: [u32_le(row, 8)? as i32, u32_le(row, 12)? as i32],
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LifeStyle {
    pub base: u16,
    pub scale: i16,
    pub number: [i16; 2],
    pub rotation: i32,
    pub images: [u8; 2],
}

pub fn life_styles(exe: &Executable) -> Result<[LifeStyle; 2]> {
    let read = |i: u32| -> Result<LifeStyle> {
        let style = exe.bytes(0x8008e414 + i * 80, 8)?;
        let parts = exe.bytes(0x8008e4b4 + i * 8, 6)?;
        Ok(LifeStyle {
            base: u16_le(style, 0)?,
            scale: i16_le(style, 2)?,
            number: [i16_le(style, 4)?, i16_le(style, 6)?],
            rotation: u32_le(parts, 0)? as i32,
            images: [parts[4], parts[5]],
        })
    };
    Ok([read(0)?, read(1)?])
}
