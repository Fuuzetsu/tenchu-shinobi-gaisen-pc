use crate::{
    ReadAt, Result,
    executable::Executable,
    game::GameAssets,
    texture::{Image, Vram},
};

pub struct Font {
    pub image: Image,
    widths: Vec<u8>,
}

fn index(code: u8) -> usize {
    let code = if code == 0x92 { b'\'' } else { code };
    usize::from(code) - if code >= 0x20 { 0x20 } else { 0 } - if code >= 0xc0 { 0x40 } else { 0 }
}

impl Font {
    pub fn load<S: ReadAt>(game: &GameAssets<S>, vram: &mut Vram) -> Result<Self> {
        Ok(Self {
            image: vram.upload_tim(&game.asset("IMAGE/FONT.TIM")?)?,
            widths: Executable::new(&game.executable)?
                .bytes(0x8008ef98, 224)?
                .to_vec(),
        })
    }

    pub fn width(&self, code: u8) -> i32 {
        i32::from(self.widths[index(code)])
    }

    pub fn glyph(&self, code: u8) -> (Image, i16) {
        let cell = index(code);
        let mode = (self.image.page >> 7) & 3;
        let shift = 2 - mode;
        let x = (self.image.page & 15) * 64
            + (u16::from(self.image.uv[0]) >> shift)
            + (cell as u16 & 15) * 3;
        let y = (self.image.page & 16) * 16 + u16::from(self.image.uv[1]) + (cell as u16 / 16) * 16;
        let nudge = match code {
            0xc0..=0xdf if code != 0xc7 => -4,
            0xe7 => 3,
            0xe0..=0xff => -2,
            _ => 0,
        };
        (
            Image {
                page: (mode << 7) | ((y & 256) >> 4) | ((x & 1023) >> 6) | 32,
                palette: self.image.palette,
                uv: [((x << shift) & ((1 << (8 - mode)) - 1)) as u8, y as u8],
                size: [3 << shift, 16],
            },
            nudge,
        )
    }
}
