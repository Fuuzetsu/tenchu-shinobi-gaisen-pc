mod rolling;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThunderStyle {
    #[default]
    Simple,
    Rolling,
    Retro,
}

impl ThunderStyle {
    pub const ALL: [Self; 3] = [Self::Simple, Self::Rolling, Self::Retro];
}

pub(super) enum Thunder {
    Simple(Simple),
    Rolling(Box<rolling::Rolling>),
}

impl Thunder {
    pub fn new(cue: ThunderCue, rate: u32) -> Self {
        match cue.style {
            ThunderStyle::Simple => Self::Simple(Simple::new(cue, rate)),
            ThunderStyle::Rolling | ThunderStyle::Retro => {
                Self::Rolling(Box::new(rolling::Rolling::new(cue, rate)))
            }
        }
    }

    pub fn finished(&self) -> bool {
        match self {
            Self::Simple(s) => s.finished(),
            Self::Rolling(s) => s.finished(),
        }
    }

    pub fn frame(&mut self) -> [f32; 2] {
        match self {
            Self::Simple(s) => s.frame(),
            Self::Rolling(s) => s.frame(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThunderCue {
    pub seed: u32,
    pub strength: f32,
    pub pan: f32,
    pub style: ThunderStyle,
}

pub(super) struct Simple {
    seed: u32,
    age: u32,
    rate: u32,
    filters: [f32; 3],
    coefficients: [f32; 3],
    volume: [f32; 2],
}

impl Simple {
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
    fn rolling_thunder_keeps_stereo_tails_and_retro_sample_hold() {
        for rate in [22050, 48000] {
            for style in [ThunderStyle::Rolling, ThunderStyle::Retro] {
                let cue = ThunderCue {
                    seed: 42,
                    strength: 1.0,
                    pan: 0.0,
                    style,
                };
                let mut thunder = Thunder::new(cue, rate);
                let mut energy = [0.0; 10];
                let mut difference = 0.0;
                let mut repeated = 0;
                let mut previous = [0.0; 2];
                for (second, energy) in energy.iter_mut().enumerate() {
                    for _ in 0..rate {
                        let frame = thunder.frame();
                        assert!(frame.iter().all(|v| v.is_finite() && v.abs() < 1.0));
                        *energy += frame.iter().map(|v| v * v).sum::<f32>();
                        difference += (frame[0] - frame[1]).abs();
                        if second < 3 && previous == frame {
                            repeated += 1;
                        }
                        previous = frame;
                    }
                }
                assert!(difference > 1.0, "No diffuse stereo tail");
                assert!(
                    energy[5] > 0.001 && energy[0] > energy[9] * 100.0,
                    "{style:?}: {energy:?}"
                );
                assert!(thunder.finished());
                if style == ThunderStyle::Retro {
                    assert!(repeated > rate);
                }
                let mut mixer = Mixer::new(rate);
                mixer.thunder(Some(cue));
                for _ in 0..rate {
                    mixer.frame();
                }
                mixer.thunder(Some(ThunderCue { seed: 7, ..cue }));
                assert!(mixer.thunder.iter().all(Option::is_some));
                for _ in 0..rate * 11 {
                    mixer.frame();
                }
                assert!(mixer.thunder.iter().all(Option::is_none));
                assert_eq!(mixer.frame(), [0.0; 2]);
            }
        }
    }

    #[test]
    fn thunder_is_bounded_fades_and_obeys_pause_volume_and_clear() {
        for rate in [22050, 48000] {
            let mut mixer = Mixer::new(rate);
            mixer.thunder(Some(ThunderCue {
                seed: 42,
                strength: 1.0,
                pan: -0.25,
                style: ThunderStyle::Simple,
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
                style: ThunderStyle::Simple,
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
