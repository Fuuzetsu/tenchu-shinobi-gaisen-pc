use super::{campaign::Playback, demo};
use crate::{
    performance::CpuFrame,
    renderer::Renderer,
    settings::Settings,
    window::{benchmark as shared, overlay::Overlay},
};
use std::time::Instant;
use tenchu_assets::{ReadAt, Result, disc::Disc};
use winit::{
    event::WindowEvent,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub struct Benchmarks {
    pub ui: shared::Benchmarks<()>,
    scene: Option<demo::Scene>,
    clips: Vec<demo::Clip>,
    overlay: Option<Overlay>,
    cancelled: Option<String>,
    last: Option<Instant>,
}

impl Default for Benchmarks {
    fn default() -> Self {
        let mut ui = shared::Benchmarks::new(None);
        ui.description = "Run four fixed camera paths in native T2 JP. Native heroes and original gameplay are fixed; your graphics and artwork settings are measured. The active campaign or randomizer stays paused and untouched.";
        Self {
            ui,
            scene: None,
            clips: Vec::new(),
            overlay: None,
            cancelled: None,
            last: None,
        }
    }
}

pub struct Host<'a> {
    pub window: &'a Window,
    pub surface: &'a wgpu::Surface<'static>,
    pub config: &'a mut wgpu::SurfaceConfiguration,
    pub renderer: &'a mut Renderer,
    pub adapter: &'a wgpu::AdapterInfo,
}

impl Benchmarks {
    pub fn open(&self) -> bool {
        self.ui.open
    }

    pub fn running(&self) -> bool {
        self.ui.run.is_some() || self.ui.pending.is_some()
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if self.ui.show(ctx) {
            self.cancel("Cancelled by user");
        }
    }

    pub fn cancel(&mut self, reason: &str) {
        self.cancelled = Some(reason.into());
    }

    pub fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if !self.running() {
            return false;
        }
        if let Some(overlay) = &mut self.overlay {
            overlay.event(window, event);
        }
        match event {
            WindowEvent::Focused(false) if self.ui.run.as_ref().is_some_and(|r| !r.offscreen()) => {
                self.cancel("Cancelled: window lost focus")
            }
            WindowEvent::Resized(_) if self.ui.run.as_ref().is_some_and(|r| r.frame > 1) => {
                self.cancel("Cancelled: window size changed")
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state.is_pressed()
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape) =>
            {
                self.cancel("Cancelled by user");
            }
            _ => {}
        }
        matches!(
            event,
            WindowEvent::KeyboardInput { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::Touch(_)
        )
    }

    pub fn start(
        &mut self,
        disc: &Disc<impl ReadAt>,
        options: shared::Options,
        current: &Settings,
        renderer: &mut Renderer,
        adapter: &wgpu::AdapterInfo,
    ) -> Result<()> {
        self.clips = demo::catalog()?;
        if self.clips.is_empty() {
            return Err("No native benchmark scenes".into());
        }
        let artwork = if current.graphics.asset_pack {
            "Native T2 JP · replacement artwork enabled, disc fallback"
        } else {
            "Native T2 JP · disc artwork"
        };
        self.ui.run = Some(shared::Run::new(
            (),
            options,
            current,
            renderer,
            adapter,
            renderer.size(),
            renderer.format(),
            artwork.into(),
        ));
        self.ui.open = false;
        self.last = None;
        self.load_scene(disc, renderer)
    }

    fn load_scene(&mut self, disc: &Disc<impl ReadAt>, renderer: &mut Renderer) -> Result<()> {
        let run = self.ui.run.as_mut().unwrap();
        let clip = self.clips[run.scene].clone();
        run.frame = 0;
        run.title.clone_from(&clip.title);
        run.artwork = if run.settings.graphics.asset_pack {
            "Native T2 JP · replacement artwork enabled, disc fallback"
        } else {
            "Native T2 JP · disc artwork"
        }
        .into();
        self.scene = Some(demo::Scene::load(disc, clip, &run.settings, renderer)?);
        self.last = None;
        Ok(())
    }

    pub fn advance(
        &mut self,
        disc: &Disc<impl ReadAt>,
        renderer: &mut Renderer,
        timing: &mut CpuFrame,
    ) -> Result<bool> {
        let run = self
            .ui
            .run
            .as_mut()
            .ok_or("No native benchmark is running")?;
        if run.frame >= run.frames() {
            if !run.finish_scene(&self.scene.as_ref().unwrap().clip.id) {
                return Err("No frames were measured; keep the game visible".into());
            }
            run.scene += 1;
            if run.scene == self.clips.len() {
                if !run.next_trial() {
                    return Ok(false);
                }
                run.scene = 0;
            }
            self.load_scene(disc, renderer)?;
        }
        let run = self.ui.run.as_mut().unwrap();
        let scene = self.scene.as_mut().unwrap();
        run.frame += 1;
        let phase = Instant::now();
        scene.advance(run.frame, &run.settings)?;
        timing.simulation = phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        renderer.profiling = true;
        scene.prepare(run.frame, run.frames(), &run.settings, renderer)?;
        timing.prepare = phase.elapsed().as_secs_f32() * 1000.0;
        Ok(true)
    }

