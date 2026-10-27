use super::*;
use serde::Serialize;

const WARMUP: u64 = 60;
const AUTO_QUALITIES: [u16; 5] = [
    settings::presets::MODERN_ULTRA,
    settings::presets::MODERN_HIGH,
    settings::presets::MODERN_MEDIUM,
    settings::presets::MODERN_LOW,
    0,
];

#[derive(Clone)]
pub struct Options {
    pub preset: Option<u16>,
    pub seconds: u16,
    pub size: Option<[u32; 2]>,
    pub output: Option<PathBuf>,
    pub exit: bool,
    pub offscreen: bool,
    pub auto_target: Option<u16>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            preset: None,
            seconds: 8,
            size: None,
            output: None,
            exit: false,
            offscreen: false,
            auto_target: None,
        }
    }
}

#[derive(Default)]
pub(super) struct Benchmarks {
    pub pending: Option<Options>,
    pub run: Option<Run>,
    pub report: Option<Report>,
    pub open: bool,
    target_fps: u16,
}

struct Saved {
    campaign: Campaign,
    settings: Settings,
    debug: crate::debug::Debug,
    paused: bool,
    fault: Option<String>,
    menu_open: bool,
    recording: Option<Recorder<BufWriter<File>>>,
}

pub(super) struct Run {
    pub target: Option<wgpu::TextureView>,
    saved: Saved,
    options: Options,
    pub settings: Settings,
    report: Report,
    samples: Samples,
    pub artwork: String,
    pub frame: u64,
    pub scene: usize,
    pub title: String,
    quality_step: usize,
}

#[derive(Serialize, Default, Debug)]
pub(super) struct Distribution {
    samples: usize,
    mean_ms: f32,
    p50_ms: f32,
    p95_ms: f32,
    p99_ms: f32,
}

impl Distribution {
    fn new(values: impl Iterator<Item = f32>) -> Self {
        let mut values: Vec<_> = values.filter(|v| v.is_finite() && *v >= 0.0).collect();
        if values.is_empty() {
            return Self::default();
        }
        values.sort_unstable_by(f32::total_cmp);
        let percentile = |p: usize| values[(values.len() * p).div_ceil(100).saturating_sub(1)];
        Self {
            samples: values.len(),
            mean_ms: values.iter().sum::<f32>() / values.len() as f32,
            p50_ms: percentile(50),
            p95_ms: percentile(95),
            p99_ms: percentile(99),
        }
    }
}

#[derive(Serialize)]
pub(super) struct SceneResult {
    quality: String,
    graphics: settings::Graphics,
    id: String,
    title: String,
    artwork: String,
    frame: Distribution,
    cpu_work: Distribution,
    simulation_audio: Distribution,
    geometry_uploads: Distribution,
    overlay: Distribution,
    submit: Distribution,
    present_wait: Distribution,
    gpu: Distribution,
    gpu_passes: [Distribution; 4],
}

#[derive(Serialize)]
pub(super) struct Report {
    schema: u8,
    adapter: String,
    backend: String,
    size: [u32; 2],
    browser: bool,
    offscreen: bool,
    warmup_frames: u64,
    frames_per_scene: u64,
    graphics: settings::Graphics,
    artwork: String,
    scenes: Vec<SceneResult>,
    cancelled: Option<String>,
    recommendation: Option<settings::library::Auto>,
}

#[derive(Default)]
struct Samples {
    cpu: Vec<performance::CpuFrame>,
    gpu: Vec<[f32; 4]>,
    gpu_serial: u64,
}

impl Samples {
    fn record(
        &mut self,
        frame: u64,
        cpu: performance::CpuFrame,
        gpu: Option<crate::renderer::timing::Sample>,
        submitted: u64,
    ) {
        if frame > WARMUP {
            self.cpu.push(cpu);
            if let Some(sample) = gpu
                && sample.serial > self.gpu_serial
            {
                self.gpu.push(sample.milliseconds);
                self.gpu_serial = sample.serial;
            }
        } else {
            self.gpu_serial = submitted;
        }
    }
}

impl Run {
    pub fn record(
        &mut self,
        cpu: performance::CpuFrame,
        timer: Option<&crate::renderer::timing::Timer>,
    ) {
        self.samples.record(
            self.frame,
            cpu,
            timer.and_then(|t| t.latest),
            timer.map_or(0, |t| t.submitted_serial()),
        );
    }

