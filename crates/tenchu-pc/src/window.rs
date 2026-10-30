pub mod benchmark;
mod demo;
mod diagnostics;
mod frontend;
mod navigation;
mod overlay;
mod performance;
mod playback;
mod randomizer;
#[cfg(not(target_arch = "wasm32"))]
pub mod retail;
mod settings_reset;
#[cfg(test)]
mod tests;
mod tools;
#[cfg(not(target_arch = "wasm32"))]
mod video;
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
    source::DiscSource,
    storage::Storage,
    ui,
};
use std::{fs::File, io::BufWriter, path::PathBuf, sync::Arc};
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::{clock::GameClock, game::Game};
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

struct Application {
    assets: GameAssets<DiscSource>,
    campaign: Campaign,
    storage: Storage,
    view: Option<View>,
    clock: GameClock,
    last_frame: Instant,
    controls: Controls,
    orbit: crate::navigation::Orbit,
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
    performance: performance::Performance,
    recent_inputs: diagnostics::RecentInputs,
    benchmarks: benchmark::Benchmarks,
    presets: settings::library::Manager,
    randomizer: crate::randomizer::Editor,
    fps_elapsed: f32,
    fps_frames: u32,
    debug: crate::debug::Debug,
    screenshot: bool,
    demo: Option<crate::demo::Player>,
    demo_exit: bool,
    #[cfg(not(target_arch = "wasm32"))]
    video: Option<crate::video::Export>,
    modifiers: ModifiersState,
    frontend: Option<crate::frontend::Frontend>,
}

