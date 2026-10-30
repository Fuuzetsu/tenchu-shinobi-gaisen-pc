pub mod mission;

use crate::{
    ReadAt, Result,
    executable::{Executable, MotionRegistration},
    game::GameAssets,
    i16_le,
    model::arc_entry,
    script::Language,
    slice, u16_le, u32_le,
};
use sha2::{Digest, Sha256};

/// A TIM independent of its original VRAM address, including palettes' blend bits.
pub struct Image {
    pub size: [usize; 2],
    pub pixels: Vec<u16>,
    palette: Vec<u16>,
    indices: Vec<u8>,
}

impl Image {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if u32_le(bytes, 0)? != 0x10 {
            return Err("Invalid frontend TIM signature".into());
        }
        let flags = u32_le(bytes, 4)?;
        let mode = flags & 3;
        if mode > 2 || flags & !11 != 0 {
            return Err("Unsupported frontend TIM mode".into());
        }
        let mut at = 8;
        let palette = if flags & 8 != 0 {
            let length = u32_le(bytes, at)? as usize;
            let block = slice(bytes, at, length)?;
            let count = usize::from(u16_le(block, 8)?) * usize::from(u16_le(block, 10)?);
            let palette = slice(block, 12, count * 2)?;
            at += length;
            palette
        } else {
            &[]
        };
        let block = slice(bytes, at, u32_le(bytes, at)? as usize)?;
        let width = usize::from(u16_le(block, 8)?) << (2 - mode);
        let height = usize::from(u16_le(block, 10)?);
        if width == 0 || height == 0 || width > 4096 || height > 512 {
            return Err("Invalid frontend image dimensions".into());
        }
        let count = width * height;
        let data = slice(
            block,
            12,
            match mode {
                0 => count.div_ceil(2),
                1 => count,
                _ => count * 2,
            },
        )?;
        let indices = match mode {
            0 => (0..count)
                .map(|i| (data[i / 2] >> ((i % 2) * 4)) & 15)
                .collect(),
            1 => data.to_vec(),
            _ => Vec::new(),
        };
        let pixels = (0..width * height)
            .map(|i| match mode {
                0 => u16_le(
                    palette,
                    usize::from((data[i / 2] >> ((i % 2) * 4)) & 15) * 2,
                ),
                1 => u16_le(palette, usize::from(data[i]) * 2),
                _ => u16_le(data, i * 2),
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            size: [width, height],
            pixels,
            palette: palette
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect(),
            indices,
        })
    }

    pub fn from_pixels(size: [usize; 2], pixels: Vec<u16>) -> Self {
        assert_eq!(size[0] * size[1], pixels.len());
        Self {
            size,
            pixels,
            palette: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn from_vram(vram: &crate::texture::Vram, image: crate::texture::Image) -> Self {
        let [w, h] = image.size.map(usize::from);
        Self::from_pixels(
            [w, h],
            (0..w * h)
                .map(|i| {
                    vram.texel(
                        image.page,
                        image.palette,
                        [
                            u16::from(image.uv[0]) + (i % w) as u16,
                            u16::from(image.uv[1]) + (i / w) as u16,
                        ],
                    )
                })
                .collect(),
        )
    }

    pub fn palette_color(&self, index: usize) -> Option<u16> {
        self.palette.get(index).copied()
    }

    pub fn recolor(&mut self, index: usize, color: u16) {
        let Some(previous) = self.palette.get_mut(index) else {
            return;
        };
        if *previous == color {
            return;
        }
        *previous = color;
        for (pixel, &entry) in self.pixels.iter_mut().zip(&self.indices) {
            if usize::from(entry) == index {
                *pixel = color;
            }
        }
    }

    pub fn archive(bytes: &[u8]) -> Result<Vec<Self>> {
        (0..usize::from(u16_le(bytes, 0)?))
            .map(|index| Self::parse(arc_entry(bytes, index)?))
            .collect()
    }
}

pub struct Tables {
    pub blood_order: [usize; 9],
    pub cursor_targets: [[i16; 2]; 10],
    pub cursor_starts: [[i16; 2]; 10],
    pub motions: [Vec<MotionRegistration>; 2],
    pub sound_test_music: [usize; 19],
    pub control_positions: [[i16; 2]; 8],
    pub control_actions: [[usize; 8]; 4],
    pub button_assign: [[u8; 8]; 4],
}

impl Tables {
    pub fn load<S: ReadAt>(assets: &GameAssets<S>) -> Result<Self> {
        let file = assets.disc.find("TENCHU/MENU.EXE")?;
        let bytes = assets.disc.file(&file).read(0, file.size as usize)?;
        if format!("{:x}", Sha256::digest(&bytes))
            != "5de759508d87fa3bd3122329eaaf00467ff5f4d964a1673699309f3b6983c5ab"
        {
            return Err("Unsupported MENU.EXE; expected original SLPS-019.01".into());
        }
        let exe = Executable::new(&bytes)?;
        let pairs = |address: u32, stride: u32| -> Result<Vec<[i16; 2]>> {
            (0..10)
                .map(|i| {
                    let row = exe.bytes(address + i * stride, 4)?;
                    Ok([i16_le(row, 0)?, i16_le(row, 2)?])
                })
                .collect()
        };
        let shorts = |address: u32, count: usize| -> Result<Vec<i16>> {
            exe.bytes(address, count * 2)?
                .chunks_exact(2)
                .map(|b| i16_le(b, 0))
                .collect()
        };
        let blood = shorts(0x800506d0, 9)?;
        let music = exe
            .bytes(0x800526d8, 19 * 4)?
            .chunks_exact(4)
            .map(|b| u32_le(b, 0).map(|n| n as usize))
            .collect::<Result<Vec<_>>>()?;
        let actions = shorts(0x80052698, 32)?;
        // ControlLabelPositions stores four localized coordinate pairs per action.
        let positions = (0..8)
            .map(|i| {
                let row = exe.bytes(0x80052618 + i * 16 + 12, 4)?;
                Ok([i16_le(row, 0)?, i16_le(row, 2)?])
            })
            .collect::<Result<Vec<_>>>()?;
        let main = Executable::new(&assets.executable)?;
        let buttons = main.bytes(0x80011210, 32)?;
        Ok(Self {
            blood_order: std::array::from_fn(|i| blood[i] as usize),
            cursor_targets: pairs(0x800528cc, 4)?.try_into().unwrap(),
            cursor_starts: pairs(0x800528f4, 4)?.try_into().unwrap(),
            motions: [
                exe.motion_registry(0x80050ce4)?,
                exe.motion_registry(0x80050d2c)?,
            ],
            sound_test_music: music.try_into().unwrap(),
            control_positions: positions.try_into().unwrap(),
            control_actions: std::array::from_fn(|i| {
                std::array::from_fn(|j| actions[i * 8 + j] as usize)
            }),
            button_assign: std::array::from_fn(|i| buttons[i * 8..i * 8 + 8].try_into().unwrap()),
        })
    }
}

pub fn language_suffix(language: Language) -> char {
    match language {
        Language::English => 'e',
        Language::French => 'f',
        Language::Italian => 'i',
        Language::Japanese => 'j',
    }
}

pub fn sound_directory(language: Language) -> &'static str {
    match language {
        Language::English => "SOUND/ENGLISH",
        Language::French => "SOUND/FRENCH",
        Language::Italian => "SOUND/ITALIAN",
        Language::Japanese => "SOUND",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movie {
    Opening,
    Training,
    Rikimaru,
    Ayame,
    Credits,
}

impl Movie {
    pub fn path(self) -> &'static str {
        match self {
            Self::Opening => "TENCHU/MOVIE/OPEN06.STR",
            Self::Training => "TENCHU/MOVIE/TRAINING.STR",
            Self::Rikimaru => "TENCHU/MOVIE/ENDRIKI.STR",
            Self::Ayame => "TENCHU/MOVIE/ENDAYA.STR",
            Self::Credits => "TENCHU/MOVIE/ENDING.STR",
        }
    }
    pub fn last_frame(self) -> u32 {
        match self {
            Self::Opening => 1879,
            Self::Training => 247,
            Self::Rikimaru | Self::Ayame => 627,
            Self::Credits => 5295,
        }
    }
    pub fn skippable(self) -> bool {
        !matches!(self, Self::Rikimaru | Self::Ayame)
    }
    pub fn subtitles(self, language: Language) -> Option<String> {
        let name = match self {
            Self::Opening => return None,
            Self::Training => "STAGE14R",
            Self::Rikimaru => "STAGE12R",
            Self::Ayame => "STAGE12A",
            Self::Credits => "STAGE13R",
        };
        Some(format!("{}/{name}.CAD", language.directory()))
    }
    pub fn voices(self, language: Language) -> Option<String> {
        let name = match self {
            Self::Opening => return None,
            Self::Training => "TRAINING",
            Self::Rikimaru => "ENDR",
            Self::Ayame => "ENDA",
            Self::Credits => "ENDC",
        };
        Some(format!("{}/{name}.VAB", sound_directory(language)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptionCommand {
    Wait(u32),
    Voice(i16),
    Text(Vec<u8>),
    Clear,
}

pub fn movie_commands(bytes: &[u8]) -> Result<Vec<CaptionCommand>> {
    let mut commands = Vec::new();
    for at in (0..bytes.len()).step_by(12) {
        let row = slice(bytes, at, 12)?;
        let value = i16_le(row, 2)?;
        match i16_le(row, 0)? {
            0 => (),
            1 if value >= 0 => {
                commands.push(CaptionCommand::Wait(value as u32));
                if value == 0 {
                    return Ok(commands);
                }
            }
            5 => commands.push(CaptionCommand::Voice(i16_le(row, 4)?)),
            8 if value == -1 => commands.push(CaptionCommand::Clear),
            8 if value >= 0 => {
                let text = bytes
                    .get(value as usize..)
                    .ok_or("Movie caption offset outside asset")?;
                let end = text
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or("Unterminated movie caption")?;
                commands.push(CaptionCommand::Text(text[..end].to_vec()));
            }
            operation => return Err(format!("Invalid movie command {operation} at {at}")),
        }
    }
    Err("Movie commands have no terminating wait".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn title_palette_changes_reveal_only_the_addressed_pixels() {
        let mut bytes = Vec::new();
        for n in [0x10u32, 8, 44] {
            bytes.extend(n.to_le_bytes());
        }
        for n in [0u16, 0, 16, 1] {
            bytes.extend(n.to_le_bytes());
        }
        for n in 0u16..16 {
            bytes.extend(n.to_le_bytes());
        }
        bytes.extend(14u32.to_le_bytes());
        for n in [0u16, 0, 1, 1] {
            bytes.extend(n.to_le_bytes());
        }
        bytes.extend([0x10, 0x11]);
        let mut image = Image::parse(&bytes).unwrap();
        assert_eq!(image.pixels, [0, 1, 1, 1]);
        image.recolor(1, 0);
        assert_eq!(image.pixels, [0; 4]);
        image.recolor(1, 0x801f);
        assert_eq!(image.pixels, [0, 0x801f, 0x801f, 0x801f]);
    }
    #[test]
    fn tim_retains_transparent_black_and_blend_bits_across_wide_images() {
        let mut bytes = Vec::new();
        for n in [0x10u32, 2, 12 + 600 * 2] {
            bytes.extend(n.to_le_bytes());
        }
        for n in [0u16, 0, 600, 1] {
            bytes.extend(n.to_le_bytes());
        }
        for i in 0..600 {
            bytes.extend((if i % 2 == 0 { 0u16 } else { 0x8000 }).to_le_bytes());
        }
        let image = Image::parse(&bytes).unwrap();
        assert_eq!(image.size, [600, 1]);
        assert_eq!(&image.pixels[598..], &[0, 0x8000]);
        bytes.pop();
        assert!(Image::parse(&bytes).is_err());
    }
    #[test]
    fn movie_caption_offsets_and_termination_are_checked() {
        let mut bytes = vec![0; 49];
        bytes[0..2].copy_from_slice(&8i16.to_le_bytes());
        bytes[2..4].copy_from_slice(&48i16.to_le_bytes());
        bytes[12..14].copy_from_slice(&1i16.to_le_bytes());
        bytes[14..16].copy_from_slice(&15i16.to_le_bytes());
        bytes[24..26].copy_from_slice(&5i16.to_le_bytes());
        bytes[28..30].copy_from_slice(&7i16.to_le_bytes());
        bytes[36..38].copy_from_slice(&1i16.to_le_bytes());
        assert_eq!(
            movie_commands(&bytes).unwrap(),
            vec![
                CaptionCommand::Text(vec![]),
                CaptionCommand::Wait(15),
                CaptionCommand::Voice(7),
                CaptionCommand::Wait(0)
            ]
        );
        bytes[2..4].copy_from_slice(&100i16.to_le_bytes());
        assert!(movie_commands(&bytes).is_err());
        assert!(movie_commands(&[0; 24]).is_err());
    }
}