    fn finish_scene(&mut self, player: &crate::demo::Player) {
        let Samples { cpu, gpu, .. } = &mut self.samples;
        if cpu.is_empty() {
            return;
        }
        self.report.scenes.push(SceneResult {
            quality: self
                .options
                .preset
                .map_or("Current graphics", |i| settings::presets::NAMES[i as usize])
                .into(),
            graphics: self.settings.graphics.clone(),
            id: player.clip().id.clone(),
            title: self.title.clone(),
            artwork: self.artwork.clone(),
            frame: Distribution::new(cpu.iter().map(|f| f.interval)),
            cpu_work: Distribution::new(
                cpu.iter()
                    .map(|f| f.simulation + f.prepare + f.overlay + f.draw.submit),
            ),
            simulation_audio: Distribution::new(cpu.iter().map(|f| f.simulation)),
            geometry_uploads: Distribution::new(cpu.iter().map(|f| f.prepare)),
            overlay: Distribution::new(cpu.iter().map(|f| f.overlay)),
            submit: Distribution::new(cpu.iter().map(|f| f.draw.submit)),
            present_wait: Distribution::new(cpu.iter().map(|f| f.draw.wait)),
            gpu: Distribution::new(gpu.iter().map(|p| p.iter().sum())),
            gpu_passes: std::array::from_fn(|i| Distribution::new(gpu.iter().map(|p| p[i]))),
        });
        cpu.clear();
        gpu.clear();
    }
}

impl SceneResult {
    fn meets_target(&self, fps: u16) -> bool {
        meets_target(&self.frame, &self.cpu_work, &self.gpu, fps)
    }
}

fn meets_target(frame: &Distribution, cpu: &Distribution, gpu: &Distribution, fps: u16) -> bool {
    let budget = 1000.0 / f32::from(fps);
    frame.samples >= 60
        && cpu.p95_ms <= budget * 0.85
        && frame.p95_ms <= budget * 1.12
        && (gpu.samples == 0 || gpu.p95_ms <= budget * 0.85)
}

impl Run {
    fn recommend(&mut self, passed: bool) {
        let fps = self.options.auto_target.unwrap();
        let quality = settings::presets::NAMES[self.options.preset.unwrap() as usize];
        let gpu = self
            .report
            .scenes
            .iter()
            .filter(|s| s.quality == quality)
            .all(|s| s.gpu.samples > 0);
        let mut note = if passed {
            "Highest tested tier meeting the frame target, with CPU/GPU headroom. Re-run after changing resolution, artwork or hardware.".to_owned()
        } else {
            "No tier met the target at this resolution. Using Original as a fallback; try a lower resolution or frame target. Display/compositor limits can also prevent reaching the target.".to_owned()
        };
        if !gpu {
            note.push_str(" GPU timestamps unavailable: selection uses observed frames and CPU work, with less certainty about GPU headroom.");
        }
        self.report.recommendation = Some(settings::library::Auto {
            visual: settings::library::Visual::capture(&self.settings),
            quality: quality.into(),
            adapter: self.report.adapter.clone(),
            size: self.report.size,
            target_fps: fps,
            met_target: passed,
            note,
        });
    }
}

