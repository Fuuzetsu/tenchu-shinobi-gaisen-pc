use super::{
    campaign::{Action, Content, Playback},
    preferences,
    presentation::ContentDrawing,
    transport::{Input as PlaybackInput, Transport},
};
use crate::{
    audio::output::{Command, Output},
    controls::Controls,
    performance::{CpuFrame, Performance},
    renderer::Renderer,
    settings::{Graphics, Settings, Store},
    source::DiscSource,
};
use std::{sync::Arc, time::Instant};
use tenchu_assets::Result;

mod benchmark;
mod rewind;
mod state;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct View {
    window: Arc<Window>,
    adapter: wgpu::AdapterInfo,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    drawing: ContentDrawing,
    generation: u64,
    graphics: Graphics,
    context: egui::Context,
    state: egui_winit::State,
    overlay: egui_wgpu::Renderer,
    free: Vec<egui::TextureId>,
    performance: Performance,
}

impl View {
    async fn new(
        window: Arc<Window>,
        session: &Playback<DiscSource>,
        graphics: &Graphics,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(&Default::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or("No graphics adapter")?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("No surface configuration")?;
        if let Some(format) = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|f| !f.is_srgb())
        {
            config.format = format;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        let mut renderer = Renderer::new(
            &adapter,
            config.format,
            [config.width, config.height],
            session.vram(),
        )
        .await?;
        renderer.configure(graphics);
        let drawing = ContentDrawing::new(&session.content, graphics, &mut renderer)?;
        surface.configure(&renderer.device, &config);
        let context = egui::Context::default();
        crate::fonts::install(&context);
        let state = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        let mut overlay = egui_wgpu::Renderer::new(&renderer.device, config.format, None, 1, false);
        crate::frontend::gpu::register(&mut overlay, config.format);
        Ok(Self {
            window,
            adapter: adapter.get_info(),
            surface,
            config,
            renderer,
            drawing,
            generation: session.generation,
            graphics: graphics.clone(),
            context,
            state,
            overlay,
            free: Vec::new(),
            performance: Default::default(),
        })
    }

    fn draw(
        &mut self,
        session: &mut Playback<DiscSource>,
        menu: &mut preferences::Menu,
        state: &mut preferences::State<'_>,
        error: Option<&str>,
        fps: f32,
        timing: &mut CpuFrame,
    ) -> Result<preferences::Action> {
        let phase = Instant::now();
        let acquired = self.surface.get_current_texture();
        timing.draw.wait += phase.elapsed().as_secs_f32() * 1000.0;
        let frame = match acquired {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.renderer.device, &self.config);
                return Ok(preferences::Action::default());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(preferences::Action::default()),
            Err(error) => return Err(error.to_string()),
        };
        let screenshot = std::mem::take(&mut menu.screenshot);
        let phase = Instant::now();
        let target = frame.texture.create_view(&Default::default());
        self.drawing
            .draw(&session.content, &target, &mut self.renderer)?;
        timing.draw.submit += phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        let mut action = preferences::Action::default();
        let output = self
            .context
            .run(self.state.take_egui_input(&self.window), |ctx| {
                menu.title_tools(ctx, matches!(session.content, Content::Title(_)), false);
                self.drawing.paint(&session.content, ctx);
                if !menu.panel.open && !menu.modal_open() {
                    session.action = session
                        .content
                        .controls(ctx, state.settings.circle_confirm, false)
                        .or(session.action);
                    session.content.inspect(
                        ctx,
                        &session.run,
                        &mut menu.diagnostics,
                        state.paused,
                        error,
                        menu.tools.debug.photo_mode,
                    );
                }
                menu.cutscene_controls(ctx, session.content.cutscene());
                if !menu.panel.open && !menu.modal_open() {
                    session.death_controls(ctx, menu.tools.debug.photo_mode);
                }
                if state.settings.show_performance && !menu.tools.debug.photo_mode {
                    self.performance.show(
                        ctx,
                        &mut state.settings.show_performance,
                        menu.panel.open,
                        self.renderer.timings.is_some(),
                        false,
                    );
                }
                action = menu.show(ctx, state);
                action.randomizer = menu.show_randomizer(ctx, session);
                if state.settings.show_fps && !menu.tools.debug.photo_mode {
                    egui::Area::new("native-fps".into())
                        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
                        .show(ctx, |ui| {
                            ui.label(format!("{fps:.0} FPS"));
                        });
                }
            });
        self.state
            .handle_platform_output(&self.window, output.platform_output);
        let jobs = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        self.renderer.update_overlay_textures(
            &mut self.overlay,
            &mut self.free,
            output.textures_delta,
        );
        timing.overlay += phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        self.renderer.draw_overlay(
            &target,
            &mut self.overlay,
            &jobs,
            &egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: output.pixels_per_point,
            },
        );
        if screenshot {
            let result = (|| {
                let path = crate::capture::screenshot_path(&format!(
                    "tenchu2-{}-{}",
                    session.run.sequence,
                    session.content.tick()
                ))?;
                crate::capture::render_to_png(
                    &mut self.renderer,
                    [self.config.width, self.config.height],
                    self.config.format,
                    &path,
                    |renderer, target| {
                        self.drawing.draw(&session.content, target, renderer)?;
                        renderer.draw_overlay(
                            target,
                            &mut self.overlay,
                            &jobs,
                            &egui_wgpu::ScreenDescriptor {
                                size_in_pixels: [self.config.width, self.config.height],
                                pixels_per_point: output.pixels_per_point,
                            },
                        );
                        Ok(())
                    },
                )
            })();
            menu.screenshot_result(result);
        }
        timing.draw.submit += phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        self.window.pre_present_notify();
        frame.present();
        timing.draw.wait += phase.elapsed().as_secs_f32() * 1000.0;
        Ok(action)
    }
}

