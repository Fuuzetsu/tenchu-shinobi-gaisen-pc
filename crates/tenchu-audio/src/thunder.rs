#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThunderCue {
    pub seed: u32,
    pub strength: f32,
    pub pan: f32,
}

pub(super) struct Thunder {
    seed: u32,
    age: u32,
    rate: u32,
    filters: [f32; 3],
    coefficients: [f32; 3],
    volume: [f32; 2],
}

impl Thunder {
    pub fn new(cue: ThunderCue, rate: u32) -> Self {
        let pan = cue.pan.clamp(-1.0, 1.0);
        let strength = cue.strength.clamp(0.0, 1.0);
        Self {
            seed: cue.seed,
            age: 0,
            rate,
            filters: [0.0; 3],
            coefficients: [65.0, 350.0, 1800.0]
                .map(|hz| 1.0 - (-std::f32::consts::TAU * hz / rate as f32).exp()),
            volume: [
                strength * (1.0 - pan.max(0.0)),
                strength * (1.0 + pan.min(0.0)),
            ],
        }
    }

    pub fn finished(&self) -> bool {
        self.age >= self.rate * 6
    }

    pub fn frame(&mut self) -> [f32; 2] {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = (self.seed >> 8) as f32 / 8388608.0 - 1.0;
        for (filter, coefficient) in self.filters.iter_mut().zip(self.coefficients) {
            *filter += coefficient * (noise - *filter);
        }
        let time = self.age as f32 / self.rate as f32;
        self.age += 1;
        let attack = (time / 0.04).min(1.0);
        let tail = (1.0 - time / 6.0).max(0.0);
        let crack = (1.0 - time / 0.28).max(0.0);
        let rumble = self.filters[0] * 2.8 + self.filters[1] * 0.6;
        let value = (rumble * tail * tail + self.filters[2] * crack * 0.25) * attack;
        self.volume.map(|v| value * v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mixer;

    #[test]
    fn thunder_is_bounded_fades_and_obeys_pause_volume_and_clear() {
        for rate in [22050, 48000] {
            let mut mixer = Mixer::new(rate);
            mixer.thunder(Some(ThunderCue {
                seed: 42,
                strength: 1.0,
                pan: -0.25,
            }));
            let mut energy = [0.0; 7];
            for second in &mut energy {
                for _ in 0..rate {
                    let [left, right] = mixer.frame();
                    assert!(left.is_finite() && left.abs() < 0.8);
                    assert!((right - left * 0.75).abs() < 0.00001);
                    *second += left * left;
                }
            }
            assert!(energy[0] > energy[4] * 10.0);
            assert_eq!(energy[6], 0.0);
            mixer.thunder(Some(ThunderCue {
                seed: 2,
                strength: 1.0,
                pan: 0.0,
            }));
            mixer.paused = true;
            for _ in 0..rate {
                assert_eq!(mixer.frame(), [0.0; 2]);
            }
            mixer.paused = false;
            for _ in 0..100 {
                mixer.frame();
            }
            assert_ne!(mixer.frame(), [0.0; 2]);
            mixer.effects_volume = 0.0;
            assert_eq!(mixer.frame(), [0.0; 2]);
            mixer.effects_volume = 1.0;
            mixer.clear_effects();
            assert_eq!(mixer.frame(), [0.0; 2]);
        }
    }
}
