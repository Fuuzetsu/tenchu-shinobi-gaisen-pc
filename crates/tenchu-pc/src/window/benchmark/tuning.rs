use super::{FRAME_TOLERANCE, SceneResult, WORK_HEADROOM};
use crate::settings::{Graphics, Settings, presets};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    LocalResolution(u32),
    Samples(u32),
    LocalShadows(u16),
    DirectionalResolution(u32),
    ContactShading,
    SurfaceRelief,
    Filtering(u16),
    PostProcessing,
    LocalLights(u16),
    Characters(u16),
    Foliage(u16),
}

// Ordered by measured benefit versus visual loss; candidates are re-measured on this device.
const GPU: &[Change] = &[
    Change::LocalResolution(1024),
    Change::Samples(2),
    Change::LocalShadows(4),
    Change::DirectionalResolution(2048),
    Change::ContactShading,
    Change::LocalResolution(512),
    Change::LocalShadows(2),
    Change::Samples(1),
    Change::DirectionalResolution(1024),
    Change::SurfaceRelief,
    Change::Filtering(4),
    Change::LocalShadows(1),
    Change::PostProcessing,
    Change::LocalLights(4),
    Change::DirectionalResolution(512),
    Change::LocalLights(2),
];

const CPU: &[Change] = &[
    Change::Characters(1),
    Change::Foliage(3),
    Change::Characters(0),
    Change::Foliage(0),
];

impl Change {
    fn label(self) -> String {
        match self {
            Self::LocalResolution(n) => format!("Local shadow maps ≤ {n}"),
            Self::Samples(n) => format!("MSAA ≤ {n}×"),
            Self::LocalShadows(n) => format!("Local shadows ≤ {n}"),
            Self::DirectionalResolution(n) => format!("Sun/moon shadow map ≤ {n}"),
            Self::ContactShading => "Contact shading off".into(),
            Self::SurfaceRelief => "Surface relief off".into(),
            Self::Filtering(n) => format!("Anisotropy ≤ {n}×"),
            Self::PostProcessing => "Post-processing off".into(),
            Self::LocalLights(n) => format!("Local lights ≤ {n}"),
            Self::Characters(n) => {
                format!("Character detail: {}", ["Original", "Refined"][n as usize])
            }
            Self::Foliage(0) => "Foliage: Original".into(),
            Self::Foliage(_) => "Foliage: Botanical".into(),
        }
    }

    fn apply(self, g: &mut Graphics, sample_counts: &[u32]) -> bool {
        let previous = g.clone();
        match self {
            Self::LocalResolution(n) => g.lighting.resolution = g.lighting.resolution.min(n),
            Self::Samples(n) => {
                g.samples = sample_counts
                    .iter()
                    .copied()
                    .filter(|&supported| supported <= n.min(g.samples))
                    .max()
                    .unwrap_or(1);
            }
            Self::LocalShadows(n) => g.lighting.shadow_count = g.lighting.shadow_count.min(n),
            Self::DirectionalResolution(n) => {
                g.lighting.directional.resolution = g.lighting.directional.resolution.min(n);
            }
            Self::ContactShading => g.post.occlusion = 0,
            Self::SurfaceRelief => g.surface_detail = 0,
            Self::Filtering(n) => g.anisotropy = g.anisotropy.min(n),
            Self::PostProcessing => g.post.enabled = false,
            Self::LocalLights(n) => g.lighting.count = g.lighting.count.min(n),
            Self::Characters(n) => g.models.characters = g.models.characters.min(n),
            Self::Foliage(n) => g.models.foliage = g.models.foliage.min(n),
        }
        *g != previous
    }
}

pub(super) fn starting_settings(current: &Settings) -> Settings {
    let mut settings = current.clone();
    settings.set_visual_preset(presets::MODERN_ULTRA);
    settings
        .graphics
        .weather
        .clone_from(&current.graphics.weather);
    settings
}

#[derive(Serialize)]
pub(super) struct Trial {
    first_scene: usize,
    scene_count: usize,
    pub change: String,
    accepted: bool,
    budget_excess: f32,
    pub reason: String,
}