    pub fn finish(&mut self, cancelled: Option<String>) -> Option<shared::Finished<()>> {
        self.scene = None;
        self.overlay = None;
        self.cancelled = None;
        self.last = None;
        self.ui.pending = None;
        self.ui.finish(cancelled)
    }

    pub fn frame(
        &mut self,
        playback: &Playback<impl ReadAt>,
        settings: &Settings,
        host: Host<'_>,
    ) -> Result<Option<shared::Finished<()>>> {
        if let Some(reason) = self.cancelled.take() {
            self.ui.pending = None;
            return Ok(self.finish(Some(reason)));
        }
        if let Some(options) = self.ui.pending.take() {
            self.start(
                &playback.disc,
                options,
                settings,
                host.renderer,
                host.adapter,
            )?;
            self.overlay = Some(Overlay::new(host.window, host.renderer, host.config.format));
            host.config.present_mode = wgpu::PresentMode::AutoNoVsync;
            host.surface.configure(&host.renderer.device, host.config);
        }
        let mut timing = CpuFrame::default();
        let now = Instant::now();
        timing.interval = self.last.replace(now).map_or(0.0, |previous| {
            now.duration_since(previous).as_secs_f32() * 1000.0
        });
        if !self.advance(&playback.disc, host.renderer, &mut timing)? {
            return Ok(self.finish(None));
        }
        let scene = self.scene.as_mut().unwrap();
        let overlay = self.overlay.as_mut().unwrap();
        let phase = Instant::now();
        let mut cancel = false;
        overlay.custom(host.window, host.renderer, |ctx| {
            scene.drawing.paint(&scene.content, ctx);
            cancel = self.ui.show(ctx);
        });
        timing.overlay = phase.elapsed().as_secs_f32() * 1000.0;
        let run = self.ui.run.as_mut().unwrap();
        if let Some(target) = &run.target {
            let phase = Instant::now();
            scene.drawing.draw(&scene.content, target, host.renderer)?;
            overlay.draw(host.renderer, target);
            timing.draw.submit = phase.elapsed().as_secs_f32() * 1000.0;
            let phase = Instant::now();
            host.renderer.device.poll(wgpu::Maintain::Wait);
            timing.draw.wait = phase.elapsed().as_secs_f32() * 1000.0;
        } else {
            let phase = Instant::now();
            let frame = match host.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    return Err("Benchmark interrupted: surface changed".into());
                }
                Err(wgpu::SurfaceError::Timeout) => {
                    return Err("Benchmark interrupted: display timeout".into());
                }
                Err(error) => return Err(error.to_string()),
            };
            timing.draw.wait = phase.elapsed().as_secs_f32() * 1000.0;
            let phase = Instant::now();
            let target = frame.texture.create_view(&Default::default());
            scene.drawing.draw(&scene.content, &target, host.renderer)?;
            overlay.draw(host.renderer, &target);
            frame.present();
            timing.draw.submit = phase.elapsed().as_secs_f32() * 1000.0;
        }
        run.record(timing, host.renderer.timings.as_ref());
        if cancel {
            self.cancel("Cancelled by user");
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests;

pub fn restore(
    playback: &Playback<impl ReadAt>,
    drawing: &mut super::presentation::ContentDrawing,
    renderer: &mut Renderer,
    settings: &mut Settings,
    presets: &mut crate::settings::library::Manager,
    store: &crate::settings::Store,
    finished: Option<shared::Finished<()>>,
) -> Result<()> {
    let optimisation = finished
        .as_ref()
        .filter(|f| !f.options.exit)
        .and_then(|f| f.recommendation.as_ref())
        .map(|auto| {
            presets.save_auto(auto.clone())?;
            auto.visual.apply(settings);
            store.write(settings)
        })
        .transpose();
    let visuals = drawing.capture_state();
    let graphics = super::preferences::graphics(settings);
    renderer.profiling = settings.show_performance;
    renderer.configure(&graphics);
    renderer.reset_vram(playback.vram());
    let restored = if let Some(visuals) = visuals {
        drawing
            .replace(
                &playback.content,
                playback.vram(),
                &graphics,
                renderer,
                false,
            )
            .and_then(|_| drawing.restore_state(visuals, &playback.content, renderer))
    } else {
        Ok(())
    };
    optimisation?;
    if let Some(finished) = finished {
        finished.export?;
    }
    restored
}

#[cfg(not(target_os = "android"))]
pub fn command(disc: Disc<crate::source::DiscSource>, options: shared::Options) -> Result<()> {
    let settings = crate::settings::Store::new()?.load()?;
    let playback = Playback::load(disc, 9, 2, 1, Some([0; 21]), None)?;
    super::window::run_benchmark(playback, settings, options)
}
