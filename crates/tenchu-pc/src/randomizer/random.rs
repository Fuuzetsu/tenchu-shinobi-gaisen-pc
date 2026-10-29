use sha2::{Digest, Sha256};

pub struct Random(u64);

impl Random {
    pub fn new(seed: &str, round: u32, domain: &str, entry: u32) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"Tenchu randomizer v1\0");
        hash.update((seed.len() as u32).to_le_bytes());
        hash.update(seed.as_bytes());
        hash.update(round.to_le_bytes());
        hash.update(domain.as_bytes());
        hash.update(entry.to_le_bytes());
        Self(u64::from_le_bytes(hash.finalize()[..8].try_into().unwrap()))
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    pub fn index(&mut self, count: usize) -> usize {
        let count = count as u64;
        let threshold = count.wrapping_neg() % count;
        loop {
            let value = self.next();
            if value >= threshold {
                return (value % count) as usize;
            }
        }
    }

    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, self.index(i + 1));
        }
    }
}