struct Assessment {
    cpu: f32,
    gpu: f32,
    excess: f32,
    passed: bool,
    gpu_measured: bool,
    presentation_limited: bool,
}

impl Assessment {
    fn new(scenes: &[SceneResult], fps: u16, offscreen: bool) -> Self {
        let budget = 1000.0 / f32::from(fps);
        let excess = |ms: f32, allowance: f32| (ms / allowance - 1.0).max(0.0);
        let mut cpu = 0.0;
        let mut gpu = 0.0;
        let mut frame = 0.0;
        let gpu_measured = scenes.iter().all(|s| s.gpu.samples > 0);
        for s in scenes {
            cpu += excess(s.cpu_work.p95_ms, budget * WORK_HEADROOM);
            gpu += excess(s.gpu.p95_ms, budget * WORK_HEADROOM);
            if !offscreen || s.gpu.samples == 0 {
                frame += excess(s.frame.p95_ms, budget * FRAME_TOLERANCE);
            }
        }
        Self {
            cpu,
            gpu,
            excess: cpu + gpu + frame,
            passed: !scenes.is_empty() && scenes.iter().all(|s| s.meets_target(fps, offscreen)),
            gpu_measured,
            presentation_limited: gpu_measured && cpu == 0.0 && gpu == 0.0 && frame > 0.0,
        }
    }
}

struct Selection {
    graphics: Graphics,
    assessment: Assessment,
    trial: usize,
}

pub(super) struct Tuner {
    fps: u16,
    offscreen: bool,
    sample_counts: Vec<u32>,
    best: Option<Selection>,
    pending: Option<Change>,
    tried: Vec<Change>,
    pub trials: Vec<Trial>,
}

impl Tuner {
    pub fn new(fps: u16, offscreen: bool, sample_counts: Vec<u32>) -> Self {
        Self {
            fps,
            offscreen,
            sample_counts,
            best: None,
            pending: None,
            tried: Vec::new(),
            trials: Vec::new(),
        }
    }

    pub fn label(&self) -> String {
        self.pending.map_or_else(
            || "Modern Ultra".into(),
            |change| format!("Trial {} · {}", self.trials.len() + 1, change.label()),
        )
    }

    pub fn advance(
        &mut self,
        graphics: &mut Graphics,
        scenes: &[SceneResult],
        start: usize,
    ) -> bool {
        let assessment = Assessment::new(scenes, self.fps, self.offscreen);
        let accepted = self.best.as_ref().is_none_or(|best| {
            assessment.passed || assessment.excess < best.assessment.excess * 0.97
        });
        let reason = if assessment.passed {
            "All scenes meet the target"
        } else if self.best.is_none() {
            "Initial measurement"
        } else if accepted {
            "Reduced time over budget by at least 3%"
        } else {
            "No clear improvement across the scenes; reverted"
        };
        self.trials.push(Trial {
            first_scene: start,
            scene_count: scenes.len(),
            change: self
                .pending
                .map_or_else(|| "Modern Ultra".into(), Change::label),
            accepted,
            budget_excess: assessment.excess,
            reason: reason.into(),
        });
        if accepted {
            self.best = Some(Selection {
                graphics: graphics.clone(),
                assessment,
                trial: self.trials.len() - 1,
            });
        }
        let best = self.best.as_ref().unwrap();
        graphics.clone_from(&best.graphics);
        if best.assessment.passed || best.assessment.presentation_limited {
            return false;
        }
        let cpu_first = best.assessment.cpu > best.assessment.gpu;
        let (first, second) = if cpu_first {
            (
                CPU,
                if best.assessment.gpu > 0.0 || !best.assessment.gpu_measured {
                    GPU
                } else {
                    &[][..]
                },
            )
        } else {
            (GPU, CPU)
        };
        for &change in first.iter().chain(second) {
            if !self.tried.contains(&change) && change.apply(graphics, &self.sample_counts) {
                self.tried.push(change);
                self.pending = Some(change);
                return true;
            }
        }
        false
    }

