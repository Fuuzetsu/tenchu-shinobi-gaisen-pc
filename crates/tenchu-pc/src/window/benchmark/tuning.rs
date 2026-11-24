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
    HeroModels,
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
    Change::HeroModels,
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
            Self::HeroModels => "High-detail heroes off".into(),
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
            Self::HeroModels => g.models.heroes = false,
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
    tuned: bool,
}

pub(super) struct Tuner {
    fps: u16,
    offscreen: bool,
    sample_counts: Vec<u32>,
    best: Option<Selection>,
    baseline: Option<Selection>,
    pending: Option<Change>,
    tried: Vec<Change>,
    screening: bool,
    validating: bool,
    pub trials: Vec<Trial>,
}

impl Tuner {
    pub fn new(fps: u16, offscreen: bool, sample_counts: Vec<u32>, screening: bool) -> Self {
        Self {
            fps,
            offscreen,
            sample_counts,
            best: None,
            baseline: None,
            pending: None,
            tried: Vec::new(),
            screening,
            validating: false,
            trials: Vec::new(),
        }
    }

    pub fn label(&self) -> String {
        if self.validating {
            return "Full-duration validation".into();
        }
        self.pending.map_or_else(
            || "Modern Ultra".into(),
            |change| {
                format!(
                    "{} {} / {} · {}",
                    if self.screening { "Screening" } else { "Trial" },
                    self.tried.len() + 1,
                    GPU.len() + CPU.len() + 1,
                    change.label()
                )
            },
        )
    }

    pub fn screening(&self) -> bool {
        self.screening && self.pending.is_some() && !self.validating
    }

    pub fn advance(
        &mut self,
        graphics: &mut Graphics,
        scenes: &[SceneResult],
        start: usize,
    ) -> bool {
        let assessment = Assessment::new(scenes, self.fps, self.offscreen);
        let previous = if self.validating {
            self.baseline.as_ref()
        } else {
            self.best.as_ref()
        };
        let accepted = previous.is_none_or(|best| {
            assessment.passed || assessment.excess < best.assessment.excess * 0.97
        });
        let reason = if self.validating && assessment.passed {
            "Full-duration validation meets the target in every scene"
        } else if self.validating && !accepted {
            "No clear full-duration improvement; restored the initial Ultra measurement"
        } else if self.validating {
            "Full-duration validation missed the target"
        } else if assessment.passed {
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
            change: if self.validating {
                "Full-duration validation".into()
            } else {
                self.pending
                    .map_or_else(|| "Modern Ultra".into(), Change::label)
            },
            accepted,
            budget_excess: assessment.excess,
            reason: reason.into(),
        });
        if self.validating && !accepted {
            self.best = self.baseline.take();
        } else if accepted {
            let tuned = self.best.as_ref().is_some_and(|best| best.tuned)
                || (!self.validating && self.pending.is_some());
            if self.screening() && self.baseline.is_none() {
                self.baseline = self.best.take();
            }
            self.best = Some(Selection {
                graphics: graphics.clone(),
                assessment,
                trial: self.trials.len() - 1,
                tuned,
            });
        }
        if self.validating {
            self.baseline = None;
            self.screening = false;
            self.validating = false;
        }
        let best = self.best.as_ref().unwrap();
        graphics.clone_from(&best.graphics);
        if best.assessment.passed || best.assessment.presentation_limited {
            return self.validate_selection();
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
        self.validate_selection()
    }

    fn validate_selection(&mut self) -> bool {
        self.validating = self.screening && self.best.as_ref().is_some_and(|best| best.tuned);
        self.validating
    }

    pub fn selected_trial(&self) -> usize {
        self.best.as_ref().unwrap().trial
    }

    pub fn passed(&self) -> bool {
        self.best.as_ref().unwrap().assessment.passed
    }

