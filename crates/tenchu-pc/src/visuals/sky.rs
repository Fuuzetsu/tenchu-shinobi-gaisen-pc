use crate::settings::Sky;
use glam::{Mat3, Vec3};

#[derive(Clone, Copy)]
pub struct Daylight {
    pub direction: Vec3,
    pub color: Vec3,
    pub strength: f32,
    pub ambient: f32,
    pub tint: f32,
    pub day: f32,
    pub horizon: Vec3,
    pub relighting: f32,
}

fn smooth(low: f32, high: f32, value: f32) -> f32 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Daylight {
    pub fn at(minutes: f64, settings: &Sky) -> Self {
        let phase = ((minutes - 360.0).rem_euclid(1440.0) / 1440.0) as f32 * std::f32::consts::TAU;
        let altitude = 60.0_f32.to_radians();
        let solar = Mat3::from_rotation_y(f32::from(settings.heading).to_radians())
            * Vec3::new(
                phase.cos(),
                -phase.sin() * altitude.sin(),
                phase.sin() * altitude.cos(),
            );
        let day = smooth(-0.18, 0.22, -solar.y);
        let sunset = (1.0 - smooth(0.0, 0.5, -solar.y)) * day;
        let sun = solar.y <= 0.0;
        let direction = if sun { solar } else { -solar };
        let horizon = Vec3::new(0.0007, 0.0014, 0.004)
            .lerp(Vec3::new(0.14, 0.28, 0.45), day)
            .lerp(Vec3::new(0.5, 0.15, 0.045), sunset * 0.7);
        Self {
            direction,
            color: if sun {
                Vec3::new(1.0, 0.42, 0.18)
                    .lerp(Vec3::new(1.0, 0.96, 0.86), smooth(0.0, 0.6, -solar.y))
            } else {
                Vec3::new(0.58, 0.72, 1.0)
            },
            strength: smooth(0.0, 0.25, -direction.y)
                * if sun {
                    f32::from(settings.sun.min(200)) / 125.0
                } else {
                    f32::from(settings.moon.min(200)) / 400.0
                },
            ambient: (f32::from(settings.night_ambient.min(100)) * (1.0 - day)
                + f32::from(settings.day_ambient.min(100)) * day)
                / 100.0,
            tint: 0.65 - day * 0.5,
            day,
            horizon,
            relighting: day * f32::from(settings.relighting.min(100)) / 100.0,
        }
    }
}

#[derive(Default)]
pub struct Clock {
    tick: Option<u64>,
    setting: u16,
    minutes: f64,
    previous: f64,
}

impl Clock {
    pub fn tick(&mut self, tick: u64, settings: &Sky) {
        if !settings.enabled {
            *self = Self::default();
            return;
        }
        if self.tick.is_none_or(|old| old + 1 != tick) || self.setting != settings.time {
            self.minutes = f64::from(settings.time % 1440);
            self.previous = self.minutes;
        } else {
            self.previous = self.minutes;
            self.minutes += f64::from(settings.speed.min(120)) / 30.0;
            if self.minutes >= 1440.0 {
                self.minutes -= 1440.0;
                self.previous -= 1440.0;
            }
        }
        self.tick = Some(tick);
        self.setting = settings.time;
    }

    pub fn minutes(&self, settings: &Sky, alpha: f32) -> f64 {
        if self.tick.is_none() || self.setting != settings.time {
            f64::from(settings.time % 1440)
        } else {
            (self.previous + (self.minutes - self.previous) * f64::from(alpha.clamp(0.0, 1.0)))
                .rem_euclid(1440.0)
        }
    }

    pub fn sample(&self, settings: &Sky, alpha: f32) -> Option<Daylight> {
        settings
            .enabled
            .then(|| Daylight::at(self.minutes(settings, alpha), settings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solar_cycle_is_continuous_and_daylight_dominates_moonlight() {
        let settings = Sky::default();
        let noon = Daylight::at(720.0, &settings);
        let midnight = Daylight::at(0.0, &settings);
        assert!(noon.strength > midnight.strength * 8.0);
        assert!(noon.ambient > midnight.ambient);
        for minute in 0..1440 {
            let a = Daylight::at(f64::from(minute), &settings);
            let b = Daylight::at(f64::from(minute + 1), &settings);
            assert!((a.direction.length() - 1.0).abs() < 0.0001);
            assert!(a.direction.y <= 0.0001);
            assert!(a.horizon.distance(b.horizon) < 0.01);
            assert!(
                a.direction
                    .mul_add(Vec3::splat(a.strength), -b.direction * b.strength)
                    .length()
                    < 0.02
            );
        }
    }

    #[test]
    fn sky_clock_ignores_display_cadence_and_interpolates_across_midnight() {
        let mut clock = Clock::default();
        let mut settings = Sky {
            enabled: true,
            time: 1439,
            speed: 60,
            ..Default::default()
        };
        clock.tick(1, &settings);
        clock.tick(2, &settings);
        assert_eq!(clock.minutes(&settings, 0.5), 0.0);
        assert_eq!(clock.minutes(&settings, 1.0), 1.0);
        for _ in 0..100 {
            clock.sample(&settings, 0.5);
        }
        clock.tick(3, &settings);
        assert_eq!(clock.minutes(&settings, 1.0), 3.0);
        settings.time = 600;
        assert_eq!(clock.minutes(&settings, 0.5), 600.0);
        clock.tick(4, &settings);
        assert_eq!(clock.minutes(&settings, 1.0), 600.0);
    }
}
