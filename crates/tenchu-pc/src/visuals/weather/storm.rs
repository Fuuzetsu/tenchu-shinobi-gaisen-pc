use crate::render_mesh::{self, Vertex};
use crate::settings::Weather;
use glam::Vec3;
use tenchu_audio::ThunderCue;

#[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
pub(super) struct Storm {
    seed: u32,
    remaining: Option<u16>,
    age: Option<u16>,
    delay: u16,
    strength: f32,
    pan: f32,
    bolts: Vec<[Vec3; 2]>,
    #[serde(skip)]
    pub thunder: Option<ThunderCue>,
}

impl Storm {
    fn random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f32 / 16777216.0
    }

    pub fn tick(&mut self, settings: &Weather, center: Vec3) {
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
                    style: settings.thunder_style,
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
                self.strike(center);
            }
        } else if let Some(remaining) = &mut self.remaining {
            *remaining -= 1;
        }
    }

    fn strike(&mut self, center: Vec3) {
        self.bolts.clear();
        let angle = self.random() * std::f32::consts::TAU;
        let distance = 80000.0 + self.random() * 30000.0;
        let base = center + Vec3::new(angle.cos() * distance, 2000.0, angle.sin() * distance);
        let height = 40000.0 + self.random() * 20000.0;
        let mut previous = base - Vec3::Y * height;
        for i in 1..=16 {
            let point = base
                + Vec3::new(
                    (self.random() - 0.5) * 4500.0,
                    -height * (1.0 - i as f32 / 16.0),
                    (self.random() - 0.5) * 4500.0,
                );
            self.bolts.push([previous, point]);
            previous = point;
        }
        for index in [3, 7, 10] {
            let mut previous = self.bolts[index][1];
            let angle = self.random() * std::f32::consts::TAU;
            let direction = Vec3::new(angle.cos(), 0.55, angle.sin()) * 2400.0;
            for _ in 0..4 {
                let point = previous
                    + direction
                    + Vec3::new(
                        (self.random() - 0.5) * 1700.0,
                        (self.random() - 0.5) * 1200.0,
                        (self.random() - 0.5) * 1700.0,
                    );
                self.bolts.push([previous, point]);
                previous = point;
            }
        }
    }

    pub fn append(
        &self,
        vertices: &mut Vec<Vertex>,
        forward: Vec3,
        settings: &Weather,
        alpha: f32,
    ) {
        if !settings.storm_bolts || !settings.mode.storm() {
            return;
        }
        let age = f32::from(self.age.unwrap_or(u16::MAX)) + alpha.clamp(0.0, 1.0);
        let strength = self.flash(settings, alpha)
            * if age < 6.0 {
                1.0
            } else if age < 8.0 {
                0.1
            } else {
                0.55
            };
        if strength < 0.001 {
            return;
        }
        for (i, &[a, b]) in self.bolts.iter().enumerate() {
            let across = (b - a).cross(forward).normalize_or_zero();
            let width = if i < 16 { 90.0 } else { 40.0 };
            for (scale, color, opacity) in [
                (5.0, [0.22, 0.36, 0.65], (strength * 1.2).min(1.0)),
                (1.0, [1.4, 1.5, 1.65], (strength * 3.0).min(1.0)),
            ] {
                let start = vertices.len();
                super::quad(
                    vertices,
                    (a + b) * 0.5,
                    across * width * scale,
                    (b - a) * 0.53,
                    color,
                    opacity,
                );
                for v in &mut vertices[start..] {
                    v.uv[1] = 0.0;
                    v.flags = (v.flags & !render_mesh::FOG) | render_mesh::UNLIT;
                }
            }
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
    #[ignore = "requires TENCHU_DISC and TENCHU_STORM_REVIEW; captures sky bolts over Mission 1 without advancing gameplay"]
    fn mission_storm_comparison() {
        use crate::visuals::{models::Models, sky::Daylight};
        use glam::Mat4;
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU_STORM_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let game = crate::session::Options::default().load(&assets).unwrap();
        let before = postcard::to_allocvec(&game).unwrap();
        let mut graphics = crate::settings::Graphics {
            fog: false,
            ..Default::default()
        };
        graphics.weather.mode = WeatherMode::Rainstorm;
        graphics.weather.lightning = 70;
        graphics.weather.lightning_interval = 8;
        graphics.sky.enabled = true;
        let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
        let mut models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
        review.renderer.bind_models(&mut models);
        let stage = review.renderer.upload(&crate::render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let center = Vec3::from(game.player().position.to_array().map(|n| n as f32));
        let mut storm = Storm::default();
        for _ in 0..300 {
            storm.tick(&graphics.weather, center);
            if storm.age == Some(4) {
                break;
            }
        }
        let eye = center - Vec3::Y * 11000.0;
        let target = storm.bolts[8][1];
        let forward = (target - eye).normalize();
        let camera = Mat4::perspective_rh(0.85, 1280.0 / 720.0, 50.0, 250000.0)
            * Mat4::look_at_rh(eye, target, Vec3::NEG_Y);
        review.renderer.daylight = Some(Daylight::at(1320.0, &graphics.sky));
        review.renderer.weather.flash = storm.flash(&graphics.weather, 0.0);
        for enabled in [false, true] {
            graphics.weather.storm_bolts = enabled;
            review.configure(graphics.clone());
            let mut vertices = Vec::new();
            storm.append(&mut vertices, forward, &graphics.weather, 0.0);
            let count = vertices.len() as u32;
            let effects = review
                .renderer
                .upload_effects(crate::render_effects::Geometry {
                    vertices,
                    batches: vec![crate::render_effects::Batch {
                        blend: 5,
                        range: 0..count,
                    }],
                });
            review
                .capture(
                    camera,
                    &[&stage],
                    Some(&effects),
                    false,
                    &output.join(format!("bolts-{enabled}.png")),
                )
                .unwrap();
        }
        assert_ne!(
            std::fs::read(output.join("bolts-false.png")).unwrap(),
            std::fs::read(output.join("bolts-true.png")).unwrap(),
            "The comparison camera must see the sky bolt"
        );
        assert_eq!(before, postcard::to_allocvec(&game).unwrap());
    }

    #[test]
    fn bolts_keep_world_positions_through_movement_and_restore_and_can_be_hidden() {
        let mut settings = Weather {
            mode: WeatherMode::Rainstorm,
            lightning_interval: 8,
            ..Default::default()
        };
        let mut storm = Storm::default();
        for _ in 0..300 {
            storm.tick(&settings, Vec3::ZERO);
            if storm.age == Some(4) {
                break;
            }
        }
        assert_eq!(storm.age, Some(4));
        assert_eq!(storm.bolts.len(), 28);
        let points = storm.bolts.clone();
        let mut restored: Storm =
            postcard::from_bytes(&postcard::to_allocvec(&storm).unwrap()).unwrap();
        let mut vertices = Vec::new();
        storm.append(&mut vertices, Vec3::Z, &settings, 0.0);
        assert!(!vertices.is_empty());
        assert!(
            vertices
                .iter()
                .all(|v| v.position.iter().all(|n| n.is_finite()))
        );
        vertices.clear();
        settings.storm_bolts = false;
        storm.append(&mut vertices, Vec3::Z, &settings, 0.0);
        assert!(vertices.is_empty());
        assert!(storm.flash(&settings, 0.0) > 0.0);
        settings.storm_bolts = true;
        settings.lightning = 0;
        storm.append(&mut vertices, Vec3::Z, &settings, 0.0);
        assert!(vertices.is_empty());
        let mut heard = false;
        for _ in 0..150 {
            storm.tick(&settings, Vec3::splat(10000.0));
            restored.tick(&settings, Vec3::splat(10000.0));
            assert_eq!(storm.bolts, points);
            assert_eq!(storm.bolts, restored.bolts);
            assert_eq!(storm.thunder, restored.thunder);
            heard |= storm.thunder.is_some();
        }
        assert!(heard, "Hiding flashes must not mute the delayed thunder");
    }

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
            storm.tick(&settings, Vec3::ZERO);
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
        storm.tick(&settings, Vec3::ZERO);
        assert_eq!(storm.flash(&settings, 0.5), 0.0);
        assert_eq!(storm.thunder, None);
        settings.mode = WeatherMode::Snowstorm;
        storm.tick(&settings, Vec3::ZERO);
        assert_eq!(storm.flash(&settings, 0.5), 0.0);
    }
}
