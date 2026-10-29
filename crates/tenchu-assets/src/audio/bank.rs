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
    pub fn import_program(&mut self, source: &Self, index: usize) -> Result<usize> {
        let Some(mut program) = source
            .programs
            .get(index)
            .ok_or("Invalid imported voice program")?
            .clone()
        else {
            let index = self.programs.len();
            self.programs.push(None);
            return Ok(index);
        };
        let mut samples = std::collections::BTreeMap::new();
        for tone in &mut program.tones {
            tone.sample = *samples.entry(tone.sample).or_insert_with(|| {
                let index = self.samples.len();
                self.samples.push(source.samples[tone.sample].clone());
                index
            });
        }
        program.volume = (u16::from(program.volume) * u16::from(source.volume)
            / u16::from(self.volume.max(1)))
        .min(127) as u8;
        let index = self.programs.len();
        self.programs.push(Some(program));
        Ok(index)
    }

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
            let mut entries = Vec::with_capacity(16);
            // The count excludes empty slots; sound IDs still address all 16 slots.
            for tone in slice(tones, used * 512, 512)?.chunks_exact(32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_voices_keep_sparse_slots_and_remap_shared_samples() {
        let tone = Tone {
            mode: 0,
            volume: 127,
            pan: 64,
            center: 60,
            fine: 0,
            adsr: [0; 2],
            sample: 1,
        };
        let source = Bank {
            volume: 127,
            pan: 64,
            programs: vec![
                None,
                Some(Program {
                    volume: 80,
                    pan: 64,
                    tones: vec![tone.clone(), tone],
                }),
            ],
            samples: vec![
                Sample {
                    frames: vec![],
                    loop_start: None,
                },
                Sample {
                    frames: vec![10, -20, 30],
                    loop_start: Some(1),
                },
            ],
        };
        let mut destination = Bank {
            volume: 127,
            pan: 64,
            programs: vec![None; 128],
            samples: vec![Sample {
                frames: vec![777],
                loop_start: None,
            }],
        };
        assert_eq!(destination.import_program(&source, 0).unwrap(), 128);
        assert!(destination.programs[128].is_none());
        assert_eq!(destination.import_program(&source, 1).unwrap(), 129);
        assert_eq!(destination.samples.len(), 2);
        assert_eq!(destination.samples[0].frames, [777]);
        let program = destination.programs[129].as_ref().unwrap();
        assert!(program.tones.iter().all(|tone| tone.sample == 1));
        assert_eq!(destination.samples[1].frames, [10, -20, 30]);
        assert_eq!(destination.samples[1].loop_start, Some(1));
        assert!(destination.import_program(&source, 2).is_err());
    }

    #[test]
    fn sparse_tones_keep_their_sound_id_slots() {
        let mut bytes = vec![0; 32 + 128 * 16 + 512 + 512 + 16];
        bytes[..4].copy_from_slice(b"pBAV");
        bytes[18..20].copy_from_slice(&1u16.to_le_bytes());
        bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
        bytes[32] = 2;
        for slot in [0, 12] {
            let tone = 32 + 128 * 16 + slot * 32;
            bytes[tone + 2] = 127;
            bytes[tone + 22..tone + 24].copy_from_slice(&1u16.to_le_bytes());
        }
        let sizes = 32 + 128 * 16 + 512;
        bytes[sizes + 2..sizes + 4].copy_from_slice(&2u16.to_le_bytes());
        bytes[sizes + 512 + 1] = 1;
        bytes[sizes + 512 + 2..].fill(0x11);
        let bank = Bank::parse(&bytes).unwrap();
        let tones = &bank.programs[0].as_ref().unwrap().tones;
        assert_eq!(tones.len(), 16);
        assert_eq!(tones[0].sample, 1);
        assert_eq!(tones[12].sample, 1);
        assert!(tones[1..12].iter().all(|tone| tone.sample == 0));
        assert!(tones[13..].iter().all(|tone| tone.sample == 0));
        assert!(
            bank.samples[tones[12].sample]
                .frames
                .iter()
                .any(|&x| x != 0)
        );
    }
}
