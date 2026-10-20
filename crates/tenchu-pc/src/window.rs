use crate::{
    campaign::{Action, Campaign, Screen},
    controls::{self, Command, Controls},
    render_effects, render_mesh,
    renderer::{self, EffectMesh, Mesh, Renderer},
    session::Options,
    source::FileSource,
    storage::Storage,
    ui,
};
use std::{sync::Arc, time::Instant};
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::{clock::GameClock, game::Game};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
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
    controls: Controls,
    paused: bool,
    faulted: bool,
    fault: Option<String>,
    focused: bool,
    error: Option<String>,
    audio: Option<crate::audio::Audio>,
}

impl Application {
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
                self.controls.clear();
                self.save();
                self.update_view();
            }
            Err(error) => self.pause_error(error),
        }
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.controls.clear();
        if let Some(audio) = &mut self.audio {
            audio.pause(paused || !self.focused);
            if !paused {
                audio.effect(0x0a);
            }
        }
        self.last_frame = Instant::now();
        self.clock.discard_host_backlog();
        self.update_view();
    }

    fn command(&mut self, command: Command, events: &ActiveEventLoop) {
        match command {
            Command::Pause if self.campaign.playing() && !self.faulted => {
                self.set_paused(!self.paused);
            }
            Command::Menu(Action::Back) if matches!(self.campaign.screen, Screen::Launcher) => {
                events.exit();
            }
            Command::Menu(action) => self.action(action),
            _ => {}
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
                let update = self.controls.poll(false);
                self.focused = focused;
                if update.disconnected && self.campaign.playing() {
                    self.set_paused(true);
                }
                if let Some(audio) = &mut self.audio {
                    audio.pause(!focused || self.paused);
                }
                self.controls.clear();
                self.last_frame = Instant::now();
                self.clock.discard_host_backlog();
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if !self.focused || is_synthetic || event.repeat {
                    return;
                }
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                if !self
                    .controls
                    .key(code, event.state == ElementState::Pressed)
                {
                    return;
                }
                if let Some(command) = controls::keyboard_command(code, self.campaign.playing()) {
                    self.command(command, events);
                }
            }

            WindowEvent::RedrawRequested => {
                let update = self.controls.poll(self.focused);
                if update.disconnected && self.campaign.playing() {
                    eprintln!("Active controller disconnected; gameplay paused.");
                    self.set_paused(true);
                } else if self.focused
                    && let Some(command) = controls::gamepad_command(
                        update.pressed,
                        self.campaign.playing(),
                        self.paused,
                    )
                {
                    self.command(command, events);
                }
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
                        if let Err(error) = self.campaign.tick(self.controls.buttons()) {
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
                            self.controls.clear();
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
        controls: Controls::new(),
        paused: false,
        faulted: false,
        fault: None,
        focused: true,
        error: None,
        audio,
    };
    println!(
        "Keyboard: WASD/arrows move, Space jump, Shift crouch, J attack, E item, Q look, Z/C item selection, M/Tab map. P pause, F5 retry, Esc inventory.\nController: D-pad/left stick move, south jump, west attack, east/R1 crouch, north item, L1 look, L2/R2 items, Select map, Start pause. Menus: south confirm, east back, west retry.\nGameplay is still in development; an unsupported action pauses with a diagnostic."
    );
    EventLoop::new()
        .map_err(|e| e.to_string())?
        .run_app(&mut application)
        .map_err(|e| e.to_string())?;
    application.error.map_or(Ok(()), Err)
}
