#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Random {
    pub seed: u32,
    pub calls: u64,
}

impl Random {
    pub fn new(seed: u32) -> Self {
        Self { seed, calls: 0 }
    }

    pub fn next15(&mut self) -> i32 {
        self.seed = self.seed.wrapping_mul(0x41c64e6d).wrapping_add(12345);
        self.calls += 1;
        ((self.seed >> 16) & 0x7fff) as i32
    }
}
