use crate::{
    campaign::{Action, Campaign, Screen},
    controls::{self, Command, Controls},
    recording::{Recorder, Start},
    render_effects, render_mesh,
    renderer::{self, EffectMesh, Mesh, Renderer},
    session::Options,
    settings::{
        self, Settings,
        menu::{Item, Menu},
    },
    source::FileSource,
    storage::Storage,
    ui,
};
use std::{fs::File, io::BufWriter, path::PathBuf, sync::Arc, time::Instant};
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::{clock::GameClock, game::Game};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window, WindowId},
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

    fn settings(&mut self, settings: &Settings) {
        self.renderer.configure(&settings.graphics);
        self.window
            .set_fullscreen(settings.fullscreen.then_some(Fullscreen::Borderless(None)));
        let mode = if settings.vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        if self.config.present_mode != mode {
            self.config.present_mode = mode;
            self.surface.configure(&self.renderer.device, &self.config);
        }
    }

    fn update(
        &mut self,
        campaign: &Campaign,
        paused: bool,
        error: Option<&str>,
        settings: &Settings,
        menu: &Menu,
        fps: f32,
    ) {
        let game = &campaign.game;
        if self.vram_revision != game.mission.vram.revision {
            self.renderer.update_vram(&game.mission.vram);
            self.vram_revision = game.mission.vram.revision;
        }
        self.actor = self.renderer.upload(&render_mesh::characters(game));
        let mut geometry = if menu.open {
            menu.build(&game.mission.font, settings)
        } else if campaign.playing() && !paused && error.is_none() {
            render_effects::build_with_hud(game, settings.show_hud)
        } else {
            ui::build(campaign, paused, error)
        };
        if settings.show_fps && !menu.open {
            geometry.append(render_effects::text(
                &game.mission.font,
                &[(
                    -154,
                    -115,
                    format!("{fps:.0} FPS  tick {}", game.tick),
                    [1.0, 0.85, 0.35],
                )],
            ));
        }
        self.effects = self.renderer.upload_effects(geometry);
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
    recording_directory: Option<PathBuf>,
    recording: Option<Recorder<BufWriter<File>>>,
    settings: Settings,
    settings_store: settings::Store,
    menu: Menu,
    pointer: [f64; 2],
    fps: f32,
    fps_elapsed: f32,
    fps_frames: u32,
}

impl Application {
    fn recording_error(&mut self, error: String) {
        eprintln!("Recording stopped: {error}");
        self.recording = None;
    }

    fn flush_recording(&mut self) {
        if let Some(recording) = &mut self.recording
            && let Err(error) = recording.flush()
        {
            self.recording_error(error);
        }
    }

    fn finish_recording(&mut self) {
        self.flush_recording();
        self.recording = None;
    }

    fn record_input(&mut self, buttons: u16) {
        if let Some(recording) = &mut self.recording
            && let Err(error) = recording.push(buttons)
        {
            self.recording_error(error);
        }
    }

    fn update_view(&mut self) {
        if let Some(view) = &mut self.view {
            view.update(
                &self.campaign,
                self.paused,
                self.fault.as_deref(),
                &self.settings,
                &self.menu,
                self.fps,
            );
        }
    }

    fn is_suspended(&self) -> bool {
        self.paused || self.menu.open || (!self.focused && self.settings.pause_on_defocus)
    }

    fn sync_pause(&mut self) {
        let paused = self.is_suspended();
        if let Some(audio) = &mut self.audio {
            audio.pause(paused);
        }
        self.controls.clear();
        self.last_frame = Instant::now();
        self.clock.discard_host_backlog();
    }

    fn toggle_menu(&mut self) {
        self.menu.open = !self.menu.open;
        self.flush_recording();
        self.sync_pause();
        self.update_view();
    }

