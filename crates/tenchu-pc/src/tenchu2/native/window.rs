use super::{
    campaign::{Action, Content, Playback},
    presentation::ContentDrawing,
};
use crate::{
    audio::output::{Command, Output},
    controls::Controls,
    renderer::Renderer,
    settings::Graphics,
    source::DiscSource,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tenchu_assets::Result;

use tenchu_core::clock::GameClock;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct View {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    drawing: ContentDrawing,
    generation: u64,
    context: egui::Context,
    state: egui_winit::State,
    overlay: egui_wgpu::Renderer,
    free: Vec<egui::TextureId>,
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
            surface,
            config,
            renderer,
            drawing,
            generation: session.generation,
            context,
            state,
            overlay,
            free: Vec::new(),
        })
    }

    fn draw(
        &mut self,
        session: &mut Playback<DiscSource>,
        open: &mut bool,
        paused: &mut bool,
        error: Option<&str>,
    ) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.renderer.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(error) => return Err(error.to_string()),
        };
        let target = frame.texture.create_view(&Default::default());
        self.drawing
            .draw(&session.content, &target, &mut self.renderer)?;
        let output = self
            .context
            .run(self.state.take_egui_input(&self.window), |ctx| {
                self.drawing.paint(&session.content, ctx);
                session.action = session
                    .content
                    .inspect(ctx, &session.run, open, paused, error)
                    .or(session.action);
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
        self.renderer.draw_overlay(
            &target,
            &mut self.overlay,
            &jobs,
            &egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: output.pixels_per_point,
            },
        );
        self.window.pre_present_notify();
        frame.present();
        Ok(())
    }
}

struct Application {
    session: Playback<DiscSource>,
    graphics: Graphics,
    view: Option<View>,
    controls: Controls,
    audio: Option<Output>,
    clock: GameClock,
    last: Instant,
    paused: bool,
    focused: bool,
    open: bool,
    error: Option<String>,
    quit: bool,
}

impl Application {
    fn step(&mut self) -> Result<()> {
        let update = self.session.step(self.controls.buttons().swap_bytes())?;
        self.quit |= update.quit;
        if let Some(audio) = &self.audio {
            update.play_audio(audio);
        }
        if let Some(view) = &mut self.view {
            if view.generation != self.session.generation {
                view.drawing.replace(
                    &self.session.content,
                    self.session.vram(),
                    &self.graphics,
                    &mut view.renderer,
                )?;
                view.generation = self.session.generation;
                if let Some(audio) = &self.audio {
                    self.session.content.start_audio(audio);
                }
                self.clock.discard_host_backlog();
                self.last = Instant::now();
            } else {
                view.drawing
                    .update(&self.session.content, &mut view.renderer)?;
            }
        }
        Ok(())
    }

    fn redraw(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        let _ = self.controls.poll(self.focused);
        if self.paused || !self.focused || self.error.is_some() {
            self.clock.discard_host_backlog();
        } else {
            self.clock.elapse(elapsed.min(Duration::from_millis(250)));
            while self.clock.step().is_some() {
                self.step()?;
            }
        }
        if let Some(audio) = &self.audio {
            audio.send(Command::Pause {
                paused: self.paused || !self.focused || self.error.is_some(),
                music_while_paused: false,
            });
        }
        if let Some(view) = &mut self.view {
            view.draw(
                &mut self.session,
                &mut self.open,
                &mut self.paused,
                self.error.as_deref(),
            )?;
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
                pollster::block_on(View::new(Arc::new(window), &self.session, &self.graphics))
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
                    if pressed && !event.repeat {
                        match code {
                            KeyCode::Escape => events.exit(),
                            KeyCode::F1 => self.open = !self.open,
                            KeyCode::F3 => self.paused = !self.paused,
                            KeyCode::F4 if self.paused && self.error.is_none() => {
                                if let Err(error) = self.step() {
                                    self.fail(error);
                                }
                            }
                            KeyCode::Enter => {
                                self.session.action = match self.session.content {
                                    Content::Results(_) => Some(Action::Continue),
                                    Content::Failed | Content::Complete => Some(Action::Retry),
                                    _ => None,
                                }
                            }
                            _ => {}
                        }
                    }
                    if !consumed || !pressed {
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

pub fn run(session: Playback<DiscSource>, graphics: Graphics) -> Result<()> {
    let audio = Output::new()
        .map_err(|error| eprintln!("Audio unavailable: {error}"))
        .ok();
    let mut application = Application {
        session,
        graphics,
        view: None,
        controls: Controls::new(),
        audio,
        clock: GameClock::default(),
        last: Instant::now(),
        paused: false,
        focused: true,
        open: false,
        error: None,
        quit: false,
    };
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    events
        .run_app(&mut application)
        .map_err(|e| e.to_string())?;
    application.error.map_or(Ok(()), Err)
}
