use crate::{Result, slice, texture::Vram, u16_le, u32_le};

pub struct Patch {
    rect: [u16; 4],
    words: Vec<u16>,
}

impl Patch {
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