impl Application {
    fn exporting_video(&self) -> bool {
        cfg!(not(target_arch = "wasm32")) && self.demo.as_ref().is_some_and(|d| d.video.is_some())
    }

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
        #[cfg(target_arch = "wasm32")]
        crate::web::update_snapshot(
            &self.campaign,
            self.is_suspended(),
            self.fault.as_deref().unwrap_or(""),
        );
        let alpha = if let Some(demo) = &self.demo {
            demo.alpha()
        } else if self.is_suspended()
            || !self.campaign.playing()
            || self.debug.speed == 0
            || self.campaign.game.cinematic.active
                && (self.settings.cutscene_speed == 0 || self.controls.held(KeyCode::Backspace))
        {
            1.0
        } else {
            self.clock.interpolation()
        };
        let offline = self.exporting_video() || self.benchmarks.run.is_some();
        self.debug.follow_camera = self.navigation_camera(alpha, 0.0);
        if let Some(view) = &mut self.view {
            view.update(view::Frame {
                campaign: &self.campaign,
                paused: self.paused && self.demo.is_none(),
                error: self.fault.as_deref(),
                settings: &self.settings,
                menu: &self.menu,
                fps: self.fps,
                offline,
                debug: &self.debug,
                alpha,
            });
            if let Some(demo) = &self.demo {
                view.window.set_title(&format!(
                    "Tenchu — showcase | {} | {:.1} / {}s | tick {}",
                    demo.clip().id,
                    demo.time(),
                    demo.clip().seconds,
                    self.campaign.game.tick
                ));
                view.labels
                    .retain(|label| !matches!(label.0, crate::debug::Anchor::BottomLeft(_)));
            }
        }
    }

    fn is_suspended(&self) -> bool {
        self.paused
            || (self.randomizer.open && self.demo.is_none())
            || self.demo.as_ref().is_some_and(|d| d.library || d.finished)
            || (self.menu.open && self.settings.pause_in_menu)
            || (!self.focused && self.settings.pause_on_defocus)
    }

    fn sync_pause(&mut self) {
        let paused = self.is_suspended();
        if paused || !self.campaign.playing() {
            self.release_pointer();
        }
        #[cfg(target_arch = "wasm32")]
        crate::web::media_pause(paused);
        if let Some(audio) = &mut self.audio {
            audio.pause(paused);
        }
        self.controls.clear();
        self.last_frame = Instant::now();
        self.clock.discard_host_backlog();
        if let Some(view) = &mut self.view {
            view.reset_interpolation(&self.campaign.game);
        }
    }

    fn toggle_menu(&mut self) {
        self.release_pointer();
        self.menu.edit = None;
        self.menu.hovered = None;
        self.menu.open = !self.menu.open;
        self.flush_recording();
        self.sync_pause();
        self.update_view();
    }

    fn pause_error(&mut self, error: String) {
        if let Some(frontend) = &mut self.frontend {
            frontend.notice = format!("{error} — F6: quick launcher");
        }
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
        if self.demo.is_some() || self.campaign.randomized.is_some() {
            return;
        }
        if let Some(save) = self.campaign.save_request.take() {
            if !self.settings.frontend.auto_save {
                return;
            }
            match self.storage.write(&save) {
                Ok(()) => self.campaign.saved = Some(save),
                Err(error) => self.pause_error(error),
            }
        }
    }

    fn action(&mut self, action: Action) {
        let action = if matches!(action, Action::Start) {
            Action::Confirm
        } else {
            action
        };
        if self.randomizer_command(action) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if matches!(action, Action::Confirm | Action::Replay)
            && !self.campaign.playing()
            && !self.browser_ready()
        {
            return;
        }
        let briefing = matches!(self.campaign.screen, Screen::Results(_))
            && matches!(action, Action::Confirm | Action::Start)
            && !self.campaign.quick_launcher;
        let start = if self.recording_directory.is_some()
            && self.debug.sight_percent == 100
            && !self.settings.keep_enemies_active
            && !self.settings.model_moveset
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
                if self.frontend.is_some()
                    && (matches!(self.campaign.screen, Screen::Complete)
                        || matches!(self.campaign.screen, Screen::Launcher)
                            && !self.campaign.quick_launcher)
                {
                    self.open_title();
                }
                if reload && let Err(error) = self.reload_game() {
                    self.pause_error(error);
                    return;
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
                    && (self.debug.sight_percent != 100 || self.settings.keep_enemies_active)
                {
                    self.menu.notice = "Recording disabled: enemy simulation modified".into();
                    eprintln!("{}", self.menu.notice);
                }
                if !self.campaign.playing() {
                    self.finish_recording();
                }
                if let Some(audio) = &mut self.audio {
                    audio.screen(&self.campaign.screen);
                    audio.effect(match action {
                        Action::Up | Action::Down | Action::Left | Action::Right => 0x0b,
                        _ => 0x0a,
                    });
                }
                if matches!(self.campaign.screen, Screen::Inventory(_))
                    && let Some(frontend) = &mut self.frontend
                    && let Err(error) = frontend.prepare(&self.campaign, &self.assets, briefing)
                {
                    self.pause_error(error);
                    return;
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
        if self.randomizer.open {
            if matches!(command, Command::Menu(Action::Back)) {
                self.toggle_randomizer();
            }
            return;
        }
        if self.demo.is_some() {
            if matches!(command, Command::Pause) {
                self.set_paused(!self.paused);
            }
            return;
        }
        if self.frontend_visible() {
            if let Command::Menu(action) = command {
                if matches!(self.campaign.screen, Screen::Results(_)) {
                    if self.frontend.as_ref().is_some_and(|f| f.ready()) {
                        self.action(action);
                    }
                } else {
                    self.frontend_action(action);
                }
                self.frontend_events(events);
            }
            return;
        }
        match command {
            Command::Pause if self.campaign.playing() && !self.faulted => {
                self.set_paused(!self.paused);
            }
            Command::Menu(Action::Back) if matches!(self.campaign.screen, Screen::Launcher) => {
                self.open_title();
                self.frontend_events(events);
            }
            Command::Menu(action) => self.action(action),
            _ => {}
        }
    }
}

impl ApplicationHandler for Application {
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if self.focused
            && !self.menu.open
            && let DeviceEvent::MouseMotion { delta } = event
        {
            if self.debug.looking
                && let Some(camera) = &mut self.debug.camera
            {
                camera.look(delta.0, delta.1);
            } else if self.orbit.looking
                && !self.is_suspended()
                && self.navigation_enabled()
                && self.campaign.game.player_can_orbit()
                && self.campaign.game.player().pad.held & tenchu_core::input::L1 == 0
            {
                self.orbit
                    .mouse(delta.0, delta.1, &self.settings.navigation);
            }
        }
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        crate::web::closed();
        self.finish_recording();
        self.controls.clear();
        self.audio.take();
        #[cfg(not(target_arch = "wasm32"))]
        self.video.take();
        self.frontend.take();
        // EGL/Wayland resources must die before winit closes its display connection.
        if let Some(view) = self.view.take() {
            #[cfg(not(target_arch = "wasm32"))]
            view.renderer.device.poll(wgpu::Maintain::Wait);
            drop(view);
        }
    }

    fn resumed(&mut self, _events: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        let events = _events;
        #[cfg(not(target_arch = "wasm32"))]
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
                    if self.exporting_video() {
                        let view = self.view.as_mut().unwrap();
                        view.resize(crate::video::SIZE[0], crate::video::SIZE[1]);
                        match crate::video::Export::new(
                            self.demo.as_ref().unwrap(),
                            &view.renderer,
                            view.config.format,
                        ) {
                            Ok(video) => self.video = Some(video),
                            Err(error) => {
                                self.error = Some(error);
                                events.exit();
                                return;
                            }
                        }
                        events.set_control_flow(winit::event_loop::ControlFlow::Poll);
                    }
                    if let Err(error) = self.start_demo_clip() {
                        self.error = Some(error);
                        events.exit();
                        return;
                    }
                    self.update_view();
                }
                Err(error) => {
                    self.error = Some(error);
                    events.exit();
                }
            }
        }
        self.update_view();
        self.last_frame = Instant::now();
        self.clock.discard_host_backlog();
    }

    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        #[cfg(target_arch = "wasm32")]
        let event = {
            let mut event = event;
            if let Some(view) = &self.view {
                match &mut event {
                    WindowEvent::CursorMoved { position, .. } => view.scale_pointer(position),
                    WindowEvent::Touch(touch) => view.scale_pointer(&mut touch.location),
                    _ => {}
                }
            }
            event
        };
        if self.exporting_video() {
            if matches!(event, WindowEvent::CloseRequested)
                || matches!(event, WindowEvent::KeyboardInput { ref event, .. }
                    if event.physical_key == PhysicalKey::Code(KeyCode::Escape))
            {
                self.error = Some("Video export cancelled".into());
                events.exit();
            }
            return;
        }
        if self.menu.open
            && self.controls.capturing()
            && let WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } = &event
            && let PhysicalKey::Code(code) = event.physical_key
            && code != KeyCode::F1
        {
            if self.focused
                && !is_synthetic
                && !event.repeat
                && event.state == ElementState::Pressed
            {
                self.controls.editor_key(code);
            }
            return;
        }
        if let Some(view) = &mut self.view {
            view.overlay.event(&view.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(size) => {
                if self.benchmarks.run.as_ref().is_some_and(|r| r.frame > 1)
                    && self
                        .view
                        .as_ref()
                        .is_some_and(|v| v.size != [size.width, size.height])
                    && let Err(error) =
                        self.finish_benchmark(Some("Cancelled: window size changed".into()))
                {
                    self.pause_error(error);
                }
                if let Some(view) = &mut self.view {
                    #[cfg(target_arch = "wasm32")]
                    {
                        self.settings.fullscreen = view.window.fullscreen().is_some();
                    }
                    view.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(focused) => {
                if !focused
                    && self
                        .benchmarks
                        .run
                        .as_ref()
                        .is_some_and(|r| r.target.is_none())
                    && let Err(error) =
                        self.finish_benchmark(Some("Cancelled: window lost focus".into()))
                {
                    self.pause_error(error);
                }
                if !focused {
                    self.controls.cancel_capture();
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
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Right,
                ..
            } => {
                self.release_pointer();
            }
            WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. }
                if self.menu.open || self.randomizer.open || self.demo.is_some() => {}
            WindowEvent::MouseInput { state, button, .. }
                if (self.debug.camera.is_some()
                    || self.debug.click_teleport
                    || self.navigation_enabled())
                    && self.focused =>
            {
                if button == MouseButton::Right
                    && (self.debug.camera.is_some() || self.navigation_enabled())
                {
                    if (self.debug.camera.is_some()
                        || !self.is_suspended() && self.campaign.game.player_can_orbit())
                        && let Some(view) = &self.view
                    {
                        let looking = view
                            .window
                            .set_cursor_grab(CursorGrabMode::Locked)
                            .or_else(|_| view.window.set_cursor_grab(CursorGrabMode::Confined))
                            .is_ok();
                        self.debug.looking = looking && self.debug.camera.is_some();
                        self.orbit.looking = looking && self.debug.camera.is_none();
                        view.window.set_cursor_visible(!looking);
                    }
                } else if button == MouseButton::Middle
                    && state == ElementState::Pressed
                    && self.navigation_enabled()
                    && self.debug.camera.is_none()
                    && !self.is_suspended()
                    && self.campaign.game.player_can_orbit()
                {
                    self.orbit.recenter(&self.campaign.game);
                } else if button == MouseButton::Left
                    && state == ElementState::Pressed
                    && (self.debug.camera.is_some() || self.debug.click_teleport)
                {
                    self.pick_debug_target();
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
                if self.benchmarks.run.is_some() {
                    if code == KeyCode::Escape
                        && event.state == ElementState::Pressed
                        && let Err(error) = self.finish_benchmark(Some("Cancelled by user".into()))
                    {
                        self.pause_error(error);
                    }
                    return;
                }
                if event.state == ElementState::Pressed && code == KeyCode::F7 {
                    self.toggle_randomizer();
                    return;
                }
                if self.randomizer.open {
                    if event.state == ElementState::Pressed
                        && matches!(code, KeyCode::Escape | KeyCode::F1)
                    {
                        self.toggle_randomizer();
                    }
                    return;
                }
                if event.state == ElementState::Pressed && code == KeyCode::F1 {
                    self.controls.close_editor();
                    self.toggle_menu();
                    return;
                }
                if self.menu.open && self.controls.editor.open {
                    if event.state == ElementState::Pressed {
                        self.controls.editor_key(code);
                    }
                    return;
                }
                if event.state == ElementState::Pressed && code == KeyCode::F8 {
                    let previous = self.settings.clone();
                    self.settings.retro = !self.settings.retro;
                    self.apply_settings(&previous);
                    self.update_view();
                    return;
                }
                if event.state == ElementState::Pressed && code == KeyCode::F9 {
                    self.screenshot = true;
                    return;
                }
                if event.state == ElementState::Pressed
                    && code == KeyCode::F6
                    && self.campaign.randomized.is_none()
                    && self.demo.is_none()
                    && self.frontend.is_some()
                    && !self.campaign.playing()
                {
                    self.frontend.as_mut().unwrap().quick_launcher();
                    self.frontend_events(events);
                    return;
                }
                if self.demo_key(code, event.state == ElementState::Pressed) {
                    return;
                }
                if self.menu.open {
                    if self.menu.edit.is_some()
                        || self.view.as_ref().is_some_and(|v| v.overlay.editing())
                    {
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
                        KeyCode::F10 => Some(Item::PhotoMode),
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
                    if code == KeyCode::Escape && self.debug.click_teleport {
                        self.debug.click_teleport = false;
                        self.release_pointer();
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
                if let Some(command) =
                    self.controls
                        .key_command(code, self.campaign.playing(), self.paused)
                {
                    self.command(command, events);
                }
            }

            WindowEvent::RedrawRequested => {
                #[cfg(target_arch = "wasm32")]
                crate::web::prepare_language(
                    &self.assets,
                    self.campaign.options.language.unwrap_or_default(),
                );
                #[cfg(target_arch = "wasm32")]
                if self.randomizer.open {
                    crate::web::prepare_language(
                        &self.assets,
                        self.randomizer.setup.document.rules.language.game(),
                    );
                }
                if let Some(options) = self.benchmarks.pending.take() {
                    let exit = options.exit;
                    if let Err(error) = self.start_benchmark(options) {
                        if exit {
                            self.error = Some(error);
                            events.exit();
                            return;
                        }
                        self.menu.notice = error;
                    }
                }
                let update = self
                    .controls
                    .poll(self.focused && self.benchmarks.run.is_none());
                if update.disconnected && self.campaign.playing() {
                    eprintln!("Active controller disconnected; gameplay paused.");
                    self.set_paused(true);
                } else if self.focused && self.menu.open && !self.controls.editor.open {
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
                self.advance_navigation(elapsed.as_secs_f32());
                if self.focused
                    && self.demo.is_none()
                    && !self.menu.open
                    && !self.randomizer.open
                    && let Some(camera) = &mut self.debug.camera
                {
                    camera.advance(
                        &self.controls,
                        elapsed.as_secs_f32(),
                        f32::from(self.debug.flight_speed) * 1000.0,
                    );
                }
                self.fps_elapsed += elapsed.as_secs_f32();
                let fps_changed = self.fps_elapsed >= 0.5;
                if fps_changed {
                    self.fps = self.fps_frames as f32 / self.fps_elapsed;
                    self.fps_elapsed = 0.0;
                    self.fps_frames = 0;
                }
                let profile = self.settings.show_performance || self.benchmarks.run.is_some();
                let mut timing = performance::CpuFrame {
                    interval: elapsed.as_secs_f32() * 1000.0,
                    ..Default::default()
                };
                let phase = Instant::now();
                let suspended = self.is_suspended();
                if let Some(audio) = &mut self.audio {
                    audio.pause(suspended);
                    audio.poll();
                }
                let before = self.campaign.game.tick;
                if self.benchmarks.run.is_some() {
                    if let Err(error) = self.benchmark_frame() {
                        if self.demo_exit {
                            self.error = Some(error);
                        } else {
                            self.pause_error(error);
                        }
                    }
                } else if self.demo.is_some() {
                    self.advance_demo(elapsed);
                } else if self.frontend_visible() {
                    self.frontend
                        .as_mut()
                        .unwrap()
                        .narration_ready(self.audio.as_ref());
                    if let Err(error) = self.frontend.as_mut().unwrap().advance(
                        elapsed,
                        suspended,
                        &mut self.campaign,
                        &self.assets,
                    ) {
                        self.pause_error(error);
                    }
                    self.frontend_events(events);
                } else {
                    self.advance_simulation(elapsed);
                    self.start_ending(events);
                }
                self.debug.update(&self.campaign.game);
                timing.simulation = phase.elapsed().as_secs_f32() * 1000.0;
                let phase = Instant::now();
                if self.demo.is_some()
                    || before != self.campaign.game.tick
                    || fps_changed && self.settings.show_fps
                    || self.debug.camera.is_some()
                    || self.navigation_enabled() && !suspended
                    || self.settings.graphics.animated_presentation()
                        && !suspended
                        && self.campaign.playing()
                {
                    self.update_view();
                }
                timing.prepare = phase.elapsed().as_secs_f32() * 1000.0;
                let phase = Instant::now();
                self.update_overlay();
                self.frontend_events(events);
                if self.demo_exit
                    || self
                        .demo
                        .as_ref()
                        .is_some_and(|d| d.finished && d.exit_when_finished)
                {
                    events.exit();
                }
                timing.overlay = phase.elapsed().as_secs_f32() * 1000.0;
                #[cfg(not(target_arch = "wasm32"))]
                if self.screenshot {
                    self.save_screenshot();
                }
                if let Some(view) = &mut self.view {
                    #[cfg(target_arch = "wasm32")]
                    if self.screenshot {
                        view.dirty = true;
                    }
                    view.renderer.profiling = profile;
                    #[cfg(not(target_arch = "wasm32"))]
                    let target = self.benchmarks.run.as_ref().and_then(|r| r.target.as_ref());
                    #[cfg(not(target_arch = "wasm32"))]
                    let drawn = if let Some(target) = target {
                        view.benchmark_draw(target, &self.campaign.game, &self.debug)
                    } else {
                        view.draw(&self.campaign.game, &self.debug)
                    };
                    #[cfg(target_arch = "wasm32")]
                    let drawn = view.draw(&self.campaign.game, &self.debug);
                    match drawn {
                        Ok(Some(draw)) => {
                            timing.draw = draw;
                            self.fps_frames += 1;
                            if let Some(run) = &mut self.benchmarks.run {
                                run.record(timing, view.renderer.timings.as_ref());
                            }
                        }
                        Ok(None) => (),
                        Err(error) => {
                            self.error = Some(error);
                            events.exit();
                        }
                    }
                    self.performance.record(
                        timing,
                        view.renderer.timings.as_ref().and_then(|t| t.latest),
                    );
                }
                #[cfg(target_arch = "wasm32")]
                if self.screenshot {
                    self.screenshot = false;
                    crate::web::download_frame();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _events: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        let events = _events;
        #[cfg(not(target_arch = "wasm32"))]
        if self.exporting_video() {
            if self.video.is_some() {
                match self.export_video_frame() {
                    Ok(true) => events.exit(),
                    Ok(false) => (),
                    Err(error) => {
                        self.error = Some(error);
                        events.exit();
                    }
                }
            }
            return;
        }
        if let Some(view) = &self.view {
            view.window.request_redraw();
        }
    }
}

#[derive(Default)]
pub struct Startup {
    pub quick_launcher: bool,
    pub difficulty: Option<tenchu_core::ai::Difficulty>,
    pub benchmark: Option<benchmark::Options>,
}

pub fn run(
    assets: GameAssets<DiscSource>,
    mut options: Options,
    disc: &std::path::Path,
    recording_directory: Option<PathBuf>,
    demo: Option<crate::demo::Player>,
    startup: Startup,
) -> Result<()> {
    let storage = Storage::new()?;
    let settings_store = settings::Store::new()?;
    let presets = settings::library::Manager::new(crate::storage::UserFile::Presets.path()?);
    let (mut settings, notice) = match settings_store.load() {
        Ok(settings) => (settings, String::new()),
        Err(error) => {
            eprintln!("Settings: {error}. Using defaults.");
            (
                Settings::default(),
                "Settings file unreadable; using defaults".into(),
            )
        }
    };
    #[cfg(target_arch = "wasm32")]
    {
        settings.fullscreen = false;
    }
    if let Some(directory) = std::env::var_os("TENCHU_MODEL_DIR") {
        settings.model_directory = directory.to_string_lossy().into_owned();
    }
    let model_directory = settings.model_directory.clone();
    if demo.as_ref().is_some_and(|d| d.muted) {
        settings.volume = 0;
    }
    let saved = if demo.is_some() {
        None
    } else {
        storage.load()?
    };
    if demo.is_none()
        && let Some(difficulty) = startup
            .difficulty
            .or_else(|| saved.as_ref().map(|s| s.selection.difficulty))
    {
        options.difficulty = difficulty;
    }
    let mut campaign = Campaign::new(&assets, options, saved)?;
    campaign.game.roster.keep_all_active = settings.keep_enemies_active;
    campaign.game.intelligence.sight_percent = settings.sight_percent;
    let frontend = if demo.is_none() && startup.benchmark.is_none() {
        campaign.frontend = true;
        campaign.quick_launcher = false;
        if let Some(save) = &campaign.saved {
            campaign.progress = save.progress.clone();
            campaign.seed = save.seed;
        }
        campaign.screen = Screen::Frontend;
        let mut frontend =
            crate::frontend::Frontend::new(&assets, disc, &campaign, settings.frontend.clone())?;
        settings.frontend = frontend.preferences.clone();
        if startup.quick_launcher {
            campaign.quick_launcher = true;
            campaign.screen = Screen::Launcher;
            frontend.quick_launcher();
        } else {
            frontend.play_movie(tenchu_assets::frontend::Movie::Opening, &assets)?;
        }
        Some(frontend)
    } else if demo
        .as_ref()
        .is_some_and(|d| d.catalog.iter().any(|clip| clip.frontend.is_some()))
    {
        Some(crate::frontend::Frontend::new(
            &assets,
            disc,
            &campaign,
            settings.frontend.clone(),
        )?)
    } else {
        None
    };
    let debug = crate::debug::Debug::new(&campaign.game)?;
    let mut audio = if demo.as_ref().is_some_and(|d| d.video.is_some()) {
        None
    } else {
        match crate::audio::Audio::new(disc, &assets, &campaign) {
            Ok(audio) => Some(audio),
            Err(error) => {
                eprintln!("Audio unavailable: {error}");
                None
            }
        }
    };
    if let Some(audio) = &mut audio {
        audio.volume(settings.volume);
        audio.preferences(&settings.frontend);
    }
    let mut application = Application {
        assets,
        campaign,
        storage,
        view: None,
        clock: GameClock::default(),
        last_frame: Instant::now(),
        controls: Controls::new(),
        orbit: Default::default(),
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
            model_directory,
            ..Menu::default()
        },
        pointer: [0.0; 2],
        fps: 0.0,
        performance: Default::default(),
        recent_inputs: Default::default(),
        benchmarks: benchmark::Benchmarks::new(startup.benchmark),
        presets,
        randomizer: crate::randomizer::Editor::new(crate::storage::UserFile::Randomizer.path()?),
        fps_elapsed: 0.0,
        fps_frames: 0,
        debug,
        screenshot: false,
        demo,
        demo_exit: false,
        #[cfg(not(target_arch = "wasm32"))]
        video: None,
        modifiers: ModifiersState::empty(),
        frontend,
    };
    if application.exporting_video() {
        println!(
            "Offline 4K/60 export. Every frame is rendered; host speed only affects completion time. Esc cancels."
        );
    } else if application.demo.is_some() {
        println!(
            "Showcase: Space pauses, Left/Right change clip, Home restarts, Tab opens the picker, H toggles captions, V toggles controls, F1 inspects temporary settings, F9 saves a PNG, Esc exits. Saves and preferences are unchanged."
        );
    } else {
        println!(
            "Default keyboard: WASD/arrows move, Space jump, Shift crouch, J attack, E item, Q look, Z/C item selection, M/Tab map. P pause, F5 retry, Esc inventory.\nDefault controller: D-pad/left stick move, south jump, west attack, east/R1 crouch, north item, L1 look, L2/R2 items, Select map, Start pause. Menus: south confirm, east back, west retry.\nRemap keyboard/controller inputs in F1 > General > Controls. Saved bindings override these defaults.\nGameplay is still in development; an unsupported action pauses with a diagnostic."
        );
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let window = View::browser_window(&event_loop)?;
        wasm_bindgen_futures::spawn_local(async move {
            let result: Result<()> = async {
                let mut view =
                    View::from_window(window, &application.campaign.game, &application.settings)
                        .await?;
                if !view
                    .renderer
                    .sample_counts
                    .contains(&application.settings.graphics.samples)
                {
                    application.settings.graphics.samples = 1;
                }
                view.settings(&application.settings, &application.campaign.game)?;
                application.view = Some(view);
                crate::web::ready();
                event_loop.spawn_app(application);
                Ok(())
            }
            .await;
            if let Err(error) = result {
                crate::web::error(&error);
            }
        });
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let result = event_loop
            .run_app(&mut application)
            .map_err(|e| e.to_string());
        application.finish_recording();
        result?;
        application.error.map_or(Ok(()), Err)
    }
}