    pub fn selected_trial(&self) -> usize {
        self.best.as_ref().unwrap().trial
    }

    pub fn passed(&self) -> bool {
        self.best.as_ref().unwrap().assessment.passed
    }

    pub fn quality(&self) -> &'static str {
        if self.selected_trial() == 0 {
            "Modern Ultra"
        } else {
            "Selectively tuned Ultra"
        }
    }

    pub fn note(&self) -> String {
        let assessment = &self.best.as_ref().unwrap().assessment;
        let mut note = if assessment.passed {
            "Validated all four scenes with measured timing headroom. Kept only adjustments that helped on this device.".to_owned()
        } else if assessment.presentation_limited {
            "CPU/GPU work fits the budget, but observed frames miss the target. Kept visual quality; check display refresh, compositor or browser pacing.".to_owned()
        } else {
            "The target was not reached. Kept the best measured configuration; try a lower resolution or frame target, and close background workloads.".to_owned()
        };
        if !assessment.gpu_measured {
            note.push_str(" GPU timestamps unavailable: selection uses observed frames and CPU work; display limits can hide GPU improvements.");
        } else if self.offscreen {
            note.push_str(" Offscreen selection uses CPU/GPU work, not serialized frame intervals; confirm interactive frame pacing in the game.");
        }
        note.push_str(" Artwork, weather, hair/cloth, interpolation, modern water and gameplay are preserved. Re-run after changing resolution, artwork or hardware.");
        note
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::benchmark::Distribution;

    fn scenes(cpu: f32, gpu: Option<f32>, frame: f32) -> Vec<SceneResult> {
        let measured = |ms| Distribution::new(std::iter::repeat_n(ms, 240));
        (0..4)
            .map(|_| SceneResult {
                quality: String::new(),
                graphics: Graphics::default(),
                id: String::new(),
                title: String::new(),
                artwork: String::new(),
                frame: measured(frame),
                cpu_work: measured(cpu),
                simulation_audio: Distribution::default(),
                geometry_uploads: Distribution::default(),
                overlay: Distribution::default(),
                submit: Distribution::default(),
                present_wait: Distribution::default(),
                gpu: gpu.map(measured).unwrap_or_default(),
                gpu_passes: Default::default(),
            })
            .collect()
    }

    fn ultra() -> Graphics {
        starting_settings(&Settings::default()).graphics
    }

    #[test]
    fn tuning_follows_cpu_and_gpu_pressure_and_requires_every_scene_to_pass() {
        let mut tuner = Tuner::new(60, false, vec![1, 2, 4]);
        let mut graphics = ultra();
        assert!(tuner.advance(&mut graphics, &scenes(35.0, Some(20.0), 40.0), 0));
        assert_eq!(graphics.models.characters, 1);
        assert_eq!(graphics.lighting.resolution, 2048);
        assert!(tuner.advance(&mut graphics, &scenes(5.0, Some(20.0), 25.0), 4));
        assert_eq!(graphics.models.characters, 1);
        assert_eq!(graphics.lighting.resolution, 1024);
        let mut almost = scenes(5.0, Some(12.0), 16.7);
        almost[2].gpu = Distribution::new(std::iter::repeat_n(20.0, 240));
        assert!(tuner.advance(&mut graphics, &almost, 8));
        assert!(!tuner.passed());
        assert_eq!(graphics.samples, 2);
        assert!(!tuner.advance(&mut graphics, &scenes(5.0, Some(12.0), 16.7), 12));
        assert!(tuner.passed());
        assert_eq!(tuner.selected_trial(), 3);
        assert!(
            tuner
                .trials
                .iter()
                .all(|t| t.accepted && t.scene_count == 4)
        );
        assert_eq!(tuner.trials[3].first_scene, 12);
    }

    #[test]
    fn slower_and_negligible_changes_are_reverted_instead_of_cascading() {
        let mut tuner = Tuner::new(60, true, vec![1, 2, 4]);
        let mut graphics = ultra();
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(30.0), 40.0), 0));
        assert_eq!(graphics.lighting.resolution, 1024);
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(32.0), 40.0), 4));
        assert_eq!(graphics.lighting.resolution, 2048);
        assert_eq!(graphics.samples, 2);
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(29.99), 40.0), 8));
        assert_eq!(graphics.samples, 4);
        assert_eq!(graphics.lighting.shadow_count, 4);
        assert!(!tuner.trials[1].accepted && !tuner.trials[2].accepted);
        assert_eq!(tuner.selected_trial(), 0);
    }

    #[test]
    fn lowering_one_of_two_bottlenecks_is_useful_even_if_the_other_is_still_slow() {
        let mut tuner = Tuner::new(60, true, vec![1, 2, 4]);
        let mut graphics = ultra();
        assert!(tuner.advance(&mut graphics, &scenes(20.0, Some(21.0), 45.0), 0));
        assert!(tuner.advance(&mut graphics, &scenes(20.0, Some(12.0), 35.0), 4));
        assert!(tuner.trials[1].accepted);
        assert_eq!(graphics.lighting.resolution, 1024);
        assert_eq!(graphics.models.characters, 1);
    }

    #[test]
    fn display_limits_keep_quality_and_missing_gpu_times_use_observed_frames() {
        let mut tuner = Tuner::new(60, false, vec![1, 2, 4]);
        let mut graphics = ultra();
        assert!(!tuner.advance(&mut graphics, &scenes(3.0, Some(4.0), 33.3), 0));
        assert!(!tuner.passed());
        assert_eq!(graphics, ultra());
        assert!(tuner.note().contains("display refresh"));

        let mut tuner = Tuner::new(60, false, vec![1, 2, 4]);
        assert!(tuner.advance(&mut graphics, &scenes(3.0, None, 33.3), 0));
        assert!(!tuner.advance(&mut graphics, &scenes(3.0, None, 16.7), 4));
        assert!(tuner.passed());
        assert!(tuner.note().contains("GPU timestamps unavailable"));
    }

    #[test]
    fn search_is_bounded_and_preserves_weather_artwork_and_inexpensive_enhancements() {
        let mut current = Settings {
            retro: true,
            keep_enemies_active: true,
            sight_percent: 150,
            ..Default::default()
        };
        current.graphics.asset_pack = false;
        current.graphics.weather.mode = crate::settings::WeatherMode::Snow;
        current.graphics.weather.wind = 80;
        current.graphics.weather.amount = 55;
        current.graphics.weather.quality = 0;
        let settings = starting_settings(&current);
        assert_eq!(settings.graphics.weather, current.graphics.weather);
        assert!(!settings.graphics.asset_pack && !settings.retro);
        assert!(settings.keep_enemies_active && settings.sight_percent == 150);
        let mut graphics = settings.graphics.clone();
        let mut tuner = Tuner::new(60, true, vec![1, 4]);
        let mut time = 200.0;
        while tuner.advance(
            &mut graphics,
            &scenes(1.0, Some(time), time + 2.0),
            tuner.trials.len() * 4,
        ) {
            time *= 0.96;
            assert!(tuner.trials.len() <= GPU.len() + CPU.len());
            assert!([1, 4].contains(&graphics.samples));
            assert!(![1, 2].contains(&graphics.models.foliage));
            assert!(
                graphics.interpolation
                    && graphics.smooth_models
                    && !graphics.blob_shadows
                    && !graphics.fog
            );
            assert_eq!(graphics.weather, current.graphics.weather);
            assert!(!graphics.asset_pack);
            assert_eq!(graphics.motion, settings.graphics.motion);
            assert_eq!(graphics.water, settings.graphics.water);
            assert_eq!(graphics.effects, settings.graphics.effects);
            assert_eq!(graphics.blood, settings.graphics.blood);
            assert!(graphics.lighting.enabled && graphics.lighting.shadows);
        }
        assert!(!tuner.passed());
        assert!(graphics.lighting.shadow_count >= 1);
        assert!(graphics.lighting.count >= 2);
    }
}
