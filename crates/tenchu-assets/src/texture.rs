use crate::{Result, relative, slice, u16_le, u32_le};

pub struct Vram {
    pub words: Vec<u16>,
}

impl Default for Vram {
    fn default() -> Self {
        Self {
            words: vec![0; 1024 * 512],
        }
    }
}

impl Vram {
    pub fn upload_tim(&mut self, data: &[u8]) -> Result<()> {
        if u32_le(data, 0)? != 0x10 {
            return Err("Invalid TIM signature".into());
        }
        let flags = u32_le(data, 4)?;
        let mut at = 8;
        for _ in 0..if flags & 8 != 0 { 2 } else { 1 } {
            let size = u32_le(data, at)? as usize;
            let block = slice(data, at, size)?;
            let x = usize::from(u16_le(block, 4)?);
            let y = usize::from(u16_le(block, 6)?);
            let w = usize::from(u16_le(block, 8)?);
            let h = usize::from(u16_le(block, 10)?);
            if x + w > 1024 || y + h > 512 {
                return Err("TIM block outside VRAM".into());
            }
            let pixels = slice(block, 12, w * h * 2)?;
            for row in 0..h {
                for col in 0..w {
                    self.words[(y + row) * 1024 + x + col] = u16_le(pixels, (row * w + col) * 2)?;
                }
            }
            at += size;
        }
        Ok(())
    }

    pub fn upload_tpd(&mut self, data: &[u8]) -> Result<()> {
        if u32_le(data, 0)? != 0x96082100 {
            return Err("Invalid TPD signature".into());
        }
        let count = u32_le(data, 4)? as usize;
        if count > 65536 {
            return Err("Excessive TPD image count".into());
        }
        slice(data, 8, count * 4)?;
        for i in 0..count {
            let at = relative(data, 8, 8 + i * 4)?;
            self.upload_tim(&data[at..])?;
        }
        Ok(())
    }
}
