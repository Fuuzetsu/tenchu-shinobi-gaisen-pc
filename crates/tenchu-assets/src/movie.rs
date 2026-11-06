mod tables;
use crate::{ReadAt, Result, disc::Disc};

pub struct Frame {
    pub number: u32,
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

pub struct Stream {
    sector: u32,
    end: u32,
    bytes: Vec<u8>,
}

impl Stream {
    pub fn open<S: ReadAt>(disc: &Disc<S>, path: &str) -> Result<Self> {
        let entry = disc.find(path)?;
        disc.xa_sector(entry.sector)?;
        Ok(Self {
            sector: entry.sector,
            end: entry.sector + entry.size.div_ceil(2048),
            bytes: Vec::new(),
        })
    }

    pub fn next<S: ReadAt>(&mut self, disc: &Disc<S>) -> Result<Option<Frame>> {
        let mut expected = 0;
        let mut count = 0;
        let mut number = 0;
        let mut length = 0;
        let mut size = [0; 2];
        while self.sector < self.end {
            let sector = disc.xa_sector(self.sector)?;
            self.sector += 1;
            let data = &sector[8..2056];
            if data[..4] != [0x60, 0x01, 0x01, 0x80] {
                continue;
            }
            let chunk = word(data, 4)?;
            if chunk == 0 {
                if expected != 0 {
                    return Err("Incomplete STR frame".into());
                }
                count = word(data, 6)?;
                number = u32::from_le_bytes(data[8..12].try_into().unwrap());
                length = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
                size = [usize::from(word(data, 16)?), usize::from(word(data, 18)?)];
                if count == 0 || count > 512 || length > usize::from(count) * 2016 || length < 8 {
                    return Err("Invalid STR frame length".into());
                }
                self.bytes.clear();
            }
            if chunk != expected
                || word(data, 6)? != count
                || u32::from_le_bytes(data[8..12].try_into().unwrap()) != number
            {
                return Err("STR chunks out of order".into());
            }
            self.bytes.extend_from_slice(&data[32..]);
            expected += 1;
            if expected == count {
                return Ok(Some(Frame {
                    number,
                    size,
                    rgba: decode(&self.bytes[..length], size)?,
                }));
            }
        }
        if expected != 0 {
            return Err("Truncated STR movie".into());
        }
        Ok(None)
    }
}

fn word(data: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        data.get(offset..offset + 2)
            .ok_or("Truncated video header")?
            .try_into()
            .unwrap(),
    ))
}

struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl Bits<'_> {
    fn read(&mut self, count: u32) -> Result<u16> {
        let mut value = 0;
        for _ in 0..count {
            let at = self.position / 16 * 2;
            let w = word(self.bytes, at)?;
            value = (value << 1) | ((w >> (15 - self.position % 16)) & 1);
            self.position += 1;
        }
        Ok(value)
    }
    fn dc(&mut self, chroma: bool) -> Result<i32> {
        let entries: &[(u16, u32, u32)] = if chroma {
            &[
                (0b00, 2, 0),
                (0b01, 2, 1),
                (0b10, 2, 2),
                (0b110, 3, 3),
                (0b1110, 4, 4),
                (0b11110, 5, 5),
                (0b111110, 6, 6),
                (0b1111110, 7, 7),
                (0b11111110, 8, 8),
            ]
        } else {
            &[
                (0b100, 3, 0),
                (0b00, 2, 1),
                (0b01, 2, 2),
                (0b101, 3, 3),
                (0b110, 3, 4),
                (0b1110, 4, 5),
                (0b11110, 5, 6),
                (0b111110, 6, 7),
                (0b1111110, 7, 8),
            ]
        };
        let mut code = 0;
        for n in 1..=8 {
            code = code * 2 + self.read(1)?;
            if let Some(&(_, _, count)) = entries
                .iter()
                .find(|&&(prefix, len, _)| prefix == code && len == n)
            {
                if count == 0 {
                    return Ok(0);
                }
                let bits = i32::from(self.read(count)?);
                return Ok(if bits < 1 << (count - 1) {
                    bits - ((1 << count) - 1)
                } else {
                    bits
                } * 4);
            }
        }
        Err("Invalid or premature video DC end code".into())
    }
    fn ac(&mut self) -> Result<Option<u16>> {
        let mut code = 0;
        for n in 1..=12 {
            code = code * 2 + self.read(1)?;
            if n == 2 && code == 2 {
                return Ok(None);
            }
            if n == 6 && code == 1 {
                return self.read(16).map(Some);
            }
            if let Some(&(_, _, count, values)) = tables::AC
                .iter()
                .find(|&&(prefix, len, _, _)| prefix == code && len == n)
            {
                let word = values[usize::from(self.read(count)?)];
                return Ok(Some(if self.read(1)? == 0 {
                    word
                } else {
                    word & 0xfc00 | ((-(word as i16)) as u16 & 0x3ff)
                }));
            }
        }
        Err("Invalid video AC code".into())
    }
}

const SCAN: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];
const QUANT: [i32; 64] = [
    2, 16, 16, 19, 16, 19, 22, 22, 22, 22, 22, 22, 26, 24, 26, 27, 27, 27, 26, 26, 26, 26, 27, 27,
    27, 29, 29, 29, 34, 34, 34, 29, 29, 29, 27, 27, 29, 29, 32, 32, 34, 34, 37, 38, 37, 35, 35, 34,
    35, 38, 38, 40, 40, 40, 48, 48, 46, 46, 56, 56, 58, 69, 69, 83,
];