struct Application {
    session: Playback<DiscSource>,
    settings: Settings,
    store: Store,
    view: Option<View>,
    controls: Controls,
    audio: Option<Output>,
    transport: Transport,
    rewind: super::rewind::Rewind,
    last: Instant,
    paused: bool,
    focused: bool,
    menu: preferences::Menu,
    presets: crate::settings::library::Manager,
    fps: f32,
    fps_time: f32,
    fps_frames: u32,
    error: Option<String>,
    quit: bool,
    gallery: Option<super::models::Gallery>,
    gallery_error: Option<String>,
    report_save: crate::diagnostics::Save,
}

impl Application {
    fn toggle_randomizer(&mut self) -> Result<()> {
        let saved = crate::storage::UserFile::LastDisc.path()?;
        let gaisen = crate::startup::remembered_path(&saved).filter(|p| p.exists());
        self.menu
            .toggle_randomizer(&mut self.session, gaisen.as_deref())?;
        self.controls.clear();
        self.controls.close_editor();
        self.transport.discard_backlog();
        self.last = Instant::now();
        Ok(())
    }

    fn configure_gameplay(&mut self) -> Result<()> {
        use tenchu_core::game::sheathing::UnarmedStyle;
        if let Content::Gameplay(session) = &mut self.session.content
            && self.settings.sheathing.unarmed_style == UnarmedStyle::Onikage
            && session.needs_unarmed(self.settings.sheathing)
        {
            let mut load = || {
                let saved = crate::storage::UserFile::LastDisc.path()?;
                let path = crate::startup::remembered_path(&saved)
                    .ok_or("Open Gaisen once with play to connect its disc for Onikage attacks. Tatsumaru uses the current T2 disc.")?;
                let assets = crate::startup::open(&path)?;
                session.configure_onikage(&assets, self.settings.sheathing)
            };
            if let Err(error) = load() {
                self.settings.sheathing.unarmed = false;
                self.store.write(&self.settings)?;
                self.menu.panel.notice = error;
            }
        }
        self.session.configure(&self.settings)
    }

