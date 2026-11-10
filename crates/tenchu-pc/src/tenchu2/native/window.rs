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
                self.drawing.paint(&session.content, ctx);
                if !menu.panel.open {
                    session.action = session
                        .content
                        .inspect(
                            ctx,
                            &session.run,
                            &mut menu.diagnostics,
                            state.paused,
                            error,
                            menu.tools.debug.photo_mode,
                        )
                        .or(session.action);
                }
                menu.cutscene_controls(ctx, session.content.cutscene());
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
}

impl Application {
    fn step(&mut self) -> Result<()> {
        let buttons = self.menu.tools.input(
            &mut self.session.content,
            &mut self.controls,
            &self.settings.navigation,
            self.focused && !self.menu.panel.open,
        );
        let mut view = self
            .view
            .as_mut()
            .filter(|v| v.generation == self.session.generation);
        let motion = &self.settings.rendering().graphics.motion;
        let update = self.transport.frame_step(
            &mut self.session,
            buttons,
            self.audio.as_ref(),
            |content| {
                if let Some(view) = &mut view {
                    view.drawing.tick(content, motion)?;
                }
                Ok(())
            },
        )?;
        self.menu
            .tools
            .sync(&self.session.content, self.session.generation);
        self.quit |= update.quit;
        if update.changed {
            self.last = Instant::now();
        }
        Ok(())
    }

