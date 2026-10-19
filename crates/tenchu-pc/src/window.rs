use crate::{
    campaign::{Action, Campaign, Screen},
    render_effects, render_mesh,
    renderer::{self, EffectMesh, Mesh, Renderer},
    session::Options,
    source::FileSource,
    storage::Storage,
    ui,
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
    effects: EffectMesh,
    size: [u32; 2],
    vram_revision: u64,
}

impl View {
    fn new(events: &ActiveEventLoop, game: &Game) -> Result<Self> {
        let window = Arc::new(
            events
                .create_window(
                    Window::default_attributes()
                        .with_title("Tenchu — gameplay preview")
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
        let actor = renderer.upload(&render_mesh::characters(game));
        let effects = renderer.upload_effects(render_effects::build(game));
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
            effects,
            size: [size.width, size.height],
            vram_revision: game.mission.vram.revision,
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

    fn reload(&mut self, game: &Game) {
        self.renderer.update_vram(&game.mission.vram);
        self.vram_revision = game.mission.vram.revision;
        self.stage = self
            .renderer
            .upload(&render_mesh::stage(&game.mission, &game.math));
    }

    fn update(&mut self, campaign: &Campaign, paused: bool, error: Option<&str>) {
        let game = &campaign.game;
        if self.vram_revision != game.mission.vram.revision {
            self.renderer.update_vram(&game.mission.vram);
            self.vram_revision = game.mission.vram.revision;
        }
        self.actor = self.renderer.upload(&render_mesh::characters(game));
        self.effects = self
            .renderer
            .upload_effects(ui::build(campaign, paused, error));
        self.window.set_title(&format!(
            "Tenchu — gameplay preview | tick {} | HP {} | motion {:04x}",
            game.tick,
            game.player().health,
            game.player().motion.action
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
                game.rendered_view,
                self.config.width as f32 / self.config.height as f32,
            ),
            &[&self.stage, &self.actor],
            Some(&self.effects),
        );
        self.window.pre_present_notify();
        frame.present();
        Ok(())
    }
}

struct Application {
    assets: GameAssets<FileSource>,
    campaign: Campaign,
    storage: Storage,
    view: Option<View>,
    clock: GameClock,
    last_frame: Instant,
    keys: HashSet<KeyCode>,
    paused: bool,
    faulted: bool,
    fault: Option<String>,
    focused: bool,
    error: Option<String>,
    audio: Option<crate::audio::Audio>,
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
                    KeyCode::ShiftLeft | KeyCode::ShiftRight => input::CIRCLE,
                    KeyCode::KeyJ => input::SQUARE,
                    KeyCode::KeyE => input::TRIANGLE,
                    KeyCode::KeyQ => input::L1,
                    KeyCode::KeyZ => input::L2,
                    KeyCode::KeyC => input::R2,
                    KeyCode::KeyM | KeyCode::Tab => input::SELECT,
                    _ => 0,
                }
        })
    }

    fn update_view(&mut self) {
        if let Some(view) = &mut self.view {
            view.update(&self.campaign, self.paused, self.fault.as_deref());
        }
    }

    fn pause_error(&mut self, error: String) {
        eprintln!(
            "Tick {}: {error}. Press F5 to retry.",
            self.campaign.game.tick
        );
        self.paused = true;
        if let Some(audio) = &mut self.audio {
            audio.pause(true);
        }
        self.faulted = true;
        self.fault = Some(error);
        self.update_view();
    }

    fn save(&mut self) {
        if let Some(save) = self.campaign.save_request.take() {
            match self.storage.write(&save) {
                Ok(()) => self.campaign.saved = Some(save),
                Err(error) => self.pause_error(error),
            }
        }
    }

    fn action(&mut self, action: Action) {
        match self.campaign.action(action, &self.assets) {
            Ok(reload) => {
                if let Some(audio) = &mut self.audio {
                    if reload && let Err(error) = audio.reload(&self.assets, &self.campaign.game) {
                        eprintln!("Audio: {error}");
                    }
                    audio.screen(&self.campaign.screen);
                    audio.effect(match action {
                        Action::Up | Action::Down | Action::Left | Action::Right => 0x0b,
                        _ => 0x0a,
                    });
                }
                if reload && let Some(view) = &mut self.view {
                    view.reload(&self.campaign.game);
                }
                self.clock = GameClock::default();
                self.last_frame = Instant::now();
                self.paused = false;
                self.faulted = false;
                self.fault = None;
                self.keys.clear();
                self.save();
                self.update_view();
            }
            Err(error) => self.pause_error(error),
        }
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.view.is_none() {
            match View::new(events, &self.campaign.game) {
                Ok(view) => {
                    self.view = Some(view);
                    self.update_view();
                }
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
                if let Some(audio) = &mut self.audio {
                    audio.pause(!focused || self.paused);
                }
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
                    KeyCode::Escape if matches!(self.campaign.screen, Screen::Launcher) => {
                        events.exit()
                    }
                    KeyCode::Escape => self.action(Action::Back),
                    KeyCode::F5 => self.action(Action::Replay),
                    KeyCode::KeyP if !self.faulted && self.campaign.playing() => {
                        self.paused = !self.paused;
                        if let Some(audio) = &mut self.audio {
                            audio.pause(self.paused);
                            if !self.paused {
                                audio.effect(0x0a);
                            }
                        }
                        self.clock.discard_host_backlog();
                        self.update_view();
                    }
                    _ if !self.campaign.playing() => {
                        let action = match code {
                            KeyCode::Enter => Some(Action::Confirm),
                            KeyCode::ArrowUp => Some(Action::Up),
                            KeyCode::ArrowDown => Some(Action::Down),
                            KeyCode::ArrowLeft => Some(Action::Left),
                            KeyCode::ArrowRight => Some(Action::Right),
                            KeyCode::KeyR => Some(Action::Replay),
                            _ => None,
                        };
                        if let Some(action) = action {
                            self.action(action);
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame);
                self.last_frame = now;
                if let Some(audio) = &mut self.audio {
                    audio.pause(!self.focused || self.paused);
                    audio.poll();
                }
                let before = self.campaign.game.tick;
                if self.focused && !self.paused && self.campaign.playing() {
                    self.clock.elapse(elapsed);
                    while self.clock.step().is_some() {
                        if let Err(error) = self.campaign.tick(self.buttons()) {
                            self.pause_error(error);
                            self.clock.discard_host_backlog();
                            break;
                        }
                        if let Some(audio) = &mut self.audio {
                            if let Err(error) = audio.tick(&self.campaign.game) {
                                eprintln!("Audio: {error}");
                            }
                            audio.screen(&self.campaign.screen);
                        }
                        if !self.campaign.playing() {
                            self.clock.discard_host_backlog();
                            break;
                        }
                    }
                    self.save();
                }
                if let Some(view) = &mut self.view {
                    if before != self.campaign.game.tick && !self.paused {
                        view.update(&self.campaign, self.paused, self.fault.as_deref());
                    }
                    if let Err(error) = view.draw(&self.campaign.game) {
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

pub fn run(assets: GameAssets<FileSource>, options: Options, disc: &std::path::Path) -> Result<()> {
    let storage = Storage::new()?;
    let campaign = Campaign::new(&assets, options, storage.load()?)?;
    let audio = match crate::audio::Audio::new(disc, &assets, &campaign) {
        Ok(audio) => Some(audio),
        Err(error) => {
            eprintln!("Audio unavailable: {error}");
            None
        }
    };
    let mut application = Application {
        assets,
        campaign,
        storage,
        view: None,
        clock: GameClock::default(),
        last_frame: Instant::now(),
        keys: HashSet::new(),
        paused: false,
        faulted: false,
        fault: None,
        focused: true,
        error: None,
        audio,
    };
    println!(
        "Gameplay: WASD/arrows move, Space jump, Shift crouch, J attack, E item, Q look, Z/C item selection, M/Tab map. P pause, F5 retry, Esc inventory.\nRemaining items and special attacks are still being implemented; an unsupported action pauses with a diagnostic."
    );
    EventLoop::new()
        .map_err(|e| e.to_string())?
        .run_app(&mut application)
        .map_err(|e| e.to_string())?;
    application.error.map_or(Ok(()), Err)
}