impl Benchmarks {
    pub fn new(pending: Option<Options>) -> Self {
        Self {
            pending,
            target_fps: 60,
            ..Default::default()
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut cancel = false;
        if let Some(run) = &self.run {
            egui::Window::new("Performance benchmark")
                .collapsible(false)
                .anchor(egui::Align2::LEFT_TOP, [16.0, 16.0])
                .show(ctx, |ui| {
                    ui.label(&run.title);
                    if run.options.auto_target.is_some() {
                        ui.strong(settings::presets::NAMES[run.options.preset.unwrap() as usize]);
                    }
                    ui.label(format!(
                        "Scene {} · {} / {} frames",
                        run.scene + 1,
                        run.frame,
                        run.report.frames_per_scene
                    ));
                    ui.weak(if run.frame <= WARMUP {
                        "Warming up (excluded from results)"
                    } else {
                        "Measuring CPU and GPU · fixed 60 Hz presentation path"
                    });
                    cancel = ui.button("Cancel (Esc)").clicked();
                });
        } else if self.open {
            egui::Window::new("Benchmarks").open(&mut self.open).default_width(520.0).show(ctx, |ui| {
                ui.label("Run fixed camera paths through lights, architecture, vegetation and water.");
                ui.weak("Loading and the first 60 frames of each scene are excluded. Your game and settings are restored afterwards. Keep the window focused and its size unchanged.");
                if ui.button("Benchmark current graphics").clicked() {
                    self.pending = Some(Options::default());
                }
                ui.horizontal(|ui| {
                    if self.target_fps == 0 { self.target_fps = 60; }
                    egui::ComboBox::from_id_salt("benchmark-target").selected_text(format!("{} FPS", self.target_fps)).show_ui(ui, |ui| {
                        for fps in [30, 60, 120] { ui.selectable_value(&mut self.target_fps, fps, format!("{fps} FPS")); }
                    });
                    if ui.button("Auto-optimise graphics").clicked() {
                        self.pending = Some(Options { auto_target: Some(self.target_fps), seconds: 5, ..Default::default() });
                    }
                });
                ui.weak("Automatic optimisation tests quality tiers and applies the best passing result as Auto-optimised. Gameplay settings are kept.");
                if let Some(report) = &self.report {
                    ui.separator();
                    ui.label(format!("{} · {} · {} × {}", report.adapter, report.backend, report.size[0], report.size[1]));
                    ui.weak(&report.artwork);
                    if let Some(reason) = &report.cancelled { ui.colored_label(egui::Color32::YELLOW, reason); }
                    ui.weak("Times in milliseconds. Lower is better; p95 includes the slowest 5% boundary.");
                    if let Some(auto) = &report.recommendation { ui.strong(format!("Auto-optimised: {}", auto.quality)); ui.label(&auto.note); }
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| { egui::Grid::new("benchmark-results").striped(true).show(ui, |ui| {
                        for heading in ["Quality / scene", "FPS", "Frame p95", "CPU p95", "GPU p95"] { ui.strong(heading); }
                        ui.end_row();
                        for scene in &report.scenes {
                            ui.label(format!("{}\n{}", scene.quality, scene.title));
                            ui.label(format!("{:.0}", 1000.0 / scene.frame.mean_ms.max(0.001)));
                            ui.label(format!("{:.2}", scene.frame.p95_ms));
                            ui.label(format!("{:.2}", scene.cpu_work.p95_ms));
                            ui.label(if scene.gpu.samples == 0 { "Unavailable".into() } else { format!("{:.2}", scene.gpu.p95_ms) });
                            ui.end_row();
                        }
                    }); });
                    ui.weak("GPU passes: shadows, scene, post-processing, HUD. Browser frame rate may be limited by the display; missing GPU timestamps do not mean zero cost.");
                }
            });
        }
        cancel
    }
}