    fn redraw(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let polled = self.controls.poll(self.focused);
        if polled.disconnected {
            self.paused = true;
        }
        self.menu.controller(polled.menu_pressed);
        if matches!(self.session.content, Content::Gameplay(_))
            && polled.menu_pressed & tenchu_core::input::START != 0
            && !self.menu.panel.open
        {
            self.paused = !self.paused;
        }
        let paused = self.paused
            || !self.focused && self.settings.pause_on_defocus
            || self.error.is_some()
            || self.menu.panel.open && self.settings.pause_in_menu
            || self.controls.editor.open
            || self.presets.open;
        let input = PlaybackInput {
            buttons: self.menu.tools.input(
                &mut self.session.content,
                &mut self.controls,
                &self.settings.navigation,
                self.focused && !self.menu.panel.open,
            ),
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
            .filter(|v| v.generation == self.session.generation);
        let motion = &self.settings.rendering().graphics.motion;
        let update = self.transport.advance(
            &mut self.session,
            elapsed,
            input,
            self.audio.as_ref(),
            |content| {
                if let Some(view) = &mut view {
                    view.drawing.tick(content, motion)?;
                }
                Ok(())
            },
        )?;
        self.menu
            .tools
            .sync(&self.session.content, self.session.generation);
        self.quit |= update.quit;
        if update.changed {
            self.last = Instant::now();
        }
        if let Some(audio) = &self.audio {
            audio.send(Command::Pause {
                paused,
                music_while_paused: self.settings.music_while_paused
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
        let mut timing = CpuFrame {
            simulation: now.elapsed().as_secs_f32() * 1000.0,
            ..Default::default()
        };
        let prepare = Instant::now();
        self.menu.tools.advance(
            &mut self.controls,
            elapsed.as_secs_f32(),
            self.focused && !self.menu.panel.open,
            &self.settings.navigation,
        );
        let wants_models = (self.menu.panel.open && self.menu.panel.page == 10)
            || !self.settings.player_model.is_empty()
            || self.settings.moveset_id().is_some();
        if wants_models && self.gallery.is_none() && self.gallery_error.is_none() {
            match super::models::Gallery::remembered(&self.settings.model_directory) {
                Ok(gallery) => self.gallery = Some(gallery),
                Err(error) => {
                    self.menu.panel.notice = error.clone();
                    self.gallery_error = Some(error);
                }
            }
        }
        let mut models_changed = false;
        if let (Content::Gameplay(session), Some(gallery)) =
            (&mut self.session.content, &mut self.gallery)
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
                    models_changed = changed;
                    if changed {
                        session.reset_interpolation()?;
                    }
                }
                Err(error) => self.menu.panel.notice = error,
            }
        }
        let mut save_diagnostics = false;
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
                self.transport.interpolation(&self.session.content, input)
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
                self.settings.show_hud,
            )?;
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
            timing.prepare = prepare.elapsed().as_secs_f32() * 1000.0;
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
            if let Some(bodies) = action.models
                && let Some(gallery) = &mut self.gallery
            {
                gallery.bodies = bodies;
                gallery.picker.clear();
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
            if let Some(audio) = &self.audio {
                let (ambience, thunder) = view.drawing.weather_audio(&graphics.weather);
                let (_, accelerated) = input.pacing(&self.session.content);
                audio.send(Command::Ambience(if accelerated {
                    Default::default()
                } else {
                    ambience
                }));
                audio.send(Command::Thunder(if accelerated { None } else { thunder }));
            }
            self.quit |= action.leave;
            timing.prepare += prepare.elapsed().as_secs_f32() * 1000.0;
            if action.step && self.error.is_none() {
                let phase = Instant::now();
                self.step()?;
                timing.simulation += phase.elapsed().as_secs_f32() * 1000.0;
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
        if previous != self.settings {
            if let Some(audio) = &self.audio {
                audio.send(Command::Volume(self.settings.volume));
            }
            self.store.write(&self.settings)?;
        }
        if let Some(view) = &mut self.view {
            view.performance.record_at(
                now,
                timing,
                view.renderer.timings.as_ref().and_then(|t| t.latest),
            );
        }
        Ok(())
    }

    fn save_diagnostics(&mut self, suspended: bool) -> Result<()> {
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
        let bytes = crate::diagnostics::finish(report, &self.settings, self.store.path.parent())?;
        if let Some(notice) =
            crate::diagnostics::save(&super::diagnostics::filename(&self.session), &bytes)?
        {
            self.menu.panel.notice = notice;
        }
        Ok(())
    }

    fn fail(&mut self, error: String) {
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
        let result = events
            .create_window(
                Window::default_attributes()
                    .with_title("Tenchu 2 JP — native integration preview")
                    .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
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
        self.menu.tools.event(
            &event,
            self.focused && !self.menu.panel.open,
            &self.settings.navigation,
        );
        let consumed = view.state.on_window_event(&view.window, &event).consumed;
        match event {
            WindowEvent::CloseRequested => events.exit(),
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
                            KeyCode::Escape
                                if !self.menu.panel.open
                                    && (self.menu.tools.debug.click_teleport
                                        || self.menu.tools.debug.teleport_armed) =>
                            {
                                self.menu.tools.debug.click_teleport = false;
                                self.menu.tools.debug.teleport_armed = false;
                            }
                            KeyCode::Escape if self.menu.panel.open || self.menu.diagnostics => {
                                self.menu.back(&mut self.controls, &mut self.presets);
                            }
                            KeyCode::Escape
                                if self.menu.tools.debug.camera.is_some()
                                    || self.menu.tools.debug.camera_actor.is_some() =>
                            {
                                self.menu.tools.debug.return_camera();
                                self.controls.clear();
                            }
                            KeyCode::Escape => events.exit(),
                            KeyCode::F6 => events.exit(),
                            KeyCode::F1 => {
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
                            KeyCode::F3 => self.paused = !self.paused,
                            KeyCode::F4 if self.paused && self.error.is_none() => {
                                if let Err(error) = self.step() {
                                    self.fail(error);
                                }
                            }
                            KeyCode::Enter if !self.menu.panel.open && !consumed => {
                                self.session.action = match self.session.content {
                                    Content::MissionSelection(_)
                                    | Content::Prologue(_)
                                    | Content::Equipment(_)
                                    | Content::Results(_) => Some(Action::Continue),
                                    Content::Failed | Content::Complete => Some(Action::Retry),
                                    _ => None,
                                }
                            }
                            _ => {}
                        }
                    }
                    if !consumed && !self.menu.panel.open || !pressed {
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

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(view) = &self.view {
            view.window.request_redraw();
        }
    }
}

pub fn run(session: Playback<DiscSource>, settings: Settings) -> Result<()> {
    let audio = Output::new()
        .map_err(|error| eprintln!("Audio unavailable: {error}"))
        .ok();
    if let Some(audio) = &audio {
        audio.send(Command::Volume(settings.volume));
    }
    let menu = preferences::Menu::new(&settings);
    let mut application = Application {
        session,
        settings,
        store: Store::new()?,
        view: None,
        controls: Controls::new(),
        audio,
        transport: Transport::default(),
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
    };
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    events
        .run_app(&mut application)
        .map_err(|e| e.to_string())?;
    application.error.map_or(Ok(()), Err)
}
