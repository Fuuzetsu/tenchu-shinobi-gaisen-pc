use crate::Result;

const FILTERS: [[i32; 2]; 5] = [[0, 0], [60, 0], [115, -52], [98, -55], [122, -60]];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Predictor {
    history: [i32; 2],
}

fn shift(header: u8) -> u32 {
    let shift = u32::from(header & 15);
    if shift > 12 { 9 } else { shift }
}

impl Predictor {
    fn sample(&mut self, residual: i32, filter: [i32; 2], xa: bool) -> i16 {
        let [a, b] = std::array::from_fn(|i| self.history[i] * filter[i]);
        let feedback = if xa {
            (a + b + 32) >> 6
        } else {
            (a >> 6) + (b >> 6)
        };
        let value = (residual + feedback).clamp(-32768, 32767);
        self.history = [value, self.history[0]];
        value as i16
    }

    pub fn spu_block(&mut self, bytes: &[u8]) -> Result<[i16; 28]> {
        let block = crate::slice(bytes, 0, 16)?;
        let filter = *FILTERS
            .get(usize::from(block[0] >> 4))
            .ok_or("Invalid sound ADPCM filter")?;
        let shift = shift(block[0]);
        Ok(std::array::from_fn(|index| {
            let nibble = (block[2 + index / 2] >> ((index % 2) * 4)) & 15;
            let residual = i32::from((u16::from(nibble) << 12) as i16) >> shift;
            self.sample(residual, filter, false)
        }))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Xa {
    channels: [Predictor; 2],
}

impl Xa {
    pub fn group(&mut self, bytes: &[u8], stereo: bool, eight_bit: bool) -> Result<Vec<[i16; 2]>> {
        let group = crate::slice(bytes, 0, 128)?;
        let channels = if stereo { 2 } else { 1 };
        let units = if eight_bit { 4 } else { 8 };
        let mut output = vec![[0; 2]; units * 28 / channels];
        for unit in 0..units {
            let header = group[4 + unit];
            let filter = FILTERS[usize::from((header >> 4) & 3)];
            let shift = shift(header);
            let channel = unit % channels;
            for sample in 0..28 {
                let value = if eight_bit {
                    i32::from(group[16 + unit + sample * 4] as i8) << 8
                } else {
                    let nibble = (group[16 + unit / 2 + sample * 4] >> (unit % 2 * 4)) & 15;
                    i32::from((u16::from(nibble) << 12) as i16)
                };
                let value = self.channels[channel].sample(value >> shift, filter, true);
                let target = &mut output[unit / channels * 28 + sample];
                target[channel] = value;
                if !stereo {
                    target[1] = value;
                }
            }
        }
        Ok(output)
    }
}

#[derive(Clone, Debug)]
pub struct Sample {
    pub frames: Vec<i16>,
    pub loop_start: Option<usize>,
}

impl Sample {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if !bytes.len().is_multiple_of(16) {
            return Err("Partial sound ADPCM block".into());
        }
        let mut predictor = Predictor::default();
        let mut frames = Vec::with_capacity(bytes.len() / 16 * 28);
        let mut loop_start = 0;
        for block in bytes.chunks_exact(16) {
            if block[1] & 4 != 0 {
                loop_start = frames.len();
            }
            frames.extend(predictor.spu_block(block)?);
            if block[1] & 1 != 0 {
                return Ok(Self {
                    frames,
                    loop_start: (block[1] & 2 != 0).then_some(loop_start),
                });
            }
        }
        Ok(Self {
            frames,
            loop_start: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_signed_nibbles_saturate_and_keep_loop_boundaries() {
        let mut block = [0; 16];
        block[2] = 0x87;
        let frames = Predictor::default().spu_block(&block).unwrap();
        assert_eq!(&frames[..4], &[28672, -32768, 0, 0]);
        block[0] = 0x10;
        block[2..].fill(0x77);
        let frames = Predictor::default().spu_block(&block).unwrap();
        assert_eq!(frames[0], 28672);
        assert!(frames[1..].iter().all(|&s| s == 32767));
        let mut data = vec![0; 48];
        data[17] = 4;
        data[33] = 3;
        let sample = Sample::decode(&data).unwrap();
        assert_eq!(sample.frames.len(), 84);
        assert_eq!(sample.loop_start, Some(28));
        data[33] = 1;
        assert_eq!(Sample::decode(&data).unwrap().loop_start, None);
        assert!(Sample::decode(&data[..47]).is_err());
    }

    #[test]
    fn xa_interleaves_units_with_separate_stereo_predictors() {
        let mut group = [0; 128];
        group[4..12].fill(12);
        for row in group[16..].chunks_exact_mut(4) {
            row.copy_from_slice(&[0x21, 0x43, 0x65, 0x87]);
        }
        let stereo = Xa::default().group(&group, true, false).unwrap();
        assert_eq!(stereo.len(), 112);
        for (block, values) in [[1, 2], [3, 4], [5, 6], [7, -8]].into_iter().enumerate() {
            assert!(stereo[block * 28..][..28].iter().all(|&s| s == values));
        }
        let mono = Xa::default().group(&group, false, false).unwrap();
        assert_eq!(mono.len(), 224);
        for (block, value) in [1, 2, 3, 4, 5, 6, 7, -8].into_iter().enumerate() {
            assert!(mono[block * 28..][..28].iter().all(|&s| s == [value; 2]));
        }
        group[4..8].fill(8);
        let frames = Xa::default().group(&group, true, true).unwrap();
        assert_eq!(frames.len(), 56);
        assert!(frames[..28].iter().all(|&s| s == [0x21, 0x43]));
        assert!(frames[28..].iter().all(|&s| s == [0x65, -121]));
    }
}
