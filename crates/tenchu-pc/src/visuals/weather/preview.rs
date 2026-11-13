use super::*;

impl Weather {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn seek_at_current_view(&mut self, seconds: u16, surfaces: &Surfaces, settings: &Settings) {
        self.seek(
            seconds,
            self.tick.unwrap_or(0),
            self.center.unwrap_or(Vec3::ZERO),
            surfaces,
            settings,
        );
    }

    pub fn seek(
        &mut self,
        seconds: u16,
        tick: u64,
        center: Vec3,
        surfaces: &Surfaces,
        settings: &Settings,
    ) {
        *self = Self::default();
        if settings.mode == WeatherMode::Original {
            return;
        }
        self.mode = settings.mode;
        self.center = Some(center);
        let ticks = u64::from(seconds.min(3600)) * 30;
        for step in 0..ticks {
            if step >= ticks.saturating_sub(30) {
                self.tick(step, center, surfaces, settings);
            } else {
                self.accumulate(settings);
                self.storm.tick(settings, center);
                self.tick = Some(step);
            }
        }
        self.tick = Some(tick);
        self.elapsed = Some(ticks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seek_matches_nominal_accumulation_and_continues_without_stepping_game_time() {
        let surfaces = Surfaces::default();
        for mode in [WeatherMode::Rain, WeatherMode::Snow, WeatherMode::Rainstorm] {
            let settings = Settings {
                mode,
                amount: 87,
                pooling: 137,
                ..Default::default()
            };
            let mut reference = Weather {
                mode,
                ..Default::default()
            };
            for tick in 0..45 * 30 {
                reference.tick(tick, Vec3::ZERO, &surfaces, &settings);
            }
            let mut preview = Weather::default();
            preview.seek(45, 10000, Vec3::ZERO, &surfaces, &settings);
            assert_eq!(
                (preview.wet, preview.puddles, preview.snow),
                (reference.wet, reference.puddles, reference.snow)
            );
            assert_eq!(preview.sample(&settings, 0.0).time, 45.0);
            reference.accumulate(&settings);
            preview.tick(10001, Vec3::ZERO, &surfaces, &settings);
            assert_eq!(
                (preview.wet, preview.puddles, preview.snow),
                (reference.wet, reference.puddles, reference.snow)
            );
            assert!((preview.sample(&settings, 0.0).time - (45.0 + 1.0 / 30.0)).abs() < 0.0001);
            preview.seek(0, 10001, Vec3::ZERO, &surfaces, &settings);
            assert_eq!(
                (preview.wet, preview.puddles, preview.snow),
                (0.0, 0.0, 0.0)
            );
            preview.seek(10, 10001, Vec3::ZERO, &surfaces, &settings);
            assert!(preview.wet > 0.0 || preview.snow > 0.0);
        }
    }
}