    fn step(&mut self) -> Result<()> {
        if self.report_save.active() {
            return Ok(());
        }
        self.rewind.resume()?;
        self.controls.stop_rumble();
        self.configure_gameplay()?;
        let buttons = self.menu.tools.input(
            &mut self.session.content,
            &mut self.controls,
            &self.settings,
            self.focused && !self.menu.panel.open && !self.menu.modal_open(),
        );
        let mut view = self
            .view
            .as_mut()
            .filter(|v| v.generation == self.session.generation && !self.menu.actor_assets_changed);
        let graphics = self.settings.rendering().graphics.clone();
        let update = self.transport.frame_step(
            &mut self.session,
            buttons,
            self.audio.as_ref(),
            |playback, observed| {
                self.rewind.observed(playback, observed, &self.menu.tools);
                if !observed.replaced
                    && let Some(view) = &mut view
                {
                    view.drawing.tick(&playback.content, &graphics)?;
                    if let Some(error) = self.rewind.record(
                        playback,
                        &view.drawing,
                        &mut self.settings,
                        &self.menu.tools,
                    ) {
                        self.menu.notice(error);
                    }
                }
                Ok(())
            },
        )?;
        self.menu
            .tools
            .sync(&self.session.content, self.session.generation);
        self.quit |= update.quit;
        if update.settings {
            self.menu.panel.open = true;
            self.controls.clear();
        }
        if update.changed {
            self.last = Instant::now();
        }
        Ok(())
    }

