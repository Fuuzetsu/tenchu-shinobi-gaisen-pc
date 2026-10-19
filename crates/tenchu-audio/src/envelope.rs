#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Attack,
    Decay,
    Sustain,
}

#[derive(Clone, Debug)]
pub struct Envelope {
    adsr: [u16; 2],
    level: i32,
    counter: u32,
    phase: Phase,
}

impl Envelope {
    pub fn new(adsr: [u16; 2]) -> Self {
        Self {
            adsr,
            level: 0,
            counter: 0,
            phase: Phase::Attack,
        }
    }

    pub fn volume(&self) -> f32 {
        self.level as f32 / 32768.0
    }

    pub fn finished(&self) -> bool {
        self.phase == Phase::Sustain && self.level == 0 && self.adsr[1] & 0x4000 != 0
    }

    pub fn tick(&mut self) {
        let [a, s] = self.adsr.map(u32::from);
        let (shift, step, decreasing, exponential, all_bits) = match self.phase {
            Phase::Attack => ((a >> 10) & 31, (a >> 8) & 3, false, a & 0x8000 != 0, 127),
            Phase::Decay => ((a >> 4) & 15, 0, true, true, 60),
            Phase::Sustain => (
                (s >> 8) & 31,
                (s >> 6) & 3,
                s & 0x4000 != 0,
                s & 0x8000 != 0,
                127,
            ),
        };
        let mut amount = if decreasing {
            -8 + step as i32
        } else {
            7 - step as i32
        };
        amount <<= 11u32.saturating_sub(shift);
        let mut increment = 0x8000 >> shift.saturating_sub(11);
        if exponential {
            if decreasing {
                amount = (amount * self.level) >> 15;
            } else if self.level > 0x6000 {
                match shift {
                    0..=9 => amount >>= 2,
                    10 => {
                        amount >>= 1;
                        increment >>= 1;
                    }
                    _ => increment >>= 2,
                }
            }
        }
        if (step | shift << 2) != all_bits {
            increment = increment.max(1);
        }
        self.counter += increment;
        if self.counter < 0x8000 {
            return;
        }
        self.counter = 0;
        self.level = (self.level + amount).clamp(0, 32767);
        if self.phase == Phase::Attack && self.level == 32767 {
            self.phase = Phase::Decay;
        } else if self.phase == Phase::Decay && self.level <= (((a & 15) + 1) * 0x800) as i32 {
            self.phase = Phase::Sustain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelopes_reach_sustain_decay_to_silence_and_can_hold_indefinitely() {
        let mut decay = Envelope::new([0x000f, 0x4000]);
        let levels: Vec<_> = (0..6)
            .map(|_| {
                decay.tick();
                decay.level
            })
            .collect();
        assert_eq!(levels, [14336, 28672, 32767, 16383, 0, 0]);
        assert!(decay.finished());
        let mut held = Envelope::new([0x000f, 0x1fc0]);
        for _ in 0..100_000 {
            held.tick();
        }
        assert_eq!(held.level, 16383);
        assert!(!held.finished());
        let mut stopped_attack = Envelope::new([0x7f00, 0]);
        for _ in 0..100_000 {
            stopped_attack.tick();
        }
        assert_eq!(stopped_attack.level, 0);
        let mut exponential = Envelope::new([0x800f, 0x1fc0]);
        let levels: Vec<_> = (0..5)
            .map(|_| {
                exponential.tick();
                exponential.level
            })
            .collect();
        assert_eq!(levels, [14336, 28672, 32256, 32767, 16383]);
    }
}