    fn settings_action(&mut self, action: Action) {
        let rows = settings::menu::rows(self.menu.page, &self.settings);
        if let Some(direction) = self.menu.action(action, rows.len()) {
            match rows[self.menu.row].item {
                Item::Fps => self.settings.show_fps = !self.settings.show_fps,
                Item::Focus => self.settings.pause_on_defocus = !self.settings.pause_on_defocus,
                Item::Fullscreen => self.settings.fullscreen = !self.settings.fullscreen,
                Item::Vsync => self.settings.vsync = !self.settings.vsync,
                Item::Hud => self.settings.show_hud = !self.settings.show_hud,
                Item::Volume => {
                    self.settings.volume =
                        (i32::from(self.settings.volume) + direction * 5).clamp(0, 100) as u16
                }
                Item::Reset => self.settings = Settings::default(),
                Item::Fog => self.settings.graphics.fog = !self.settings.graphics.fog,
                Item::FogDistance => {
                    self.settings.graphics.fog_distance =
                        (i32::from(self.settings.graphics.fog_distance) + direction * 25)
                            .clamp(25, 400) as u16
                }
                Item::Textures => {
                    self.settings.graphics.textures = !self.settings.graphics.textures
                }
                Item::Shading => self.settings.graphics.shading = !self.settings.graphics.shading,
                Item::Exposure => {
                    self.settings.graphics.exposure = (i32::from(self.settings.graphics.exposure)
                        + direction * 25)
                        .clamp(25, 300) as u16
                }
                Item::Filtering => {
                    self.settings.graphics.bilinear = !self.settings.graphics.bilinear
                }
                Item::Antialiasing => {
                    if let Some(view) = &self.view {
                        let counts = &view.renderer.sample_counts;
                        let index = counts
                            .iter()
                            .position(|&n| n == self.settings.graphics.samples)
                            .unwrap_or(0);
                        self.settings.graphics.samples = counts
                            [(index as i32 + direction).rem_euclid(counts.len() as i32) as usize];
                    }
                }
            }
            self.menu.notice = match self.settings_store.write(&self.settings) {
                Ok(()) => String::new(),
                Err(error) => {
                    eprintln!("Settings: {error}");
                    "Could not save settings; see terminal".into()
                }
            };
            if let Some(view) = &mut self.view {
                view.settings(&self.settings);
            }
            if let Some(audio) = &self.audio {
                audio.volume(self.settings.volume);
            }
        }
        self.sync_pause();
        self.update_view();
    }

