#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ambience {
    pub rain: f32,
    pub wind: f32,
    pub exposure: f32,
}

pub(super) struct Bed {
    target: Ambience,
    current: Ambience,
    seed: u32,
    filters: [[f32; 3]; 2],
    coefficients: [f32; 3],
    gains: [f32; 3],
    smoothing: f32,
    wash: f32,
    wash_step: f32,
    resonance: [[f32; 2]; 2],
    resonance_step: f32,
    age: u32,
    rate: f32,
}

impl Bed {
    pub fn new(rate: u32) -> Self {
        let coefficients = [130.0, 1100.0, 6500.0]
            .map(|hz| 1.0 - (-std::f32::consts::TAU * hz / rate as f32).exp());
        Self {
            target: Ambience::default(),
            current: Ambience::default(),
            seed: 0x728ac91d,
            filters: [[0.0; 3]; 2],
            coefficients,
            gains: coefficients.map(|a| ((2.0 - a) / a).sqrt()),
            smoothing: 1.0 - (-1.0 / (rate as f32 * 0.35)).exp(),
            wash: 0.0,
            wash_step: 1.0 - (-1.0 / (rate as f32 * 0.8)).exp(),
            resonance: [[0.0; 2]; 2],
            resonance_step: 0.0,
            age: 0,
            rate: rate as f32,
        }
    }

    pub fn set(&mut self, ambience: Ambience) {
        self.target = Ambience {
            rain: ambience.rain.clamp(0.0, 1.0),
            wind: ambience.wind.clamp(0.0, 1.0),
            exposure: ambience.exposure.clamp(0.0, 1.0),
        };
    }

    pub fn finished(&self) -> bool {
        self.target.rain == 0.0
            && self.target.wind == 0.0
            && self.current.rain.max(self.current.wind) < 0.00001
    }

    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / 8388608.0 - 1.0
    }

    pub fn frame(&mut self) -> [f32; 2] {
        self.current.rain += (self.target.rain - self.current.rain) * self.smoothing;
        self.current.wind += (self.target.wind - self.current.wind) * self.smoothing;
        self.current.exposure += (self.target.exposure - self.current.exposure) * self.smoothing;
        self.wash += (self.noise() - self.wash) * self.wash_step;
        let wash = (1.0 + self.wash * 45.0).clamp(0.6, 1.4);
        let exposure = self.current.exposure;
        let gain = self.current.rain * (0.28 + 0.72 * exposure) * wash;
        if self.age.is_multiple_of(128) {
            let frequency = 240.0 + (self.wash * 60.0 + 0.5).clamp(0.0, 1.0) * 460.0;
            self.resonance_step = 2.0 * (std::f32::consts::PI * frequency / self.rate).sin();
        }
        self.age = self.age.wrapping_add(1);
        let wind_gain = self.current.wind * (0.25 + 0.75 * exposure) * wash * wash;
        std::array::from_fn(|channel| {
            let noise = self.noise();
            let filters = &mut self.filters[channel];
            for (filter, coefficient) in filters.iter_mut().zip(self.coefficients) {
                *filter += (noise - *filter) * coefficient;
            }
            let [low, middle, high] = std::array::from_fn(|i| filters[i] * self.gains[i]);
            let resonance = &mut self.resonance[channel];
            resonance[0] += self.resonance_step * resonance[1];
            let high_pass = noise - resonance[0] - 0.18 * resonance[1];
            resonance[1] += self.resonance_step * high_pass;
            let wind = low * 0.05 + middle * 0.015 + resonance[1] * 0.18;
            (low * 0.035 + middle * (0.015 + 0.035 * exposure) + high * 0.025 * exposure) * gain
                + wind * wind_gain
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mixer;

    #[test]
    fn blizzard_wind_is_bounded_varies_and_fades_to_silence() {
        for rate in [22050, 44100, 48000] {
            let mut mixer = Mixer::new(rate);
            mixer.ambience(Ambience {
                wind: 1.0,
                exposure: 1.0,
                ..Default::default()
            });
            let mut energies = [0.0_f32; 8];
            for energy in &mut energies {
                for _ in 0..rate {
                    let frame = mixer.frame();
                    assert!(frame.iter().all(|v| v.is_finite() && v.abs() < 0.8));
                    *energy += frame.iter().map(|v| v * v).sum::<f32>();
                }
            }
            assert!(energies.iter().all(|&energy| energy > 0.1));
            let min = energies[2..].iter().copied().reduce(f32::min).unwrap();
            let max = energies[2..].iter().copied().reduce(f32::max).unwrap();
            assert!(max > min * 1.1, "Missing wind gusts: {energies:?}");
            mixer.ambience(Ambience::default());
            for _ in 0..rate * 5 {
                mixer.frame();
            }
            assert_eq!(mixer.frame(), [0.0; 2]);
            assert!(mixer.ambience.is_none());
        }
    }

    #[test]
    fn continuous_rain_fades_under_cover_and_obeys_pause_mute_and_clear() {
        for rate in [22050, 44100, 48000] {
            let mut mixer = Mixer::new(rate);
            let rain = Ambience {
                rain: 1.0,
                wind: 0.0,
                exposure: 1.0,
            };
            mixer.ambience(rain);
            let mut energy = [0.0; 3];
            let mut stereo = 0.0;
            for (section, total) in energy.iter_mut().enumerate() {
                if section == 1 {
                    mixer.ambience(Ambience {
                        exposure: 0.0,
                        ..rain
                    });
                }
                if section == 2 {
                    mixer.ambience(Ambience::default());
                }
                for i in 0..rate * 5 {
                    let [l, r] = mixer.frame();
                    assert!(l.is_finite() && r.is_finite() && l.abs().max(r.abs()) < 0.8);
                    if i > rate * 3 {
                        *total += l * l + r * r;
                    }
                    stereo += (l - r).abs();
                    if i.is_multiple_of(rate / 30) && section == 0 {
                        mixer.ambience(rain);
                    }
                }
            }
            assert!(energy[0] > energy[1] * 8.0 && energy[1] > 0.0, "{energy:?}");
            assert!(energy[2] < energy[0] * 0.00001);
            assert!(stereo > 1.0);
            assert!(mixer.ambience.is_none());
            mixer.ambience(rain);
            for _ in 0..rate {
                mixer.frame();
            }
            mixer.paused = true;
            for _ in 0..rate {
                assert_eq!(mixer.frame(), [0.0; 2]);
            }
            mixer.paused = false;
            assert_ne!(mixer.frame(), [0.0; 2]);
            mixer.effects_volume = 0.0;
            assert_eq!(mixer.frame(), [0.0; 2]);
            mixer.effects_volume = 1.0;
            mixer.stereo = false;
            let [l, r] = mixer.frame();
            assert_eq!(l, r);
            mixer.clear_effects();
            assert_eq!(mixer.frame(), [0.0; 2]);
        }
    }
}
