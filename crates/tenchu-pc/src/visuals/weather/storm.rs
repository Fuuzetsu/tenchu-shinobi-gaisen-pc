use crate::settings::Weather;
use tenchu_audio::ThunderCue;

#[derive(Default)]
pub(super) struct Storm {
    seed: u32,
    remaining: Option<u16>,
    age: Option<u16>,
    delay: u16,
    strength: f32,
    pan: f32,
    pub thunder: Option<ThunderCue>,
}

impl Storm {
    fn random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f32 / 16777216.0
    }

    pub fn tick(&mut self, settings: &Weather) {
        self.thunder = None;
        if !settings.mode.storm() {
            *self = Self::default();
            return;
        }
        if let Some(age) = &mut self.age {
            *age += 1;
            if *age == self.delay {
                self.thunder = Some(ThunderCue {
                    seed: self.seed,
                    strength: self.strength * f32::from(settings.thunder) / 100.0,
                    pan: self.pan,
                });
            }
        }
        if self.remaining.is_none_or(|ticks| ticks == 0) {
            let first = self.remaining.is_none();
            self.remaining = Some(
                (f32::from(settings.lightning_interval) * 30.0 * (0.8 + self.random() * 0.4))
                    as u16,
            );
            if !first {
                self.age = Some(0);
                self.strength = 0.6 + self.random() * 0.4;
                self.delay = (45.0 + self.random() * 90.0) as u16;
                self.pan = self.random() * 0.8 - 0.4;
            }
        } else if let Some(remaining) = &mut self.remaining {
            *remaining -= 1;
        }
    }

    pub fn flash(&self, settings: &Weather, alpha: f32) -> f32 {
        if !settings.mode.storm() {
            return 0.0;
        }
        let Some(age) = self.age else {
            return 0.0;
        };
        let age = f32::from(age) + alpha.clamp(0.0, 1.0);
        let rise = (age / 3.0).clamp(0.0, 1.0);
        let fall = (1.0 - (age - 3.0).max(0.0) / 15.0).max(0.0);
        rise * rise
            * (3.0 - 2.0 * rise)
            * fall
            * fall
            * self.strength
            * f32::from(settings.lightning)
            / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::WeatherMode;

    #[test]
    fn storms_schedule_single_soft_flashes_then_thunder_and_stop_when_disabled() {
        let mut settings = Weather {
            mode: WeatherMode::Rainstorm,
            lightning_interval: 8,
            ..Default::default()
        };
        let mut storm = Storm::default();
        let mut last_flash = None;
        let mut thunder = 0;
        let mut flash = 0.0;
        for tick in 0..1800 {
            storm.tick(&settings);
            let next = storm.flash(&settings, 0.0);
            assert!((0.0..=0.4).contains(&next));
            if next > 0.0 && flash == 0.0 {
                if let Some(last) = last_flash {
                    assert!(tick - last >= 190);
                }
                last_flash = Some(tick);
            }
            if let Some(cue) = storm.thunder {
                assert!((40..=140).contains(&(tick - last_flash.unwrap())));
                assert!((0.0..=0.6).contains(&cue.strength));
                thunder += 1;
            }
            flash = next;
        }
        assert!(thunder >= 5);
        settings.mode = WeatherMode::Clear;
        storm.tick(&settings);
        assert_eq!(storm.flash(&settings, 0.5), 0.0);
        assert_eq!(storm.thunder, None);
        settings.mode = WeatherMode::Snowstorm;
        storm.tick(&settings);
        assert_eq!(storm.flash(&settings, 0.5), 0.0);
    }
}
