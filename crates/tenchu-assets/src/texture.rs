use crate::{Result, relative, slice, u16_le, u32_le};

#[derive(Clone, Copy, Debug)]
pub struct Image {
    pub page: u16,
    pub palette: u16,
    pub uv: [u8; 2],
    pub size: [u16; 2],
}

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
    pub fn upload_tim(&mut self, data: &[u8]) -> Result<Image> {
        if u32_le(data, 0)? != 0x10 {
            return Err("Invalid TIM signature".into());
        }
        let flags = u32_le(data, 4)?;
        let mode = flags & 3;
        if mode == 3 {
            return Err("24-bit TIM textures are not supported".into());
        }
        let mut at = 8;
        let mut palette = 0;
        let mut image = None;
        for i in 0..if flags & 8 != 0 { 2 } else { 1 } {
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
            if i == 0 && flags & 8 != 0 {
                palette = ((y << 6) | (x >> 4)) as u16;
            } else {
                let shift = 2 - mode;
                image = Some(Image {
                    page: ((mode << 7) | ((y as u32 & 256) >> 4) | ((x as u32 & 1023) >> 6)) as u16,
                    palette,
                    uv: [
                        (((x as u32) << shift) & ((1 << (8 - mode)) - 1)) as u8,
                        y as u8,
                    ],
                    size: [(w as u16) << shift, h as u16],
                });
            }
            at += size;
        }
        image.ok_or_else(|| "TIM has no pixels".into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tim_palette_and_sprite_coordinates_use_the_source_pixel_mode() {
        let mut data = Vec::new();
        data.extend(0x10u32.to_le_bytes());
        data.extend(9u32.to_le_bytes());
        for (rect, pixels) in [
            ([32u16, 480, 256, 1], vec![0x8421u16; 256]),
            ([368, 255, 1, 1], vec![0x0201]),
        ] {
            data.extend((12u32 + pixels.len() as u32 * 2).to_le_bytes());
            for value in rect {
                data.extend(value.to_le_bytes());
            }
            for value in pixels {
                data.extend(value.to_le_bytes());
            }
        }
        let mut vram = Vram::default();
        let image = vram.upload_tim(&data).unwrap();
        assert_eq!(
            (image.page, image.palette, image.uv, image.size),
            (133, 30722, [96, 255], [2, 1])
        );
        assert_eq!(vram.words[480 * 1024 + 32], 0x8421);
        assert_eq!(vram.words[255 * 1024 + 368], 0x0201);
        data.pop();
        assert!(vram.upload_tim(&data).is_err());
    }
}
