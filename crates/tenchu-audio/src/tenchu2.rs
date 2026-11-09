use super::voice::Voice;
use tenchu_assets::tenchu2::music::Ambience;

pub(super) struct Soundscape {
    source: Ambience,
    voices: [Option<Voice>; 4],
    rate: u32,
    clock: u32,
    tick: u32,
    last_base: u32,
    base_voice: usize,
    variant_voice: usize,
    random: u32,
}

impl Soundscape {
    pub fn new(source: Ambience, rate: u32) -> Self {
        let mut result = Self {
            source,
            voices: std::array::from_fn(|_| None),
            rate,
            clock: 0,
            tick: 0,
            last_base: 0,
            base_voice: 0,
            variant_voice: 0,
            random: 1,
        };
        result.update();
        result
    }

    fn random(&mut self) -> u32 {
        // Presentation has its own stream; ambient playback must not steer Gaisen AI.
        self.random = self.random.wrapping_mul(0x41c64e6d).wrapping_add(12345);
        (self.random >> 16) & 0x7fff
    }

    fn update(&mut self) {
        let interval = self.source.interval;
        if interval == 0 {
            return;
        }
        let base = if interval > 0 {
            let elapsed = self.tick.wrapping_sub(self.last_base);
            elapsed == 0 || elapsed >= interval as u32
        } else {
            self.tick.is_multiple_of(self.source.ticks_per_second)
                && self.random().is_multiple_of(10)
        };
        let (slot, id, volume, note) = if base {
            self.last_base = self.tick;
            self.base_voice ^= 1;
            (self.base_voice, 10, 127, 36)
        } else if self
            .random()
            .is_multiple_of(self.source.ticks_per_second * 10)
        {
            self.variant_voice ^= 1;
            let (id, note) = if self.source.ship_variants {
                let id = if self.random().is_multiple_of(3) {
                    15
                } else {
                    11 + (self.random() & 1) as i16
                };
                (id, 36)
            } else {
                let note = 36 - (self.random() % 4) as u8;
                (11 + (self.random() & 1) as i16, note)
            };
            (self.variant_voice + 2, id, 63, note)
        } else {
            return;
        };
        if let Some(voice) = Voice::new(&self.source.bank, id, volume, 64, note, self.rate, false) {
            self.voices[slot] = Some(voice);
        }
    }

    pub fn frame(&mut self, envelope_ticks: u64) -> [f32; 2] {
        let mut output = [0.0; 2];
        for slot in &mut self.voices {
            if let Some(voice) = slot {
                if let Some(frame) = voice.frame(envelope_ticks) {
                    for (out, sample) in output.iter_mut().zip(frame) {
                        *out += sample;
                    }
                } else {
                    *slot = None;
                }
            }
        }
        self.clock += self.source.ticks_per_second;
        if self.clock >= self.rate {
            self.clock -= self.rate;
            self.tick = self.tick.wrapping_add(1);
            self.update();
        }
        output
    }
}
