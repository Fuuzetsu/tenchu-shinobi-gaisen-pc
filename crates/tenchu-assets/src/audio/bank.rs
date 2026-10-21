use super::adpcm::Sample;
use crate::{
    ReadAt, Result, executable::Executable, game::GameAssets, i16_le, script::Language, slice,
    u16_le, u32_le,
};

#[derive(Clone, Debug)]
pub struct Tone {
    pub mode: u8,
    pub volume: u8,
    pub pan: u8,
    pub center: u8,
    pub fine: u8,
    pub adsr: [u16; 2],
    pub sample: usize,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub volume: u8,
    pub pan: u8,
    pub tones: Vec<Tone>,
}

pub struct Bank {
    pub volume: u8,
    pub pan: u8,
    pub programs: Vec<Option<Program>>,
    pub samples: Vec<Sample>,
}

impl Bank {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = slice(bytes, 0, 32)?;
        if &header[..4] != b"pBAV" {
            return Err("Invalid VAB bank magic".into());
        }
        let programs = usize::from(u16_le(header, 18)?);
        let samples = usize::from(u16_le(header, 22)?);
        if programs > 128 || samples > 255 {
            return Err("Invalid VAB table sizes".into());
        }
        let table = slice(bytes, 32, 128 * 16)?;
        let tones = slice(bytes, 32 + 128 * 16, programs * 512)?;
        let offsets = 32 + 128 * 16 + programs * 512;
        let sizes = slice(bytes, offsets, 512)?;
        let mut cursor = offsets + 512 + usize::from(u16_le(sizes, 0)?) * 8;
        let mut decoded = vec![Sample {
            frames: Vec::new(),
            loop_start: None,
        }];
        for index in 1..=samples {
            let length = usize::from(u16_le(sizes, index * 2)?) * 8;
            decoded.push(Sample::decode(slice(bytes, cursor, length)?)?);
            cursor += length;
        }
        let mut rows = Vec::with_capacity(128);
        let mut used = 0;
        for row in table.chunks_exact(16) {
            if row[0] == 0 {
                rows.push(None);
                continue;
            }
            if row[0] > 16 || used >= programs {
                return Err("Invalid VAB program tones".into());
            }
            let mut entries = Vec::new();
            for tone in slice(tones, used * 512, usize::from(row[0]) * 32)?.chunks_exact(32) {
                let sample = i16_le(tone, 22)?;
                if sample < 0 || sample as usize > samples {
                    return Err("VAB tone sample outside bank".into());
                }
                entries.push(Tone {
                    mode: tone[1],
                    volume: tone[2],
                    pan: tone[3],
                    center: tone[4],
                    fine: tone[5],
                    adsr: [u16_le(tone, 16)?, u16_le(tone, 18)?],
                    sample: sample as usize,
                });
            }
            rows.push(Some(Program {
                volume: row[1],
                pan: row[4],
                tones: entries,
            }));
            used += 1;
        }
        if used != programs {
            return Err("VAB program count mismatch".into());
        }
        Ok(Self {
            volume: header[24],
            pan: header[25],
            programs: rows,
            samples: decoded,
        })
    }

    pub fn load<S: ReadAt>(
        game: &GameAssets<S>,
        stage: usize,
        character: i16,
        language: Language,
    ) -> Result<Self> {
        let index = match language {
            Language::English => 0,
            Language::French => 1,
            Language::Italian => 2,
            Language::Japanese => 3,
        };
        let exe = Executable::new(&game.executable)?;
        let prefix = exe.string(u32_le(exe.bytes(0x8008ea58 + index * 4, 4)?, 0)?)?;
        let name = format!(
            "{prefix}STAGE{}{}.VAB",
            stage + 1,
            if character == 0 { 'R' } else { 'A' }
        );
        Self::parse(&game.asset(&name)?)
    }
}
