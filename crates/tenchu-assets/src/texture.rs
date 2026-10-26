use crate::{Result, relative, slice, u16_le, u32_le};

/// Alpha retains the transparent, semi-transparent and opaque PS1 texture classes.
pub fn rgba8(word: u16) -> [u8; 4] {
    let [r, g, b] = [0, 5, 10].map(|shift| (((word >> shift) & 31) * 255 / 31) as u8);
    let alpha = if word == 0 {
        0
    } else if word & 0x8000 != 0 {
        128
    } else {
        255
    };
    [r, g, b, alpha]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Image {
    pub page: u16,
    pub palette: u16,
    pub uv: [u8; 2],
    pub size: [u16; 2],
}

impl Image {
    pub fn word_rect(self) -> [u16; 4] {
        let shift = 2 - ((self.page >> 7) & 3);
        [
            (self.page & 15) * 64 + (u16::from(self.uv[0]) >> shift),
            (self.page & 16) * 16 + u16::from(self.uv[1]),
            self.size[0] >> shift,
            self.size[1],
        ]
    }
}

#[derive(Clone)]
pub struct ScrollingImage {
    pub image: Image,
    pub source: [u16; 2],
    pub offset: [u16; 2],
}

#[derive(Clone)]
pub struct Vram {
    pub words: Vec<u16>,
    pub revision: u64,
    pub images: Vec<Image>,
    pub scrolling: Vec<ScrollingImage>,
}

impl Default for Vram {
    fn default() -> Self {
        Self {
            words: vec![0; 1024 * 512],
            revision: 0,
            images: Vec::new(),
            scrolling: Vec::new(),
        }
    }
}

impl Vram {
    pub fn texel(&self, page: u16, palette: u16, uv: [u16; 2]) -> u16 {
        let word = |x: usize, y: usize| self.words[(y & 511) * 1024 + (x & 1023)];
        let [u, v] = uv.map(|n| usize::from(n & 255));
        let x = usize::from(page & 15) * 64;
        let y = usize::from(page & 16) * 16;
        match (page >> 7) & 3 {
            0 => {
                let index = (word(x + u / 4, y + v) >> ((u & 3) * 4)) & 15;
                word(
                    usize::from((palette & 63) * 16 + index),
                    usize::from(palette >> 6),
                )
            }
            1 => {
                let index = (word(x + u / 2, y + v) >> ((u & 1) * 8)) & 255;
                word(
                    usize::from((palette & 63) * 16 + index),
                    usize::from(palette >> 6),
                )
            }
            _ => word(x + u, y + v),
        }
    }

    pub fn copy_rect(&mut self, source: [u16; 4], destination: [u16; 2]) -> Result<()> {
        let [x, y, w, h] = source.map(usize::from);
        let [dx, dy] = destination.map(usize::from);
        if x + w > 1024 || y + h > 512 || dx + w > 1024 || dy + h > 512 {
            return Err("Texture copy outside VRAM".into());
        }
        let pixels: Vec<_> = (y..y + h)
            .flat_map(|row| {
                self.words[row * 1024 + x..row * 1024 + x + w]
                    .iter()
                    .copied()
            })
            .collect();
        for row in 0..h {
            self.words[(dy + row) * 1024 + dx..(dy + row) * 1024 + dx + w]
                .copy_from_slice(&pixels[row * w..(row + 1) * w]);
        }
        for scroll in &mut self.scrolling {
            if w != 0
                && h != 0
                && scroll.image.word_rect() == [dx as u16, dy as u16, w as u16, h as u16]
            {
                let scale = 4 >> ((scroll.image.page >> 7) & 3);
                scroll.offset = [
                    source[0].wrapping_sub(scroll.source[0]) % w as u16 * scale,
                    source[1].wrapping_sub(scroll.source[1]) % h as u16,
                ];
            }
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

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
        let image = image.ok_or("TIM has no pixels")?;
        self.images.push(image);
        self.revision = self.revision.wrapping_add(1);
        Ok(image)
    }

    pub fn upload_tpd(&mut self, data: &[u8]) -> Result<()> {
        self.upload_tpd_images(data).map(|_| ())
    }

    pub fn upload_tpd_images(&mut self, data: &[u8]) -> Result<Vec<Image>> {
        if u32_le(data, 0)? != 0x96082100 {
            return Err("Invalid TPD signature".into());
        }
        let count = u32_le(data, 4)? as usize;
        if count > 65536 {
            return Err("Excessive TPD image count".into());
        }
        slice(data, 8, count * 4)?;
        (0..count)
            .map(|i| {
                let at = relative(data, 8, 8 + i * 4)?;
                self.upload_tim(&data[at..])
            })
            .collect()
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
        assert_eq!(image.word_rect(), [368, 255, 1, 1]);
        assert_eq!(vram.words[480 * 1024 + 32], 0x8421);
        assert_eq!(vram.words[255 * 1024 + 368], 0x0201);
        data.pop();
        assert!(vram.upload_tim(&data).is_err());
    }

    #[test]
    fn texture_copies_wrap_with_a_tiled_source_without_changing_the_palette() {
        let mut vram = Vram::default();
        for (i, word) in vram.words.iter_mut().enumerate() {
            *word = i as u16;
        }
        let original = vram.words.clone();
        let source = [704, 320, 32, 32];
        for y in 0..2 {
            for x in 0..2 {
                vram.copy_rect(source, [832 + x * 32, 256 + y * 32])
                    .unwrap();
            }
        }
        vram.copy_rect([863, 273, 32, 32], [704, 320]).unwrap();
        for y in 0..32 {
            for x in 0..32 {
                assert_eq!(
                    vram.words[(320 + y) * 1024 + 704 + x],
                    original[(320 + (y + 17) % 32) * 1024 + 704 + (x + 31) % 32]
                );
            }
        }
        assert_eq!(&vram.words[480 * 1024..], &original[480 * 1024..]);
        assert!(vram.copy_rect([1020, 0, 32, 32], [0, 0]).is_err());
    }
}