    fn pause_error(&mut self, error: String) {
        self.flush_recording();
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
        let start = if self.recording_directory.is_some()
            && matches!(action, Action::Confirm)
            && let Screen::Inventory(loadout) = &self.campaign.screen
        {
            Some(Start {
                options: self.campaign.options,
                seed: self.campaign.seed,
                inventory: loadout.clone().finish().1,
            })
        } else {
            None
        };
        match self.campaign.action(action, &self.assets) {
            Ok(reload) => {
                if reload
                    && let Some(start) = start
                    && let Some(directory) = &self.recording_directory
                {
                    match Recorder::create(directory, start) {
                        Ok(recording) => self.recording = Some(recording),
                        Err(error) => self.recording_error(error),
                    }
                }
                if !self.campaign.playing() {
                    self.finish_recording();
                }
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
        if paused {
            self.flush_recording();
        }
        self.paused = paused;
        self.sync_pause();
        if !self.is_suspended()
            && let Some(audio) = &self.audio
        {
            audio.effect(0x0a);
        }
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
                Ok(mut view) => {
                    if !view
                        .renderer
                        .sample_counts
                        .contains(&self.settings.graphics.samples)
                    {
                        self.settings.graphics.samples = 1;
                        self.menu.notice = "Saved MSAA mode unavailable; using Off".into();
                    }
                    view.settings(&self.settings);
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
                if !focused {
                    self.flush_recording();
                }
                let update = self.controls.poll(false);
                self.focused = focused;
                if update.disconnected && self.campaign.playing() {
                    self.set_paused(true);
                }
                self.sync_pause();
            }
            WindowEvent::CursorMoved { position, .. } => self.pointer = [position.x, position.y],
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } if self.menu.open => {
                if let Some(view) = &self.view {
                    let y = self.pointer[1] / f64::from(view.size[1].max(1)) * 240.0 - 120.0;
                    let x = (self.pointer[0] - f64::from(view.size[0]) / 2.0)
                        / f64::from(view.size[1].max(1))
                        * 240.0;
                    let row = ((y + 64.0) / 19.0).floor() as isize;
                    if (-150.0..150.0).contains(&x)
                        && row >= 0
                        && (row as usize)
                            < settings::menu::rows(self.menu.page, &self.settings).len()
                    {
                        self.menu.row = row as usize;
                        self.settings_action(if button == MouseButton::Right {
                            Action::Left
                        } else {
                            Action::Right
                        });
                    }
                }
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
                if event.state == ElementState::Pressed && code == KeyCode::F1 {
                    self.toggle_menu();
                    return;
                }
                if self.menu.open {
                    if event.state == ElementState::Pressed
                        && matches!(code, KeyCode::Tab | KeyCode::PageDown | KeyCode::PageUp)
                    {
                        self.menu.page(if code == KeyCode::PageUp { -1 } else { 1 });
                        self.update_view();
                        return;
                    }
                    if event.state == ElementState::Pressed
                        && let Some(Command::Menu(action)) = controls::keyboard_command(code, false)
                    {
                        self.settings_action(action);
                    }
                    return;
                }
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
                } else if self.focused && self.menu.open {
                    if update.pressed & (tenchu_core::input::L1 | tenchu_core::input::R1) != 0 {
                        self.menu
                            .page(if update.pressed & tenchu_core::input::L1 != 0 {
                                -1
                            } else {
                                1
                            });
                        self.controls.clear();
                        self.update_view();
                    }
                    if let Some(Command::Menu(action)) =
                        controls::gamepad_command(update.pressed, false, false)
                    {
                        self.settings_action(action);
                    }
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
                self.fps_elapsed += elapsed.as_secs_f32();
                self.fps_frames += 1;
                let fps_changed = self.fps_elapsed >= 0.5;
                if fps_changed {
                    self.fps = self.fps_frames as f32 / self.fps_elapsed;
                    self.fps_elapsed = 0.0;
                    self.fps_frames = 0;
                }
                let suspended = self.is_suspended();
                if let Some(audio) = &mut self.audio {
                    audio.pause(suspended);
                    audio.poll();
                }
                let before = self.campaign.game.tick;
                if !suspended && self.campaign.playing() {
                    self.clock.elapse(elapsed);
                    while self.clock.step().is_some() {
                        let buttons = if self.focused {
                            self.controls.buttons()
                        } else {
                            0
                        };
                        self.record_input(buttons);
                        if let Err(error) = self.campaign.tick(buttons) {
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
                            self.finish_recording();
                            self.controls.clear();
                            self.clock.discard_host_backlog();
                            break;
                        }
                    }
                    self.save();
                }
                if before != self.campaign.game.tick || fps_changed && self.settings.show_fps {
                    self.update_view();
                }
                if let Some(view) = &mut self.view {
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

pub fn run(
    assets: GameAssets<FileSource>,
    options: Options,
    disc: &std::path::Path,
    recording_directory: Option<PathBuf>,
) -> Result<()> {
    let storage = Storage::new()?;
    let settings_store = settings::Store::new()?;
    let (settings, notice) = match settings_store.load() {
        Ok(settings) => (settings, String::new()),
        Err(error) => {
            eprintln!("Settings: {error}. Using defaults.");
            (
                Settings::default(),
                "Settings file unreadable; using defaults".into(),
            )
        }
    };
    let campaign = Campaign::new(&assets, options, storage.load()?)?;
    let audio = match crate::audio::Audio::new(disc, &assets, &campaign) {
        Ok(audio) => Some(audio),
        Err(error) => {
            eprintln!("Audio unavailable: {error}");
            None
        }
    };
    if let Some(audio) = &audio {
        audio.volume(settings.volume);
    }
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
        recording_directory,
        recording: None,
        settings,
        settings_store,
        menu: Menu {
            notice,
            ..Menu::default()
        },
        pointer: [0.0; 2],
        fps: 0.0,
        fps_elapsed: 0.0,
        fps_frames: 0,
    };
    println!(
        "Keyboard: WASD/arrows move, Space jump, Shift crouch, J attack, E item, Q look, Z/C item selection, M/Tab map. P pause, F5 retry, Esc inventory.\nController: D-pad/left stick move, south jump, west attack, east/R1 crouch, north item, L1 look, L2/R2 items, Select map, Start pause. Menus: south confirm, east back, west retry.\nGameplay is still in development; an unsupported action pauses with a diagnostic."
    );
    let result = EventLoop::new()
        .map_err(|e| e.to_string())?
        .run_app(&mut application)
        .map_err(|e| e.to_string());
    application.finish_recording();
    result?;
    application.error.map_or(Ok(()), Err)
}
