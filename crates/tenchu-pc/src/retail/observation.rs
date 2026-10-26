use flate2::read::GzDecoder;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use tenchu_assets::Result;
use tenchu_core::math::Position;

const RAM_SIZE: usize = 0x200000;

pub struct Ram(Vec<u8>);

impl Ram {
    pub fn bytes(&self, address: u32, length: usize) -> &[u8] {
        let offset = (address & 0x1fffff) as usize;
        &self.0[offset..offset + length]
    }
    pub fn byte(&self, address: u32) -> u8 {
        self.bytes(address, 1)[0]
    }
    pub fn short(&self, address: u32) -> i16 {
        i16::from_le_bytes(self.bytes(address, 2).try_into().unwrap())
    }
    pub fn word(&self, address: u32) -> u32 {
        u32::from_le_bytes(self.bytes(address, 4).try_into().unwrap())
    }
    pub fn vector(&self, address: u32) -> Position {
        Position::from_fn(|i| self.word(address + i as u32 * 4) as i32)
    }
    pub fn short_vector(&self, address: u32) -> [i16; 3] {
        std::array::from_fn(|i| self.short(address + i as u32 * 2))
    }
    pub fn matrix(&self, address: u32) -> tenchu_core::math::Matrix {
        tenchu_core::math::Matrix {
            rotation: std::array::from_fn(|i| {
                std::array::from_fn(|j| self.short(address + (i * 3 + j) as u32 * 2))
            }),
            translation: self.vector(address + 20),
        }
    }
}

#[derive(Debug)]
pub struct Observation {
    pub pc: u32,
    pub field: u32,
    pub skip: u32,
    pub clock: u32,
    pub stage: u32,
    pub pad: u32,
    pub seed: u32,
    pub epoch: u32,
    pub caller: u32,
    pub snapshot: bool,
}

pub struct Reader {
    source: BufReader<GzDecoder<BufReader<File>>>,
    pub ram: Ram,
    initialized: bool,
    cycle: u64,
}

impl Reader {
    pub fn open(path: &Path) -> Result<Self> {
        let mut source = BufReader::new(GzDecoder::new(BufReader::new(
            File::open(path).map_err(|e| e.to_string())?,
        )));
        let mut magic = [0; 8];
        source.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic != b"TNRUN002" {
            return Err("Expected exact-update TNRUN002 recording".into());
        }
        Ok(Self {
            source,
            ram: Ram(vec![0; RAM_SIZE]),
            initialized: false,
            cycle: 0,
        })
    }

    pub fn next(&mut self) -> Result<Option<Observation>> {
        let mut header = [0; 56];
        if self
            .source
            .read(&mut header[..1])
            .map_err(|e| e.to_string())?
            == 0
        {
            return Ok(None);
        }
        self.source
            .read_exact(&mut header[1..])
            .map_err(|e| e.to_string())?;
        let word = |offset| u32::from_le_bytes(header[offset..offset + 4].try_into().unwrap());
        let size = word(0) as usize;
        if !(56..=RAM_SIZE + 65592).contains(&size) {
            return Err("Invalid observation size".into());
        }
        let cycle = u64::from_le_bytes(header[16..24].try_into().unwrap());
        if cycle < self.cycle {
            return Err("Observation clock moved backwards".into());
        }
        self.cycle = cycle;
        let row = Observation {
            pc: word(4),
            field: word(8),
            skip: word(12),
            clock: word(24),
            stage: word(28),
            pad: word(32),
            seed: word(36),
            epoch: word(40),
            caller: word(44),
            snapshot: size > 56,
        };
        if row.snapshot {
            let mut payload = vec![0; size - 56];
            self.source
                .read_exact(&mut payload)
                .map_err(|e| e.to_string())?;
            let mut chunks = payload.as_slice();
            fn take_word(chunks: &mut &[u8]) -> Result<usize> {
                if chunks.len() < 4 {
                    return Err("Truncated RAM patch".into());
                }
                let word = u32::from_le_bytes(chunks[..4].try_into().unwrap()) as usize;
                *chunks = &chunks[4..];
                Ok(word)
            }
            let count = take_word(&mut chunks)?;
            let mut end = 0;
            for _ in 0..count {
                let offset = take_word(&mut chunks)?;
                let length = take_word(&mut chunks)?;
                if offset < end
                    || length == 0
                    || offset.saturating_add(length) > RAM_SIZE
                    || length > chunks.len()
                {
                    return Err("Invalid RAM patch".into());
                }
                if !self.initialized && (count != 1 || offset != 0 || length != RAM_SIZE) {
                    return Err("Missing initial RAM snapshot".into());
                }
                end = offset + length;
                self.ram.0[offset..end].copy_from_slice(&chunks[..length]);
                chunks = &chunks[length..];
            }
            if !chunks.is_empty() || !self.initialized && count == 0 {
                return Err("Invalid RAM patch length".into());
            }
            self.initialized = true;
            if self.ram.word(0x800976b0) != row.clock || self.ram.word(0x800c79e0) != row.seed {
                return Err("Observation header disagrees with RAM snapshot".into());
            }
        }
        Ok(Some(row))
    }
}
