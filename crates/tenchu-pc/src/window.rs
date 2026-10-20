mod playback;
mod settings_reset;
#[cfg(test)]
mod tests;
mod tools;
mod view;
use view::View;

use crate::{
    campaign::{Action, Campaign, Screen},
    controls::{self, Command, Controls},
    recording::{Recorder, Start},
    render_effects, render_mesh,
    renderer::{EffectMesh, Mesh, Renderer},
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
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

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
    debug: crate::debug::Debug,
    screenshot: bool,
    modifiers: ModifiersState,
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
                &self.debug,
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
        self.release_pointer();
        self.menu.edit = None;
        self.menu.open = !self.menu.open;
        self.flush_recording();
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
            && self.debug.sight_percent == 100
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
                if reload {
                    self.release_pointer();
                    self.campaign.game.intelligence.sight_percent = self.debug.sight_percent;
                    if let Err(error) = self.debug.reload(&self.campaign.game) {
                        self.pause_error(error);
                        return;
                    }
                }
                if reload
                    && let Some(start) = start
                    && let Some(directory) = &self.recording_directory
                {
                    match Recorder::create(directory, start) {
                        Ok(recording) => self.recording = Some(recording),
                        Err(error) => self.recording_error(error),
                    }
                } else if reload
                    && self.campaign.playing()
                    && self.recording_directory.is_some()
                    && self.debug.sight_percent != 100
                {
                    self.menu.notice = "Recording disabled: sight range modified".into();
                    eprintln!("{}", self.menu.notice);
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
                    if let Err(error) = view.reload(&self.campaign.game, &self.settings) {
                        eprintln!("Visual assets: {error}; using original models");
                        self.menu.notice = error;
                    }
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
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if self.focused
            && self.debug.looking
            && !self.menu.open
            && let DeviceEvent::MouseMotion { delta } = event
            && let Some(camera) = &mut self.debug.camera
        {
            camera.look(delta.0, delta.1);
        }
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.finish_recording();
        self.audio.take();
        // EGL/Wayland resources must die before winit closes its display connection.
        if let Some(view) = self.view.take() {
            view.renderer.device.poll(wgpu::Maintain::Wait);
            drop(view);
        }
    }

    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.view.is_none() {
            match View::new(events, &self.campaign.game, &self.settings) {
                Ok(mut view) => {
                    if !view
                        .renderer
                        .sample_counts
                        .contains(&self.settings.graphics.samples)
                    {
                        self.settings.graphics.samples = 1;
                        self.menu.notice = "Saved MSAA mode unavailable; using Off".into();
                    }
                    if let Err(error) = view.settings(&self.settings, &self.campaign.game) {
                        self.error = Some(error);
                        events.exit();
                        return;
                    }
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
                    self.modifiers = ModifiersState::empty();
                    self.flush_recording();
                    self.release_pointer();
                }
                let update = self.controls.poll(false);
                self.focused = focused;
                if update.disconnected && self.campaign.playing() {
                    self.set_paused(true);
                }
                self.sync_pause();
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = [position.x, position.y];
                if self.menu.open
                    && self.menu.edit.is_none()
                    && let Some(view) = &self.view
                    && let Some(row) = self.menu.row_at(
                        self.pointer,
                        view.size,
                        settings::menu::rows(
                            self.menu.page,
                            &self.settings,
                            &self.debug,
                            self.paused,
                        )
                        .len(),
                    )
                    && row != self.menu.row
                {
                    self.menu.row = row;
                    self.update_view();
                }
            }
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
                    if (-95.0..-75.0).contains(&y) {
                        self.menu.page(if x < 0.0 { -1 } else { 1 });
                        self.update_view();
                        return;
                    }
                    let rows = settings::menu::rows(
                        self.menu.page,
                        &self.settings,
                        &self.debug,
                        self.paused,
                    );
                    if let Some(row) = self.menu.row_at(self.pointer, view.size, rows.len()) {
                        self.menu.row = row;
                        let item = rows[self.menu.row].item;
                        self.settings_action(if button == MouseButton::Right {
                            Action::Left
                        } else if item.button() || matches!(item, Item::Target(_)) {
                            Action::Confirm
                        } else {
                            Action::Right
                        });
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. }
                if self.debug.camera.is_some() && self.focused =>
            {
                if button == MouseButton::Right {
                    if state == ElementState::Released {
                        self.release_pointer();
                    } else if let Some(view) = &self.view {
                        self.debug.looking = view
                            .window
                            .set_cursor_grab(CursorGrabMode::Locked)
                            .or_else(|_| view.window.set_cursor_grab(CursorGrabMode::Confined))
                            .is_ok();
                        view.window.set_cursor_visible(!self.debug.looking);
                    }
                } else if button == MouseButton::Left && state == ElementState::Pressed {
                    self.pick_debug_target();
                }
            }
            WindowEvent::MouseWheel { delta, .. } if self.menu.open => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.settings_action(if y > 0.0 { Action::Up } else { Action::Down });
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
                if event.state == ElementState::Pressed && code == KeyCode::F9 {
                    self.screenshot = true;
                    return;
                }
                if self.menu.open {
                    if self.menu.edit.is_some() {
                        if event.state == ElementState::Pressed {
                            self.edit_coordinate(code, event.text.as_deref());
                        }
                        return;
                    }
                    if event.state == ElementState::Pressed
                        && matches!(code, KeyCode::Tab | KeyCode::PageDown | KeyCode::PageUp)
                    {
                        self.menu.page(
                            if code == KeyCode::PageUp
                                || code == KeyCode::Tab && self.modifiers.shift_key()
                            {
                                -1
                            } else {
                                1
                            },
                        );
                        self.update_view();
                        return;
                    }
                    if event.state == ElementState::Pressed && code == KeyCode::KeyR {
                        self.reset_selected_setting();
                        return;
                    }
                    if event.state == ElementState::Pressed
                        && let Some(Command::Menu(action)) = controls::keyboard_command(code, false)
                    {
                        self.settings_action(action);
                    }
                    return;
                }
                if event.state == ElementState::Pressed {
                    let item = match code {
                        KeyCode::F2 => Some(Item::FreeCamera),
                        KeyCode::F3 => Some(Item::Pause),
                        KeyCode::F4 => Some(Item::Step),
                        KeyCode::F9 => Some(Item::Screenshot),
                        _ => None,
                    };
                    if let Some(item) = item {
                        if let Err(error) = self.apply_item(item, 1) {
                            self.menu.notice = error;
                        }
                        self.release_pointer();
                        self.sync_pause();
                        self.update_view();
                        return;
                    }
                    if code == KeyCode::Escape && self.debug.camera.is_some() {
                        if self.debug.looking || self.debug.teleport_armed {
                            self.release_pointer();
                            self.debug.teleport_armed = false;
                        } else {
                            self.debug.toggle_camera(&self.campaign.game);
                            self.controls.clear();
                        }
                        self.update_view();
                        return;
                    }
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
                if self.focused
                    && !self.menu.open
                    && let Some(camera) = &mut self.debug.camera
                {
                    camera.advance(
                        &self.controls,
                        elapsed.as_secs_f32(),
                        f32::from(self.debug.flight_speed) * 1000.0,
                    );
                }
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
                self.advance_simulation(elapsed);
                self.debug.update(&self.campaign.game);
                if before != self.campaign.game.tick
                    || fps_changed && self.settings.show_fps
                    || self.debug.camera.is_some()
                {
                    self.update_view();
                }
                if self.screenshot {
                    self.save_screenshot();
                }
                if let Some(view) = &mut self.view {
                    if let Err(error) = view.draw(&self.campaign.game, &self.debug) {
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
    let debug = crate::debug::Debug::new(&campaign.game)?;
    let mut audio = match crate::audio::Audio::new(disc, &assets, &campaign) {
        Ok(audio) => Some(audio),
        Err(error) => {
            eprintln!("Audio unavailable: {error}");
            None
        }
    };
    if let Some(audio) = &mut audio {
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
        debug,
        screenshot: false,
        modifiers: ModifiersState::empty(),
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