    fn redraw(&mut self) -> Result<()> {
        if let Some(result) = self.report_save.poll() {
            match result {
                Ok(Some(notice)) => self.menu.panel.notice = notice,
                Ok(None) => {}
                Err(error) => {
                    self.menu.panel.notice = format!("Could not save diagnostic report: {error}")
                }
            }
            self.controls.clear();
            self.transport.discard_backlog();
            self.last = Instant::now();
        }
        if self.benchmark_redraw()? {
            return Ok(());
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let polled = self.controls.poll(self.focused);
        if polled.disconnected {
            self.paused = true;
        }
        self.menu
            .controller(polled.menu_pressed, self.settings.circle_confirm);
        if !self.menu.panel.open
            && !self.menu.modal_open()
            && self
                .session
                .controller_pause(polled.menu_pressed, self.settings.circle_confirm)
        {
            self.paused = !self.paused;
        }
        if !self.paused && self.rewind.preview {
            self.rewind.resume()?;
        }
        if !self.report_save.active() {
            self.advance_rewind(elapsed);
        }
        let paused = self.report_save.active()
            || self.paused
            || !self.focused && self.settings.pause_on_defocus
            || self.error.is_some()
            || self.menu.modal_open()
            || self.menu.panel.open && self.settings.pause_in_menu
            || self.controls.editor.open
            || self.presets.open;
        if !self.rewind.preview {
            self.configure_gameplay()?;
        }
        let input = PlaybackInput {
            buttons: if self.rewind.preview {
                0
            } else {
                self.menu.tools.input(
                    &mut self.session.content,
                    &mut self.controls,
                    &self.settings,
                    self.focused && !self.menu.panel.open && !self.menu.modal_open(),
                )
            },
            cutscene_speed: self.settings.cutscene_speed,
            simulation_speed: self.menu.tools.debug.speed,
            fast_forward: self.controls.held(KeyCode::Backspace) || self.menu.fast_forward,
            paused,
        };
        self.session
            .content
            .interpolate(self.settings.rendering().graphics.interpolation)?;
        let mut view = self
            .view
            .as_mut()
            .filter(|v| v.generation == self.session.generation && !self.menu.actor_assets_changed);
        let graphics = self.settings.rendering().graphics.clone();
        let feedback_enabled = self.focused
            && !input.paused
            && !input.pacing(&self.session.content).1
            && self.settings.frontend.rumble;
        let update = if self.rewind.preview {
            self.transport
                .preview(&mut self.session, self.audio.as_ref());
            super::transport::Update::default()
        } else {
            self.transport.advance(
                &mut self.session,
                elapsed,
                input,
                self.audio.as_ref(),
                |playback, observed| {
                    let content = &playback.content;
                    self.rewind.observed(playback, observed, &self.menu.tools);
                    self.controls.native_feedback(
                        content.feedback(),
                        feedback_enabled,
                        &self.settings.vibration,
                    );
                    if !observed.replaced
                        && let Some(view) = &mut view
                    {
                        view.drawing.tick(content, &graphics)?;
                        if let Some(error) = self.rewind.record(
                            playback,
                            &view.drawing,
                            &mut self.settings,
                            &self.menu.tools,
                        ) {
                            self.menu.notice(error);
                        }
                    }
                    Ok(())
                },
            )?
        };
        self.controls.native_feedback(
            self.session.content.feedback(),
            feedback_enabled && !self.session.death.frozen(),
            &self.settings.vibration,
        );
        self.menu
            .tools
            .sync(&self.session.content, self.session.generation);
        self.quit |= update.quit;
        if update.settings {
            self.menu.panel.open = true;
            self.controls.clear();
        }
        if update.changed {
            self.last = Instant::now();
        }
        if let Some(audio) = &self.audio {
            audio.send(Command::Pause {
                paused: paused || self.session.death.frozen(),
                music_while_paused: self.settings.music_while_paused
                    && !self.rewind.preview
                    && !self.session.content.cutscene(),
            });
        }
        self.fps_time += elapsed.as_secs_f32();
        self.fps_frames += 1;
        if self.fps_time >= 0.5 {
            self.fps = self.fps_frames as f32 / self.fps_time;
            self.fps_frames = 0;
            self.fps_time = 0.0;
        }
        let rewind_cpu = std::mem::take(&mut self.rewind.cpu).as_secs_f32() * 1000.0;
        let mut timing = CpuFrame {
            simulation: (now.elapsed().as_secs_f32() * 1000.0 - rewind_cpu).max(0.0),
            rewind: rewind_cpu,
            ..Default::default()
        };
        let prepare = Instant::now();
        self.menu.tools.advance(
            &mut self.controls,
            elapsed.as_secs_f32(),
            self.focused && !self.menu.panel.open && !self.menu.modal_open(),
            &self.settings.navigation,
        );
        let randomized = self.session.randomized().is_some();
        let wants_models = !randomized
            && ((self.menu.panel.open && self.menu.panel.page.uses_models())
                || !self
                    .session
                    .content
                    .current_model(self.session.run.hero, &self.settings));
        if wants_models && self.gallery.is_none() && self.gallery_error.is_none() {
            match super::models::Gallery::remembered(&self.settings.model_directory) {
                Ok(mut gallery) => {
                    if self
                        .session
                        .content
                        .current_model(self.session.run.hero, &self.settings)
                        && let Some(pack) = self.session.content.embedded_bodies()
                    {
                        gallery.bodies = pack;
                    }
                    self.gallery = Some(gallery);
                }
                Err(error) => {
                    self.menu.panel.notice = error.clone();
                    self.gallery_error = Some(error);
                }
            }
        }
        let mut models_changed = std::mem::take(&mut self.menu.actor_assets_changed);
        if randomized {
            let bodies = self
                .gallery
                .as_ref()
                .map_or_else(Arc::default, |g| g.bodies.clone());
            models_changed |= self.session.randomizer_models(&self.settings, &bodies)?;
        }
        if let (Content::Gameplay(session), Some(gallery)) =
            (&mut self.session.content, &mut self.gallery)
            && !randomized
        {
            match session.models.sync(
                &mut session.world,
                &mut session.assets,
                &mut gallery.game,
                &gallery.library,
                &gallery.bodies,
                &self.settings,
            ) {
                Ok(changed) => {
                    models_changed |= changed;
                    if changed {
                        session.reset_interpolation()?;
                    }
                }
                Err(error) => self.menu.panel.notice = error,
            }
        }
        if let (Content::Story { story, .. }, Some(gallery)) =
            (&mut self.session.content, &self.gallery)
            && !randomized
        {
            match story.models.sync(
                &story.resources,
                self.session.run.hero,
                &gallery.library,
                &gallery.bodies,
                &self.settings,
            ) {
                Ok(changed) => models_changed |= changed,
                Err(error) => self.menu.panel.notice = error,
            }
        }
        let mut save_diagnostics = false;
        let mut export_unarmed = false;
        let mut state_action = None;
        let mut rewind_action = None;
        let mut open_randomizer = false;
        let mut randomizer_action = None;
        let mut campaign_action = None;
        let previous = self.settings.clone();
        let graphics = preferences::graphics(&self.settings);
        if let Some(view) = &mut self.view {
            view.renderer.profiling = self.settings.show_performance;
            if view.window.fullscreen().is_some() != self.settings.fullscreen {
                view.window.set_fullscreen(
                    self.settings
                        .fullscreen
                        .then_some(winit::window::Fullscreen::Borderless(None)),
                );
            }
            let present_mode = if self.settings.vsync {
                wgpu::PresentMode::AutoVsync
            } else {
                wgpu::PresentMode::AutoNoVsync
            };
            if view.config.present_mode != present_mode {
                view.config.present_mode = present_mode;
                view.surface.configure(&view.renderer.device, &view.config);
            }
            if view.generation != self.session.generation
                || view.graphics != graphics
                || models_changed
            {
                view.renderer.configure(&graphics);
                if view.generation != self.session.generation
                    || crate::visuals::models::Options::from(&view.graphics)
                        != crate::visuals::models::Options::from(&graphics)
                    || models_changed
                {
                    view.drawing.replace(
                        &self.session.content,
                        self.session.vram(),
                        &graphics,
                        &mut view.renderer,
                        view.generation == self.session.generation,
                    )?;
                }
                view.generation = self.session.generation;
                view.graphics.clone_from(&graphics);
                self.last = Instant::now();
            }
            view.drawing.interpolate(if graphics.interpolation {
                self.transport.interpolation(&self.session, input)
            } else {
                1.0
            });
            self.menu.tools.update_camera(
                &self.session.content,
                &mut view.drawing,
                &self.settings.navigation,
                elapsed.as_secs_f32(),
                &mut view.renderer,
            );
            view.drawing.update(
                &self.session.content,
                &mut view.renderer,
                &self.menu.tools.debug,
                &self.settings,
            )?;
            if let Some(error) = self.rewind.record(
                &self.session,
                &view.drawing,
                &mut self.settings,
                &self.menu.tools,
            ) {
                self.menu.notice(error);
            }
            self.rewind.sync(&mut self.menu.tools);
            let original_model = if let Content::Gameplay(session) = &self.session.content {
                session
                    .assets
                    .resources
                    .registry
                    .actor(session.world.actors[session.world.player].actor.kind)
                    .map(|a| format!("tenchu2-jp:{}", a.model))
            } else {
                None
            };
            let lights = view.drawing.light_count(&view.renderer.graphics().lighting);
            let rewind_cpu = std::mem::take(&mut self.rewind.cpu).as_secs_f32() * 1000.0;
            timing.rewind += rewind_cpu;
            timing.prepare = (prepare.elapsed().as_secs_f32() * 1000.0 - rewind_cpu).max(0.0);
            let action = view.draw(
                &mut self.session,
                &mut self.menu,
                &mut preferences::State {
                    settings: &mut self.settings,
                    paused: &mut self.paused,
                    controls: &mut self.controls,
                    presets: &mut self.presets,
                    lights,
                    samples: &view.renderer.sample_counts.clone(),
                    models: self
                        .gallery
                        .as_mut()
                        .map(|gallery| gallery.state(original_model)),
                },
                self.error.as_deref(),
                self.fps,
                &mut timing,
            )?;
            let prepare = Instant::now();
            save_diagnostics = action.save_diagnostics;
            export_unarmed = action.export_unarmed;
            state_action = action.state;
            rewind_action = action.rewind;
            open_randomizer = action.open_randomizer;
            randomizer_action = action.randomizer;
            campaign_action = action.campaign;
            if let Some(bodies) = action.models
                && let Some(gallery) = &mut self.gallery
            {
                gallery.bodies = bodies;
                gallery.picker.clear();
            }
            if let Some(request) = action.spawn {
                self.menu.apply_spawn(
                    &mut self.session,
                    request,
                    self.gallery.as_ref().map(|g| &g.library),
                );
            }
            if action.reload_artwork {
                match view.drawing.replace(
                    &self.session.content,
                    self.session.vram(),
                    &graphics,
                    &mut view.renderer,
                    true,
                ) {
                    Ok(()) => self.menu.panel.notice = "Artwork reloaded".into(),
                    Err(error) => self.menu.panel.notice = error,
                }
            }
            if action.clear_blood {
                view.drawing.clear_blood();
            }
            if action.clear_weather || action.clear_footprints {
                view.drawing.clear_weather(!action.clear_weather);
            }
            self.menu.apply_tools(
                &mut self.session.content,
                [view.config.width, view.config.height],
            );
            self.menu.preview_audio(&self.session, self.audio.as_ref());
            let (_, accelerated) = input.pacing(&self.session.content);
            view.drawing.presentation_audio(
                &self.session.content,
                &graphics.weather,
                accelerated || self.rewind.preview,
                self.audio.as_ref(),
            )?;
            self.quit |= action.leave;
            timing.prepare += prepare.elapsed().as_secs_f32() * 1000.0;
            if action.step && self.error.is_none() {
                let phase = Instant::now();
                self.step()?;
                let rewind_cpu = std::mem::take(&mut self.rewind.cpu).as_secs_f32() * 1000.0;
                timing.rewind += rewind_cpu;
                timing.simulation += (phase.elapsed().as_secs_f32() * 1000.0 - rewind_cpu).max(0.0);
            }
            if action.select_mission {
                self.session.action = Some(Action::SelectMission);
                self.menu.panel.open = false;
                self.paused = false;
            }
            if !self.menu.panel.open {
                self.controls.close_editor();
                self.presets.open = false;
            }
        }
        if save_diagnostics {
            if let Err(error) = self.save_diagnostics(paused) {
                self.menu.panel.notice = error;
            }
            self.last = Instant::now();
        }
        if open_randomizer {
            self.toggle_randomizer()?;
        }
        if let Some(id) = campaign_action {
            match self.menu.select_campaign(&mut self.session, id) {
                Ok(()) => {
                    self.error = None;
                    self.rewind.clear();
                    self.controls.clear();
                    self.transport
                        .restored(&mut self.session, self.audio.as_ref());
                    self.paused = false;
                    self.last = Instant::now();
                }
                Err(error) => self.menu.panel.notice = error,
            }
        }
        if let Some(event) = randomizer_action {
            if let Err(error) = self.menu.randomizer_action(&mut self.session, event) {
                self.menu.panel.notice = error;
            } else {
                self.error = None;
                self.rewind.clear();
                self.paused = false;
            }
            self.controls.clear();
            self.transport.discard_backlog();
            self.last = Instant::now();
        }
        if export_unarmed {
            self.menu.export_unarmed(&self.session);
            self.transport.discard_backlog();
            self.last = Instant::now();
        }
        if previous != self.settings {
            if let Some(audio) = &self.audio {
                audio.send(Command::Volume(self.settings.volume));
            }
            self.store.write(&self.settings)?;
        }
        if let Some(action) = rewind_action {
            self.rewind_action(action);
        }
        if let Some(save) = state_action {
            self.state_action(save);
        }
        timing.rewind += std::mem::take(&mut self.rewind.cpu).as_secs_f32() * 1000.0;
        if let Some(view) = &mut self.view {
            view.performance.record_at(
                now,
                timing,
                view.renderer.timings.as_ref().and_then(|t| t.latest),
            );
        }
        Ok(())
    }

    fn diagnostic_report(&self, suspended: bool) -> Result<Vec<u8>> {
        let mut report = super::diagnostics::report(
            &self.session,
            &self.transport,
            &self.menu,
            &self.settings,
            &self.controls,
        );
        report["runtime"]["paused"] = serde_json::json!(self.paused);
        report["runtime"]["suspended"] = serde_json::json!(suspended);
        report["runtime"]["focused"] = serde_json::json!(self.focused);
        report["runtime"]["error"] = serde_json::json!(self.error);
        report["runtime"]["audio_active"] = serde_json::json!(self.audio.is_some());
        if let Some(view) = &self.view {
            let mut renderer = crate::diagnostics::renderer(
                &view.adapter,
                &view.window,
                &view.config,
                &view.renderer,
            );
            renderer["lights"] =
                serde_json::json!(view.drawing.light_count(&view.graphics.lighting));
            renderer["fps"] = serde_json::json!(self.fps);
            report["runtime"]["performance"] = view.performance.diagnostic_snapshot();
            report["renderer"] = renderer;
            report["effective_graphics"] = serde_json::json!(view.renderer.graphics());
        }
        crate::diagnostics::finish(report, &self.settings, self.store.path.parent())
    }

    fn save_diagnostics(&mut self, suspended: bool) -> Result<()> {
        let bytes = self.diagnostic_report(suspended)?;
        self.report_save
            .start(super::diagnostics::filename(&self.session), bytes);
        self.controls.clear();
        self.transport.discard_backlog();
        Ok(())
    }

    fn fail(&mut self, error: String) {
        self.controls.stop_rumble();
        self.session.content.clear_feedback();
        eprintln!(
            "Native JP sequence {}, tick {}: {error}",
            self.session.run.sequence,
            self.session.content.tick()
        );
        self.error = Some(error);
        self.paused = true;
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let size: winit::dpi::Size = self
            .menu
            .benchmarks
            .ui
            .pending
            .as_ref()
            .and_then(|o| o.size)
            .map_or_else(
                || winit::dpi::LogicalSize::new(1280, 720).into(),
                |s| winit::dpi::PhysicalSize::new(s[0], s[1]).into(),
            );
        let result = events
            .create_window(
                Window::default_attributes()
                    .with_title("Tenchu 2 JP — native integration preview")
                    .with_inner_size(size),
            )
            .map_err(|e| e.to_string())
            .and_then(|window| {
                pollster::block_on(View::new(
                    Arc::new(window),
                    &self.session,
                    &preferences::graphics(&self.settings),
                ))
            });
        match result {
            Ok(view) => {
                if let Some(audio) = &self.audio {
                    self.session.content.start_audio(audio);
                }
                self.view = Some(view);
                self.last = Instant::now();
            }
            Err(error) => {
                self.fail(error);
                events.exit();
            }
        }
    }

    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(view) = &mut self.view else {
            return;
        };
        if self.menu.benchmarks.event(&view.window, &event) {
            return;
        }
        let consumed = view.state.on_window_event(&view.window, &event).consumed;
        self.menu.tools.event(
            &event,
            self.focused
                && !self.menu.panel.open
                && !self.menu.diagnostics
                && !consumed
                && !view.context.is_pointer_over_area(),
            &self.settings.navigation,
        );
        match event {
            WindowEvent::CloseRequested => {
                self.controls.stop_rumble();
                events.exit();
            }
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                view.config.width = size.width;
                view.config.height = size.height;
                view.renderer.resize([size.width, size.height]);
                view.surface.configure(&view.renderer.device, &view.config);
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                if !focused {
                    self.controls.clear();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if self.controls.editor.open {
                        if pressed {
                            self.controls.editor_key(code);
                        }
                        return;
                    }
                    if pressed && !event.repeat {
                        match code {
                            KeyCode::Backspace
                                if !self.menu.panel.open
                                    && !self.menu.modal_open()
                                    && self.session.content.cutscene() =>
                            {
                                self.controls.key(code, true);
                            }
                            KeyCode::Escape
                                if !self.menu.panel.open
                                    && (self.menu.tools.debug.click_teleport
                                        || self.menu.tools.debug.click_kill
                                        || self.menu.tools.debug.teleport_armed
                                        || self.menu.tools.debug.pick_actor_position) =>
                            {
                                self.menu.tools.debug.click_teleport = false;
                                self.menu.tools.debug.click_kill = false;
                                self.menu.tools.debug.teleport_armed = false;
                                self.menu.tools.debug.pick_actor_position = false;
                            }
                            KeyCode::Escape
                                if self.menu.panel.open
                                    || self.menu.diagnostics
                                    || self.menu.modal_open() =>
                            {
                                self.menu.back(&mut self.controls, &mut self.presets);
                            }
                            KeyCode::Escape
                                if self.menu.tools.debug.camera.is_some()
                                    || self.menu.tools.debug.camera_actor.is_some() =>
                            {
                                self.menu.tools.debug.return_camera();
                                self.controls.clear();
                            }
                            KeyCode::Escape if self.session.back_menu() => self.controls.clear(),
                            KeyCode::Escape => events.exit(),
                            KeyCode::F6 => events.exit(),
                            KeyCode::F7 => {
                                if let Err(error) = self.toggle_randomizer() {
                                    self.menu.panel.notice = error;
                                }
                            }
                            KeyCode::F1 => {
                                self.menu.benchmarks.ui.open = false;
                                if let Some(editor) = &mut self.menu.randomizer {
                                    editor.open = false;
                                }
                                self.menu.panel.open = !self.menu.panel.open;
                                self.controls.clear();
                            }
                            KeyCode::F2 | KeyCode::F10 if !self.menu.panel.open => {
                                let item = if code == KeyCode::F2 {
                                    crate::settings::menu::Item::FreeCamera
                                } else {
                                    crate::settings::menu::Item::PhotoMode
                                };
                                if let Err(error) = self.menu.tools.change(item, 1) {
                                    self.menu.panel.notice = error;
                                }
                                self.controls.clear();
                            }
                            KeyCode::F8 => {
                                self.settings.retro = !self.settings.retro;
                                if let Err(error) = self.store.write(&self.settings) {
                                    self.fail(error);
                                }
                            }
                            KeyCode::F9 => self.menu.request_screenshot(),
                            KeyCode::F12
                                if self.controls.held(KeyCode::ControlLeft)
                                    || self.controls.held(KeyCode::ControlRight) =>
                            {
                                self.rewind_action(super::rewind::Action::Toggle);
                            }
                            KeyCode::F11 | KeyCode::F12 => self.state_action(code == KeyCode::F11),
                            KeyCode::F3 => {
                                if self.rewind.preview {
                                    self.rewind_action(super::rewind::Action::Resume);
                                } else {
                                    self.paused = !self.paused;
                                }
                            }
                            KeyCode::F4 if self.paused && self.error.is_none() => {
                                if let Err(error) = self.step() {
                                    self.fail(error);
                                }
                            }
                            KeyCode::Enter
                                if !self.menu.panel.open
                                    && !self.menu.modal_open()
                                    && !consumed =>
                            {
                                self.session.confirm();
                            }
                            _ => {}
                        }
                    }
                    if !consumed && !self.menu.panel.open && !self.menu.modal_open() || !pressed {
                        self.controls.key(code, pressed);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.fail(error);
                }
                if self.quit {
                    events.exit();
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.controls.stop_rumble();
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(view) = &self.view {
            view.window.request_redraw();
        }
    }
}

pub fn run(session: Playback<DiscSource>, settings: Settings) -> Result<()> {
    run_options(session, settings, None)
}

pub(super) fn run_benchmark(
    session: Playback<DiscSource>,
    settings: Settings,
    options: crate::window::benchmark::Options,
) -> Result<()> {
    run_options(session, settings, Some(options))
}

fn run_options(
    session: Playback<DiscSource>,
    settings: Settings,
    benchmark: Option<crate::window::benchmark::Options>,
) -> Result<()> {
    if benchmark.is_none() {
        crate::online::updates::UPDATES.start(settings.check_for_updates);
    }
    let audio = Output::new()
        .map_err(|error| eprintln!("Audio unavailable: {error}"))
        .ok();
    if let Some(audio) = &audio {
        audio.send(Command::Volume(settings.volume));
    }
    let mut menu = preferences::Menu::new(&settings);
    menu.benchmarks.ui.pending = benchmark;
    let mut application = Application {
        session,
        settings,
        store: Store::new()?,
        view: None,
        controls: Controls::new(),
        audio,
        transport: Transport::default(),
        rewind: Default::default(),
        last: Instant::now(),
        paused: false,
        focused: true,
        menu,
        presets: crate::settings::library::Manager::new(crate::storage::UserFile::Presets.path()?),
        fps: 0.0,
        fps_time: 0.0,
        fps_frames: 0,
        error: None,
        quit: false,
        gallery: None,
        gallery_error: None,
        report_save: Default::default(),
    };
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    events
        .run_app(&mut application)
        .map_err(|e| e.to_string())?;
    application.error.map_or(Ok(()), Err)
}