impl Application {
    pub(super) fn start_benchmark(&mut self, mut options: Options) -> Result<()> {
        if self.demo.is_some() || self.benchmarks.run.is_some() {
            return Err("Finish the current demo first".into());
        }
        #[cfg(target_arch = "wasm32")]
        if !self.browser_ready() {
            return Err(
                "The campaign is still loading in the background. Try again once it is ready."
                    .into(),
            );
        }
        let player = crate::demo::Player::benchmark(options.seconds)?;
        let campaign = Campaign::new(&self.assets, self.campaign.options, None)?;
        let debug = crate::debug::Debug::new(&campaign.game)?;
        let view = self.view.as_mut().ok_or("Window is not ready")?;
        let saved = Saved {
            campaign: std::mem::replace(&mut self.campaign, campaign),
            debug: std::mem::replace(&mut self.debug, debug),
            settings: self.settings.clone(),
            paused: self.paused,
            fault: self.fault.take(),
            menu_open: self.menu.open,
            recording: self.recording.take(),
        };
        let mut settings = self.settings.rendering().into_owned();
        if options.auto_target.is_some() {
            options.preset = Some(AUTO_QUALITIES[0]);
        }
        if let Some(preset) = options.preset {
            settings.set_visual_preset(preset);
        }
        settings.retro = false;
        settings.show_performance = false;
        settings.show_fps = false;
        settings.vsync = false;
        settings.pause_on_defocus = true;
        settings.volume = 0;
        settings.graphics.samples = view
            .renderer
            .sample_counts
            .iter()
            .copied()
            .filter(|&n| n <= settings.graphics.samples)
            .max()
            .unwrap_or(1);
        if let Some(size) = options.size {
            let _ = view
                .window
                .request_inner_size(winit::dpi::PhysicalSize::new(size[0], size[1]));
            view.resize(size[0], size[1]);
        }
        let report = Report {
            schema: 1,
            adapter: view.adapter.name.clone(),
            backend: format!("{:?}", view.adapter.backend),
            size: view.size,
            browser: cfg!(target_arch = "wasm32"),
            offscreen: options.offscreen,
            warmup_frames: WARMUP,
            frames_per_scene: u64::from(options.seconds) * 60,
            graphics: settings.graphics.clone(),
            artwork: view.artwork_status(),
            scenes: Vec::new(),
            cancelled: None,
            recommendation: None,
        };
        self.benchmarks.run = Some(Run {
            target: options.offscreen.then(|| {
                view.renderer
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("Benchmark target"),
                        size: wgpu::Extent3d {
                            width: view.size[0],
                            height: view.size[1],
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: view.config.format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                    .create_view(&Default::default())
            }),
            saved,
            options,
            settings,
            report,
            samples: Samples::default(),
            artwork: String::new(),
            frame: 0,
            scene: 0,
            title: String::new(),
            quality_step: 0,
        });
        self.demo = Some(player);
        self.benchmarks.open = false;
        self.release_pointer();
        if let Err(error) = self.start_demo_clip() {
            self.finish_benchmark(Some(error))?;
        }
        Ok(())
    }

    pub(super) fn benchmark_frame(&mut self) -> Result<()> {
        let Some(run) = self.benchmarks.run.as_mut() else {
            return Ok(());
        };
        let player = self.demo.as_mut().unwrap();
        if player.ended() {
            if run.samples.cpu.is_empty() {
                return self.finish_benchmark(Some(
                    "No frames were measured; keep the game visible".into(),
                ));
            }
            run.finish_scene(player);
            let failed = run
                .options
                .auto_target
                .is_some_and(|fps| !run.report.scenes.last().unwrap().meets_target(fps));
            if failed || !player.next() {
                if run.options.auto_target.is_some() {
                    if !failed || run.quality_step + 1 == AUTO_QUALITIES.len() {
                        run.recommend(!failed);
                        return self.finish_benchmark(None);
                    }
                    run.quality_step += 1;
                    let preset = AUTO_QUALITIES[run.quality_step];
                    run.options.preset = Some(preset);
                    run.settings.set_visual_preset(preset);
                    run.settings.graphics.samples = self
                        .view
                        .as_ref()
                        .unwrap()
                        .renderer
                        .sample_counts
                        .iter()
                        .copied()
                        .filter(|&n| n <= run.settings.graphics.samples)
                        .max()
                        .unwrap_or(1);
                    player.reset(0);
                } else {
                    return self.finish_benchmark(None);
                }
            }
            run.frame = 0;
            if let Err(error) = self.start_demo_clip() {
                return self.finish_benchmark(Some(error));
            }
        }
        self.advance_demo_frame();
        if self.faulted {
            return self.finish_benchmark(self.fault.clone());
        }
        let run = self.benchmarks.run.as_mut().unwrap();
        let player = self.demo.as_ref().unwrap();
        run.frame = player.frame;
        run.scene = player.index;
        run.title.clone_from(&player.clip().title);
        Ok(())
    }

    pub(super) fn finish_benchmark(&mut self, cancelled: Option<String>) -> Result<()> {
        let Some(mut run) = self.benchmarks.run.take() else {
            return Ok(());
        };
        if run.options.exit {
            self.error = cancelled.clone();
        }
        run.report.cancelled = cancelled;
        self.demo = None;
        self.campaign = run.saved.campaign;
        self.settings = run.saved.settings;
        self.debug = run.saved.debug;
        self.paused = run.saved.paused;
        self.fault = run.saved.fault;
        self.faulted = self.fault.is_some();
        self.menu.open = run.saved.menu_open;
        self.recording = run.saved.recording;
        let optimisation = if !run.options.exit && run.report.cancelled.is_none() {
            run.report
                .recommendation
                .as_ref()
                .map(|auto| {
                    self.presets.save_auto(auto.clone())?;
                    auto.visual.apply(&mut self.settings);
                    self.settings_store.write(&self.settings)
                })
                .transpose()
        } else {
            Ok(None)
        };
        self.benchmarks.report = Some(run.report);
        self.benchmarks.open = true;
        self.demo_exit = run.options.exit;
        let report = self.benchmarks.report.as_ref().unwrap();
        for s in &report.scenes {
            println!(
                "Benchmark {} / {}: {:.1} FPS, frame p95 {:.2} ms, CPU p95 {:.2} ms, GPU p95 {:.2} ms ({} samples)",
                s.quality,
                s.id,
                1000.0 / s.frame.mean_ms.max(0.001),
                s.frame.p95_ms,
                s.cpu_work.p95_ms,
                s.gpu.p95_ms,
                s.gpu.samples
            );
        }
        let export = run
            .options
            .output
            .as_ref()
            .map(|path| {
                let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
                std::fs::write(path, bytes)
                    .map_err(|e| format!("Benchmark report {}: {e}", path.display()))
            })
            .transpose();
        if let Some(view) = &mut self.view {
            view.reload(&self.campaign.game, &self.settings)?;
            view.settings(&self.settings, &self.campaign.game)?;
        }
        if let Some(audio) = &mut self.audio {
            audio.reload(&self.assets, &self.campaign.game)?;
            audio.screen(&self.campaign.screen);
            audio.volume(self.settings.volume);
        }
        self.controls.clear();
        self.sync_pause();
        self.update_view();
        optimisation?;
        export?;
        Ok(())
    }
}

impl View {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn benchmark_draw(
        &mut self,
        target: &wgpu::TextureView,
        game: &Game,
        debug: &crate::debug::Debug,
    ) -> Result<Option<performance::DrawTimes>> {
        let start = Instant::now();
        self.render(target, game, debug)?;
        let submit = start.elapsed().as_secs_f32() * 1000.0;
        let start = Instant::now();
        self.renderer.device.poll(wgpu::Maintain::Wait);
        Ok(Some(performance::DrawTimes {
            submit,
            wait: start.elapsed().as_secs_f32() * 1000.0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_checks_slow_frames_and_headroom_instead_of_average_fps() {
        let measured = |ms| Distribution::new(std::iter::repeat_n(ms, 240));
        let frame = measured(16.7);
        let cpu = measured(3.0);
        assert!(meets_target(&frame, &cpu, &measured(12.0), 60));
        assert!(!meets_target(&frame, &cpu, &measured(15.0), 60));
        assert!(!meets_target(&frame, &measured(15.0), &measured(3.0), 60));
        let stutters =
            Distribution::new(std::iter::repeat_n(12.0, 220).chain(std::iter::repeat_n(32.0, 20)));
        assert!(stutters.mean_ms < 16.7);
        assert!(!meets_target(&stutters, &cpu, &measured(3.0), 60));
        assert!(meets_target(&frame, &cpu, &Distribution::default(), 60));
        assert!(!meets_target(
            &measured(33.3),
            &cpu,
            &Distribution::default(),
            60
        ));
        assert!(!meets_target(
            &Distribution::default(),
            &cpu,
            &measured(3.0),
            60
        ));
    }

    #[test]
    fn warmup_delayed_and_duplicate_gpu_samples_do_not_pollute_the_measurement() {
        let mut samples = Samples::default();
        let cpu = performance::CpuFrame::default();
        let gpu = |serial| {
            Some(crate::renderer::timing::Sample {
                serial,
                milliseconds: [1.0; 4],
            })
        };
        samples.record(60, cpu, gpu(57), 60);
        samples.record(61, cpu, gpu(59), 61);
        samples.record(62, cpu, gpu(61), 62);
        samples.record(63, cpu, gpu(61), 63);
        samples.record(64, cpu, None, 64);
        samples.record(65, cpu, gpu(63), 65);
        assert_eq!(samples.cpu.len(), 5);
        assert_eq!(samples.gpu, vec![[1.0; 4]; 2]);
        samples.record(1, cpu, gpu(64), 66);
        samples.record(61, cpu, gpu(65), 67);
        assert_eq!(samples.gpu.len(), 2);
    }

    #[test]
    fn percentiles_include_outliers_without_accepting_invalid_timestamps() {
        let s =
            Distribution::new(
                (1..=100)
                    .map(|n| n as f32)
                    .chain([f32::NAN, -1.0, f32::INFINITY]),
            );
        assert_eq!(
            (s.samples, s.mean_ms, s.p50_ms, s.p95_ms, s.p99_ms),
            (100, 50.5, 50.0, 95.0, 99.0)
        );
        let single = Distribution::new([17.0].into_iter());
        assert_eq!(single.p99_ms, 17.0);
        assert_eq!(Distribution::new(std::iter::empty()).samples, 0);
    }
}