    pub fn quality(&self) -> &'static str {
        if !self.best.as_ref().unwrap().tuned {
            "Modern Ultra"
        } else {
            "Selectively tuned Ultra"
        }
    }

    pub fn note(&self) -> String {
        let assessment = &self.best.as_ref().unwrap().assessment;
        let mut note = if assessment.passed {
            "Validated every scene with measured timing headroom. Kept only adjustments that helped on this device.".to_owned()
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
                frames: 300,
                elapsed_seconds: 0.0,
                frame: measured(frame),
                cpu_work: measured(cpu),
                simulation_audio: Distribution::default(),
                geometry_uploads: Distribution::default(),
                overlay: Distribution::default(),
                submit: Distribution::default(),
                present_wait: Distribution::default(),
                gpu: gpu.map(measured).unwrap_or_default(),
                gpu_passes: Default::default(),
                gpu_scene: Default::default(),
            })
            .collect()
    }

    fn ultra() -> Graphics {
        starting_settings(&Settings::default()).graphics
    }

    #[test]
    fn tuning_follows_cpu_and_gpu_pressure_and_requires_every_scene_to_pass() {
        let mut tuner = Tuner::new(60, false, vec![1, 2, 4], false);
        let mut graphics = ultra();
        assert!(tuner.advance(&mut graphics, &scenes(35.0, Some(20.0), 40.0), 0));
        assert!(!graphics.models.heroes);
        assert_eq!(graphics.models.characters, 2);
        assert_eq!(graphics.lighting.resolution, 2048);
        assert!(tuner.advance(&mut graphics, &scenes(5.0, Some(20.0), 25.0), 4));
        assert!(!graphics.models.heroes);
        assert_eq!(graphics.models.characters, 2);
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
        let mut tuner = Tuner::new(60, true, vec![1, 2, 4], false);
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
        let mut tuner = Tuner::new(60, true, vec![1, 2, 4], false);
        let mut graphics = ultra();
        assert!(tuner.advance(&mut graphics, &scenes(20.0, Some(21.0), 45.0), 0));
        assert!(tuner.advance(&mut graphics, &scenes(20.0, Some(12.0), 35.0), 4));
        assert!(tuner.trials[1].accepted);
        assert_eq!(graphics.lighting.resolution, 1024);
        assert!(!graphics.models.heroes);
        assert_eq!(graphics.models.characters, 2);
    }

    #[test]
    fn display_limits_keep_quality_and_missing_gpu_times_use_observed_frames() {
        let mut tuner = Tuner::new(60, false, vec![1, 2, 4], false);
        let mut graphics = ultra();
        assert!(!tuner.advance(&mut graphics, &scenes(3.0, Some(4.0), 33.3), 0));
        assert!(!tuner.passed());
        assert_eq!(graphics, ultra());
        assert!(tuner.note().contains("display refresh"));

        let mut tuner = Tuner::new(60, false, vec![1, 2, 4], false);
        assert!(tuner.advance(&mut graphics, &scenes(3.0, None, 33.3), 0));
        assert!(!tuner.advance(&mut graphics, &scenes(3.0, None, 16.7), 4));
        assert!(tuner.passed());
        assert!(tuner.note().contains("GPU timestamps unavailable"));
    }

    #[test]
    fn short_screening_always_validates_the_selected_configuration() {
        let mut tuner = Tuner::new(60, true, vec![1, 2, 4], true);
        let mut graphics = ultra();
        assert!(!tuner.screening());
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(30.0), 40.0), 0));
        assert!(tuner.screening());
        let candidate = graphics.clone();
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(12.0), 16.0), 4));
        assert!(!tuner.screening());
        assert_eq!(tuner.label(), "Full-duration validation");
        assert_eq!(graphics, candidate);
        assert!(!tuner.advance(&mut graphics, &scenes(3.0, Some(13.0), 17.0), 8));
        assert!(tuner.passed());
        assert_eq!(tuner.selected_trial(), 2);
        assert_eq!(tuner.trials[2].first_scene, 8);
        assert_eq!(tuner.trials[2].scene_count, 4);
        assert_eq!(tuner.quality(), "Selectively tuned Ultra");
        assert_eq!(graphics, candidate);
    }

    #[test]
    fn full_ultra_baseline_stops_without_duplicate_validation() {
        for frame in [16.0, 33.3] {
            let mut tuner = Tuner::new(60, false, vec![1, 2, 4], true);
            let mut graphics = ultra();
            assert!(!tuner.screening());
            assert!(!tuner.advance(&mut graphics, &scenes(3.0, Some(12.0), frame), 0));
            assert_eq!(tuner.passed(), frame < 20.0);
            assert_eq!(tuner.trials.len(), 1);
            assert_eq!(tuner.selected_trial(), 0);
            assert_eq!(tuner.quality(), "Modern Ultra");
            assert_eq!(graphics, ultra());
        }
    }

    #[test]
    fn a_slow_scene_in_validation_resumes_full_measurements() {
        let mut tuner = Tuner::new(60, true, vec![1, 2, 4], true);
        let mut graphics = ultra();
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(30.0), 40.0), 0));
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(12.0), 16.0), 4));
        let mut full = scenes(3.0, Some(12.0), 16.0);
        full[3].gpu = Distribution::new(std::iter::repeat_n(22.0, 240));
        assert!(tuner.advance(&mut graphics, &full, 8));
        assert!(!tuner.passed());
        assert!(!tuner.screening());
        assert_eq!(graphics.samples, 2);
        // The validated cost replaces the optimistic short result as the comparison baseline.
        full[3].gpu = Distribution::new(std::iter::repeat_n(19.0, 240));
        assert!(tuner.advance(&mut graphics, &full, 12));
        assert!(tuner.trials[3].accepted);
        assert_eq!(graphics.samples, 2);
        assert!(!tuner.screening());
        assert!(!tuner.advance(&mut graphics, &scenes(3.0, Some(12.0), 16.0), 16));
        assert!(tuner.passed());
        assert_eq!(tuner.selected_trial(), 4);
    }

    #[test]
    fn exhausted_screening_only_validates_an_accepted_short_result() {
        for improve in [false, true] {
            let mut tuner = Tuner::new(120, true, vec![1, 2, 4], true);
            let mut graphics = ultra();
            let mut time = 200.0;
            let mut validations = 0;
            loop {
                if tuner.validating {
                    validations += 1;
                }
                let more = tuner.advance(
                    &mut graphics,
                    &scenes(2.0, Some(time), time + 20.0),
                    tuner.trials.len() * 4,
                );
                if improve {
                    time *= 0.96;
                }
                assert!(tuner.trials.len() <= GPU.len() + CPU.len() + 2);
                if !more {
                    break;
                }
            }
            assert_eq!(validations, usize::from(improve));
            assert!(!tuner.passed());
            if improve {
                assert_eq!(tuner.quality(), "Selectively tuned Ultra");
                assert_eq!(tuner.selected_trial(), tuner.trials.len() - 1);
            } else {
                assert_eq!(graphics, ultra());
                assert_eq!(tuner.quality(), "Modern Ultra");
                assert_eq!(tuner.selected_trial(), 0);
            }
        }
    }

    #[test]
    fn optimistic_short_result_cannot_replace_a_better_full_ultra_measurement() {
        let mut tuner = Tuner::new(60, true, vec![1, 2, 4], true);
        let mut graphics = ultra();
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(30.0), 40.0), 0));
        assert!(tuner.advance(&mut graphics, &scenes(3.0, Some(12.0), 16.0), 4));
        assert_eq!(tuner.label(), "Full-duration validation");
        let slower = scenes(3.0, Some(40.0), 50.0);
        assert!(tuner.advance(&mut graphics, &slower, 8));
        assert!(!tuner.trials[2].accepted);
        assert_eq!(tuner.selected_trial(), 0);
        assert_eq!(tuner.quality(), "Modern Ultra");
        assert_eq!(graphics.lighting.resolution, ultra().lighting.resolution);
        assert_eq!(graphics.samples, 2);
        while tuner.advance(&mut graphics, &slower, tuner.trials.len() * 4) {
            assert!(!tuner.screening());
            assert!(tuner.trials.len() <= GPU.len() + CPU.len() + 2);
        }
        assert!(!tuner.passed());
        assert_eq!(tuner.selected_trial(), 0);
        assert_eq!(graphics, ultra());
    }

    #[test]
    #[ignore = "requires TENCHU_BENCHMARK_REPORT; replays measured candidate timing and selection decisions"]
    fn measured_trials_preserve_their_selection_and_control_flow() {
        let report: serde_json::Value = serde_json::from_slice(
            &std::fs::read(std::env::var_os("TENCHU_BENCHMARK_REPORT").unwrap()).unwrap(),
        )
        .unwrap();
        let mut graphics: Graphics = serde_json::from_value(report["graphics"].clone()).unwrap();
        let mut tuner = Tuner::new(
            report["recommendation"]["target_fps"].as_u64().unwrap() as u16,
            report["offscreen"].as_bool().unwrap(),
            vec![1, 2, 4],
            report["screening_frames_per_scene"].as_u64().unwrap()
                < report["frames_per_scene"].as_u64().unwrap(),
        );
        let trials = report["trials"].as_array().unwrap();
        let measured = report["scenes"].as_array().unwrap();
        for (index, trial) in trials.iter().enumerate() {
            let first = trial["first_scene"].as_u64().unwrap() as usize;
            let count = trial["scene_count"].as_u64().unwrap() as usize;
            let expected: Graphics =
                serde_json::from_value(measured[first]["graphics"].clone()).unwrap();
            assert_eq!(graphics, expected, "trial {index}");
            assert_eq!(tuner.label(), measured[first]["quality"].as_str().unwrap());
            let costs: Vec<_> = measured[first..first + count]
                .iter()
                .map(|scene| {
                    scenes(
                        scene["cpu_work"]["p95_ms"].as_f64().unwrap() as f32,
                        (scene["gpu"]["samples"].as_u64().unwrap() != 0)
                            .then(|| scene["gpu"]["p95_ms"].as_f64().unwrap() as f32),
                        scene["frame"]["p95_ms"].as_f64().unwrap() as f32,
                    )
                    .pop()
                    .unwrap()
                })
                .collect();
            assert_eq!(
                tuner.advance(&mut graphics, &costs, first),
                index + 1 < trials.len(),
                "trial {index} continuation"
            );
            assert_eq!(
                tuner.trials[index].accepted,
                trial["accepted"].as_bool().unwrap()
            );
        }
        assert_eq!(
            tuner.selected_trial(),
            report["selected_trial"].as_u64().unwrap() as usize
        );
        assert_eq!(
            tuner.passed(),
            report["recommendation"]["met_target"].as_bool().unwrap()
        );
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
        let mut tuner = Tuner::new(60, true, vec![1, 4], false);
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
