use crate::{
    render_mesh,
    renderer::{self, Mesh, Renderer},
    session::Options,
    source::FileSource,
};
use std::{collections::HashSet, sync::Arc, time::Instant};
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::{clock::GameClock, game::Game, input};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
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
    stage: Mesh,
    actor: Mesh,
    size: [u32; 2],
}

impl View {
    fn new(events: &ActiveEventLoop, game: &Game) -> Result<Self> {
        let window = Arc::new(
            events
                .create_window(
                    Window::default_attributes()
                        .with_title("Tenchu — movement preview")
                        .with_inner_size(LogicalSize::new(1280, 720)),
                )
                .map_err(|e| e.to_string())?,
        );
        let instance = wgpu::Instance::new(&Default::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .ok_or("No graphics adapter for this window")?;
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
        let renderer = pollster::block_on(Renderer::new(
            &adapter,
            config.format,
            [config.width, config.height],
            &game.mission.vram,
        ))?;
        surface.configure(&renderer.device, &config);
        let stage = renderer.upload(&render_mesh::stage(&game.mission, &game.math));
        let actor = renderer.upload(&render_mesh::player(game));
        println!(
            "Renderer: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend
        );
        Ok(Self {
            window,
            surface,
            config,
            renderer,
            stage,
            actor,
            size: [size.width, size.height],
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.size = [width, height];
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.renderer.device, &self.config);
        self.renderer.resize([width, height]);
    }

    fn update(&mut self, game: &Game) {
        self.actor = self.renderer.upload(&render_mesh::player(game));
        self.window.set_title(&format!(
            "Tenchu — movement preview | tick {} | HP {} | motion {:04x}",
            game.tick, game.player.health, game.player.motion.action
        ));
    }

    fn draw(&mut self, game: &Game) -> Result<()> {
        if self.size.contains(&0) {
            return Ok(());
        }
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.renderer.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(error) => return Err(error.to_string()),
        };
        self.renderer.draw(
            &frame.texture.create_view(&Default::default()),
            renderer::game_camera(
                game.camera.view,
                self.config.width as f32 / self.config.height as f32,
            ),
            &[&self.stage, &self.actor],
        );
        self.window.pre_present_notify();
        frame.present();
        Ok(())
    }
}

struct Application {
    assets: GameAssets<FileSource>,
    options: Options,
    game: Game,
    view: Option<View>,
    clock: GameClock,
    last_frame: Instant,
    keys: HashSet<KeyCode>,
    paused: bool,
    focused: bool,
    error: Option<String>,
}

impl Application {
    fn buttons(&self) -> u16 {
        self.keys.iter().fold(0, |buttons, key| {
            buttons
                | match key {
                    KeyCode::KeyW | KeyCode::ArrowUp => input::UP,
                    KeyCode::KeyS | KeyCode::ArrowDown => input::DOWN,
                    KeyCode::KeyA | KeyCode::ArrowLeft => input::LEFT,
                    KeyCode::KeyD | KeyCode::ArrowRight => input::RIGHT,
                    KeyCode::Space => input::CROSS,
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => input::R1,
                    KeyCode::KeyJ => input::SQUARE,
                    KeyCode::KeyE => input::TRIANGLE,
                    KeyCode::KeyQ => input::L1,
                    KeyCode::KeyZ => input::L2,
                    KeyCode::KeyC => input::R2,
                    _ => 0,
                }
        })
    }

    fn pause_error(&mut self, error: String) {
        eprintln!("Tick {}: {error}. Press F5 to restart.", self.game.tick);
        self.paused = true;
        if let Some(view) = &self.view {
            view.window
                .set_title(&format!("Tenchu — {error} | F5: restart"));
        }
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.view.is_none() {
            match View::new(events, &self.game) {
                Ok(view) => self.view = Some(view),
                Err(error) => {
                    self.error = Some(error);
                    events.exit();
                }
            }
        }
        self.last_frame = Instant::now();
        self.clock.discard_host_backlog();
    }

    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(size) => {
                if let Some(view) = &mut self.view {
                    view.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.keys.clear();
                self.last_frame = Instant::now();
                self.clock.discard_host_backlog();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                if event.state == ElementState::Released {
                    self.keys.remove(&code);
                    return;
                }
                self.keys.insert(code);
                if event.repeat {
                    return;
                }
                match code {
                    KeyCode::Escape => events.exit(),
                    KeyCode::KeyP => {
                        self.paused = !self.paused;
                        self.clock.discard_host_backlog();
                    }
                    KeyCode::F5 => match self.options.load(&self.assets) {
                        Ok(game) => {
                            self.game = game;
                            self.clock = GameClock::default();
                            self.last_frame = Instant::now();
                            self.paused = false;
                            self.keys.clear();
                            if let Some(view) = &mut self.view {
                                view.update(&self.game);
                            }
                        }
                        Err(error) => self.pause_error(error),
                    },
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame);
                self.last_frame = now;
                let before = self.game.tick;
                if self.focused && !self.paused {
                    self.clock.elapse(elapsed);
                    while self.clock.step().is_some() {
                        if let Err(error) = self.game.step(self.buttons()) {
                            self.pause_error(error);
                            self.clock.discard_host_backlog();
                            break;
                        }
                    }
                }
                if let Some(view) = &mut self.view {
                    if before != self.game.tick && !self.paused {
                        view.update(&self.game);
                    }
                    if let Err(error) = view.draw(&self.game) {
                        self.error = Some(error);
                        events.exit();
                    }
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

pub fn run(assets: GameAssets<FileSource>, options: Options) -> Result<()> {
    let game = options.load(&assets)?;
    let mut application = Application {
        assets,
        options,
        game,
        view: None,
        clock: GameClock::default(),
        last_frame: Instant::now(),
        keys: HashSet::new(),
        paused: false,
        focused: true,
        error: None,
    };
    println!(
        "Movement preview: WASD/arrows move, Space jump, Shift crouch, J attack, E item, Q look, Z/C item selection. P pause, F5 restart, Esc quit.\nCombat, items, enemies and events are still being implemented; an unsupported action pauses with a diagnostic."
    );
    EventLoop::new()
        .map_err(|e| e.to_string())?
        .run_app(&mut application)
        .map_err(|e| e.to_string())?;
    application.error.map_or(Ok(()), Err)
}
