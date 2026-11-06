use super::{ThunderCue, ThunderStyle};

struct Echo {
    line: Vec<f32>,
    index: usize,
    filtered: f32,
}

pub(crate) struct Rolling {
    seed: u32,
    age: u32,
    rate: u32,
    filters: [f32; 4],
    coefficients: [f32; 4],
    gains: [f32; 4],
    rolls: [[f32; 3]; 4],
    echoes: [Echo; 4],
    damping: f32,
    volume: [f32; 2],
    retro: bool,
    cracks: Option<[[f32; 3]; 6]>,
    sample_clock: u32,
    held: [f32; 2],
}

fn noise(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as f32 / 8388608.0 - 1.0
}

impl Rolling {
    pub fn new(cue: ThunderCue, rate: u32) -> Self {
        let mut seed = cue.seed | 1;
        let coefficients = [45.0, 180.0, 900.0, 4000.0]
            .map(|hz| 1.0 - (-std::f32::consts::TAU * hz / rate as f32).exp());
        let rolls = std::array::from_fn(|i| {
            [
                if i == 0 {
                    0.0
                } else {
                    i as f32 * 0.85 + noise(&mut seed) * 0.35
                },
                3.5 + noise(&mut seed) * 0.7,
                [0.85, 0.45, 0.35, 0.25][i],
            ]
        });
        let strength = cue.strength.clamp(0.0, 1.0);
        let pan = cue.pan.clamp(-1.0, 1.0);
        let cracks = (cue.style == ThunderStyle::Natural).then(|| {
            std::array::from_fn(|i| {
                [
                    if i == 0 {
                        0.0
                    } else {
                        i as f32 * 0.21 + noise(&mut seed) * 0.10
                    },
                    0.10 + (noise(&mut seed) + 1.0) * 0.11,
                    (0.55 + noise(&mut seed) * 0.25) / (1.0 + i as f32 * 0.28),
                ]
            })
        });
        Self {
            seed,
            age: 0,
            rate,
            filters: [0.0; 4],
            coefficients,
            gains: coefficients.map(|a| ((2.0 - a) / a).sqrt()),
            rolls,
            echoes: [0.071, 0.113, 0.179, 0.293].map(|seconds| Echo {
                line: vec![0.0; (rate as f32 * seconds).round().max(1.0) as usize],
                index: 0,
                filtered: 0.0,
            }),
            damping: 1.0 - (-std::f32::consts::TAU * 1500.0 / rate as f32).exp(),
            volume: [
                strength * (1.0 - pan.max(0.0)),
                strength * (1.0 + pan.min(0.0)),
            ],
            retro: cue.style == ThunderStyle::Retro,
            cracks,
            sample_clock: 0,
            held: [0.0; 2],
        }
    }

    pub fn finished(&self) -> bool {
        self.age >= self.rate * 10
    }

    pub fn frame(&mut self) -> [f32; 2] {
        let random = noise(&mut self.seed);
        for (filter, coefficient) in self.filters.iter_mut().zip(self.coefficients) {
            *filter += (random - *filter) * coefficient;
        }
        let time = self.age as f32 / self.rate as f32;
        self.age += 1;
        let envelope: f32 = self
            .rolls
            .iter()
            .map(|[start, duration, gain]| {
                let age = time - start;
                let rise = (age / 0.08).clamp(0.0, 1.0);
                let fall = (1.0 - age / duration).clamp(0.0, 1.0);
                gain * rise * fall * fall
            })
            .sum();
        let [bass, body, grit, crack] = std::array::from_fn(|i| self.filters[i] * self.gains[i]);
        let attack = (time / 0.006).min(1.0) * (1.0 - time / 0.23).max(0.0);
        let dry = if let Some(cracks) = &self.cracks {
            let fracture: f32 = cracks
                .iter()
                .map(|[start, duration, gain]| {
                    let age = time - start;
                    let rise = (age / 0.002).clamp(0.0, 1.0);
                    let fall = (1.0 - age / duration).clamp(0.0, 1.0);
                    gain * rise * fall * fall
                })
                .sum();
            let distance = (time / 3.0).clamp(0.0, 1.0);
            (bass * 0.22 + body * (0.16 - distance * 0.10) + grit * 0.018) * envelope
                + ((crack - grit) * 0.22 + grit * 0.12 + body * 0.10) * fracture
        } else {
            (bass * 0.19 + body * 0.12 + grit * 0.025) * envelope + crack * 0.15 * attack
        };
        let taps: [f32; 4] = std::array::from_fn(|i| {
            let echo = &mut self.echoes[i];
            echo.filtered += (echo.line[echo.index] - echo.filtered) * self.damping;
            echo.filtered
        });
        let sum = taps.iter().sum::<f32>() * 0.5;
        for (echo, tap) in self.echoes.iter_mut().zip(taps) {
            echo.line[echo.index] = dry * 0.3 + (sum - tap) * 0.74;
            echo.index = (echo.index + 1) % echo.line.len();
        }
        let tail = ((10.0 - time) / 1.5).clamp(0.0, 1.0);
        let mut output = [
            dry * 0.7 + (taps[0] + taps[2]) * 0.5,
            dry * 0.7 + (taps[1] + taps[3]) * 0.5,
        ]
        .map(|v| v * tail);
        if self.retro {
            self.sample_clock += 11025;
            if self.sample_clock >= self.rate {
                self.sample_clock %= self.rate;
                self.held = output.map(|v| (v * 128.0).round() / 128.0);
            }
            output = self.held;
        }
        std::array::from_fn(|i| output[i] * self.volume[i])
    }
}
