#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rainfall {
    pub volume: f32,
    pub exposure: f32,
}

pub(super) struct Rain {
    target: Rainfall,
    current: Rainfall,
    seed: u32,
    filters: [[f32; 3]; 2],
    coefficients: [f32; 3],
    gains: [f32; 3],
    smoothing: f32,
    wash: f32,
    wash_step: f32,
}

impl Rain {
    pub fn new(rate: u32) -> Self {
        let coefficients = [130.0, 1100.0, 6500.0]
            .map(|hz| 1.0 - (-std::f32::consts::TAU * hz / rate as f32).exp());
        Self {
            target: Rainfall::default(),
            current: Rainfall::default(),
            seed: 0x728ac91d,
            filters: [[0.0; 3]; 2],
            coefficients,
            gains: coefficients.map(|a| ((2.0 - a) / a).sqrt()),
            smoothing: 1.0 - (-1.0 / (rate as f32 * 0.35)).exp(),
            wash: 0.0,
            wash_step: 1.0 - (-1.0 / (rate as f32 * 0.8)).exp(),
        }
    }

    pub fn set(&mut self, rainfall: Rainfall) {
        self.target = Rainfall {
            volume: rainfall.volume.clamp(0.0, 1.0),
            exposure: rainfall.exposure.clamp(0.0, 1.0),
        };
    }

    pub fn finished(&self) -> bool {
        self.target.volume == 0.0 && self.current.volume < 0.00001
    }

    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / 8388608.0 - 1.0
    }

    pub fn frame(&mut self) -> [f32; 2] {
        self.current.volume += (self.target.volume - self.current.volume) * self.smoothing;
        self.current.exposure += (self.target.exposure - self.current.exposure) * self.smoothing;
        self.wash += (self.noise() - self.wash) * self.wash_step;
        let wash = (1.0 + self.wash * 45.0).clamp(0.6, 1.4);
        let exposure = self.current.exposure;
        let gain = self.current.volume * (0.28 + 0.72 * exposure) * wash;
        std::array::from_fn(|channel| {
            let noise = self.noise();
            let filters = &mut self.filters[channel];
            for (filter, coefficient) in filters.iter_mut().zip(self.coefficients) {
                *filter += (noise - *filter) * coefficient;
            }
            let [low, middle, high] = std::array::from_fn(|i| filters[i] * self.gains[i]);
            (low * 0.035 + middle * (0.015 + 0.035 * exposure) + high * 0.025 * exposure) * gain
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mixer;

    #[test]
    fn continuous_rain_fades_under_cover_and_obeys_pause_mute_and_clear() {
        for rate in [22050, 44100, 48000] {
            let mut mixer = Mixer::new(rate);
            let rain = Rainfall {
                volume: 1.0,
                exposure: 1.0,
            };
            mixer.rainfall(rain);
            let mut energy = [0.0; 3];
            let mut stereo = 0.0;
            for (section, total) in energy.iter_mut().enumerate() {
                if section == 1 {
                    mixer.rainfall(Rainfall {
                        exposure: 0.0,
                        ..rain
                    });
                }
                if section == 2 {
                    mixer.rainfall(Rainfall::default());
                }
                for i in 0..rate * 5 {
                    let [l, r] = mixer.frame();
                    assert!(l.is_finite() && r.is_finite() && l.abs().max(r.abs()) < 0.8);
                    if i > rate * 3 {
                        *total += l * l + r * r;
                    }
                    stereo += (l - r).abs();
                    if i.is_multiple_of(rate / 30) && section == 0 {
                        mixer.rainfall(rain);
                    }
                }
            }
            assert!(energy[0] > energy[1] * 8.0 && energy[1] > 0.0, "{energy:?}");
            assert!(energy[2] < energy[0] * 0.00001);
            assert!(stereo > 1.0);
            assert!(mixer.rain.is_none());
            mixer.rainfall(rain);
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