pub fn decode(bytes: &[u8], size: [usize; 2]) -> Result<Vec<u8>> {
    let [width, height] = size;
    if width == 0 || height == 0 || width > 640 || height > 480 || word(bytes, 2)? != 0x3800 {
        return Err("Invalid movie dimensions or bitstream".into());
    }
    let quant = i32::from(word(bytes, 4)?);
    let version = word(bytes, 6)?;
    if !matches!(version, 1..=3) || quant > 63 {
        return Err("Unsupported video bitstream".into());
    }
    let mut bits = Bits {
        bytes: &bytes[8..],
        position: 0,
    };
    let basis: [[f32; 8]; 8] = std::array::from_fn(|x| {
        std::array::from_fn(|u| {
            ((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI / 16.0).cos()
                * if u == 0 {
                    std::f32::consts::FRAC_1_SQRT_2
                } else {
                    1.0
                }
        })
    });
    let mut previous = [0; 3];
    let mut rgba = vec![0; width * height * 4];
    for px in (0..width).step_by(16) {
        for py in (0..height).step_by(16) {
            let mut blocks = [[0.0; 64]; 6];
            for (component, block) in blocks.iter_mut().enumerate() {
                let predictor = component.min(2);
                let dc = if version == 3 {
                    previous[predictor] += bits.dc(component < 2)?;
                    previous[predictor]
                } else {
                    signed(bits.read(10)?)
                };
                let mut coefficients = [0.0; 64];
                coefficients[0] = (dc * QUANT[0]).clamp(-1024, 1023) as f32;
                let mut at = 0;
                while let Some(word) = bits.ac()? {
                    if word == 0xfe00 {
                        break;
                    }
                    at += usize::from(word >> 10) + 1;
                    if at >= 64 {
                        return Err("Video coefficient outside block".into());
                    }
                    let level = signed(word);
                    let value = if quant == 0 {
                        level * 2
                    } else {
                        (level * QUANT[at] * quant + 4) / 8
                    };
                    coefficients[if quant == 0 { at } else { SCAN[at] }] =
                        value.clamp(-1024, 1023) as f32;
                }
                let mut horizontal = [[0.0; 8]; 8];
                for (v, row) in horizontal.iter_mut().enumerate() {
                    for (x, value) in row.iter_mut().enumerate() {
                        *value = (0..8)
                            .map(|u| coefficients[v * 8 + u] * basis[x][u])
                            .sum::<f32>()
                            / 2.0;
                    }
                }
                for y in 0..8 {
                    for x in 0..8 {
                        block[y * 8 + x] =
                            ((0..8).map(|v| horizontal[v][x] * basis[y][v]).sum::<f32>() / 2.0)
                                .round();
                    }
                }
            }
            for y in 0..16.min(height - py) {
                for x in 0..16.min(width - px) {
                    let cr = blocks[0][y / 2 * 8 + x / 2];
                    let cb = blocks[1][y / 2 * 8 + x / 2];
                    let luma = blocks[2 + y / 8 * 2 + x / 8][y % 8 * 8 + x % 8] + 128.0;
                    let offset = ((py + y) * width + px + x) * 4;
                    let rgb = [
                        luma + 1.402 * cr,
                        luma - 0.3437 * cb - 0.7143 * cr,
                        luma + 1.772 * cb,
                    ];
                    for (out, value) in rgba[offset..offset + 3].iter_mut().zip(rgb) {
                        *out = value.round().clamp(0.0, 255.0) as u8;
                    }
                    rgba[offset + 3] = 255;
                }
            }
        }
    }
    Ok(rgba)
}
fn signed(word: u16) -> i32 {
    i32::from((word << 6) as i16 >> 6)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bits: &str, version: u16) -> Vec<u8> {
        let mut bytes = [0u16, 0x3800, 1, version]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        for chunk in bits.as_bytes().chunks(16) {
            let mut word = 0u16;
            for (i, &bit) in chunk.iter().enumerate() {
                word |= u16::from(bit == b'1') << (15 - i);
            }
            bytes.extend(word.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn differential_dc_blocks_and_cropped_macroblocks_decode_without_assets() {
        // Neutral chroma, then luma DC +4, unchanged, -4, unchanged.
        let bytes = frame(
            concat!("0010", "0010", "00110", "10010", "00010", "10010"),
            3,
        );
        let rgba = decode(&bytes, [13, 11]).unwrap();
        for y in 0..11 {
            for x in 0..13 {
                let value = if y < 8 { 129 } else { 128 };
                assert_eq!(&rgba[(y * 13 + x) * 4..][..4], &[value, value, value, 255]);
            }
        }
    }

    #[test]
    fn malformed_frames_are_errors_instead_of_partial_images_or_panics() {
        for bytes in [vec![], vec![0; 8], frame("0010", 3)] {
            assert!(decode(&bytes, [16, 16]).is_err());
        }
        assert!(decode(&frame("", 3), [usize::MAX, 16]).is_err());
        assert!(decode(&frame("", 4), [16, 16]).is_err());
        // Escape a run beyond the last coefficient.
        let bytes = frame(
            concat!("0010", "0010", "100", "000001", "1111110000000001"),
            3,
        );
        assert!(decode(&bytes, [16, 16]).is_err());
    }
}
