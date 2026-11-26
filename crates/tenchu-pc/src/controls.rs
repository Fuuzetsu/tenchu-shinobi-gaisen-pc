use crate::campaign::Action;
use backend::{Change, Kind};
use gilrs::{Button, Gilrs};
use std::{collections::HashSet, mem, path::PathBuf};
use tenchu_core::input;
use winit::keyboard::KeyCode;
mod backend;
mod bindings;
mod diagnostic;
pub use bindings::{SHEATHE, WALK};
mod editor;
mod rumble;
use bindings::{Bindings, PadInput, StickAxis};

pub enum Command {
    Pause,
    Menu(Action),
}

pub fn keyboard_command(code: KeyCode, playing: bool) -> Option<Command> {
    Some(match code {
        KeyCode::Escape => Command::Menu(Action::Back),
        KeyCode::F5 => Command::Menu(Action::Replay),
        _ if !playing => Command::Menu(match code {
            KeyCode::Enter => Action::Confirm,
            KeyCode::Space => Action::Start,
            KeyCode::ArrowUp => Action::Up,
            KeyCode::ArrowDown => Action::Down,
            KeyCode::ArrowLeft => Action::Left,
            KeyCode::ArrowRight => Action::Right,
            KeyCode::KeyR => Action::Replay,
            _ => return None,
        }),
        _ => return None,
    })
}

pub fn menu_labels(buttons: crate::settings::MenuButtons) -> (&'static str, &'static str) {
    buttons.positions()
}

pub fn gamepad_command(
    pressed: u16,
    playing: bool,
    paused: bool,
    menu_buttons: crate::settings::MenuButtons,
) -> Option<Command> {
    let (confirm, back) = menu_buttons.buttons();
    if playing && pressed & input::START != 0 {
        return Some(Command::Pause);
    }
    if playing && !paused {
        return None;
    }
    if pressed & back != 0 {
        return Some(Command::Menu(Action::Back));
    }
    if pressed & input::SQUARE != 0 {
        return Some(Command::Menu(Action::Replay));
    }
    if !playing && pressed & input::START != 0 {
        return Some(Command::Menu(Action::Start));
    }
    if pressed & (confirm | input::START) != 0 {
        return Some(if playing {
            Command::Pause
        } else {
            Command::Menu(Action::Confirm)
        });
    }
    if !playing {
        for (button, action) in [
            (input::UP, Action::Up),
            (input::DOWN, Action::Down),
            (input::LEFT, Action::Left),
            (input::RIGHT, Action::Right),
        ] {
            if pressed & button != 0 {
                return Some(Command::Menu(action));
            }
        }
    }
    None
}

#[derive(Default)]
struct Pad {
    active: Option<usize>,
    raw: u32,
    blocked: u32,
    pending: u32,
    menu_raw: u16,
    menu_blocked: u16,
    menu_pending: u16,
    stick: [i8; 4],
    axes: [f32; 4],
    axes_blocked: [bool; 2],
    direct: u32,
    direct_pending: u32,
    direct_blocked: u32,
    recenter: bool,
}

fn direction(value: f32, previous: i8) -> i8 {
    if value >= 0.55 || previous == 1 && value > 0.4 {
        1
    } else if value <= -0.55 || previous == -1 && value < -0.4 {
        -1
    } else {
        0
    }
}

impl Pad {
    fn buttons(&self) -> u32 {
        self.raw & !self.blocked
    }

    fn suppress(&mut self) {
        self.blocked = self.raw;
        self.pending = 0;
        self.menu_blocked = self.menu_raw;
        self.menu_pending = 0;
        self.direct_blocked = self.direct;
        self.direct_pending = 0;
        self.recenter = false;
        for (blocked, axes) in self.axes_blocked.iter_mut().zip(self.axes.chunks_exact(2)) {
            *blocked |= axes.iter().any(|v| v.abs() > 0.05);
        }
    }

    fn sample(&mut self, id: usize, buttons: u32, focused: bool) -> u32 {
        if self.active.is_some_and(|active| active != id) || self.active.is_none() && !focused {
            return 0;
        }
        let before = self.buttons();
        self.raw = buttons;
        if self.raw != 0 {
            self.active = Some(id);
        }
        self.blocked &= self.raw;
        if !focused {
            self.suppress();
        }
        let pressed = self.buttons() & !before;
        self.pending |= pressed;
        pressed
    }

    fn disconnect(&mut self, id: usize) -> bool {
        if self.active != Some(id) {
            return false;
        }
        *self = Self::default();
        true
    }

    fn sample_menu(&mut self, id: usize, buttons: u16, focused: bool) -> u16 {
        if self.active.is_some_and(|active| active != id) || self.active.is_none() && !focused {
            return 0;
        }
        let before = self.menu_raw & !self.menu_blocked;
        self.menu_raw = buttons;
        self.menu_blocked &= buttons;
        if !focused {
            self.menu_blocked = buttons;
            self.menu_pending = 0;
        } else if buttons != 0 {
            self.active = Some(id);
        }
        let pressed = (buttons & !self.menu_blocked) & !before;
        self.menu_pending |= pressed;
        pressed
    }
}

#[derive(Default)]
pub struct Update {
    pub pressed: u16,
    pub menu_pressed: u16,
    pub disconnected: bool,
}

#[derive(Default)]
struct TouchInput {
    sample: crate::touch::Sample,
    directions: [i8; 2],
    raw: u32,
    blocked: u32,
    axes_blocked: [bool; 2],
    buttons: u32,
    pending: u32,
    pressed: u16,
}

impl TouchInput {
    fn suppress(&mut self) {
        self.blocked |= self.raw;
        self.buttons = 0;
        self.pending = 0;
        self.pressed = 0;
        for (blocked, stick) in self.axes_blocked.iter_mut().zip(self.sample.sticks) {
            *blocked |= stick.length_squared() > 0.0025;
        }
    }
}

#[derive(Default)]
pub struct Controls {
    touch: TouchInput,
    bindings: Bindings,
    pub editor: editor::Editor,
    bindings_path: Option<PathBuf>,
    keys: HashSet<KeyCode>,
    pending_keys: u32,
    gilrs: Option<Gilrs>,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    sdl: Option<backend::sdl::Backend>,
    backend: Kind,
    backend_error: Option<String>,
    selected_device: Option<usize>,
    controller_history: diagnostic::History,
    report_requested: bool,
    test_keys: HashSet<KeyCode>,
    test_keyboard_active: bool,
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    sdl_rumble: Option<rumble::SdlRumble>,
    pad: Pad,
    rumble: Option<gilrs::ff::Effect>,
    native_rumble: Option<(Option<usize>, [u16; 2])>,
    pub(super) vibration_error: Option<String>,
}

impl Controls {
    pub fn held(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    pub fn new() -> Self {
        let mut controls = Self::default();
        let load = || {
            let path = crate::storage::UserFile::Controls.path()?;
            let bindings = match crate::storage::read(&path) {
                Ok(bytes) => Bindings::decode(&bytes)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Bindings::default(),
                Err(e) => return Err(e.to_string()),
            };
            Ok((path, bindings))
        };
        match load() {
            Ok((path, bindings)) => {
                controls.bindings_path = Some(path);
                controls.bindings = bindings;
            }
            Err(error) => {
                controls.editor.notice =
                    format!("Could not load controls; using defaults: {error}");
                eprintln!("{}", controls.editor.notice);
                controls.bindings_path = crate::storage::UserFile::Controls.path().ok();
            }
        }
        let kind = crate::storage::UserFile::ControllerBackend
            .path()
            .ok()
            .and_then(|path| crate::storage::read(&path).ok())
            .and_then(|bytes| serde_json::from_slice::<Kind>(&bytes).ok())
            .filter(|kind| Kind::choices().contains(kind))
            .unwrap_or_default();
        let kind = std::env::var("TENCHU_CONTROLLER_DRIVER")
            .ok()
            .and_then(|name| Kind::parse(&name))
            .unwrap_or(kind);
        controls.start_backend(kind);
        controls
    }

    fn save_bindings(&mut self) {
        self.clear();
        if let Some(reading) = self.current_reading() {
            self.pad.raw = self
                .bindings
                .controller(|b| reading.pressed(b), self.pad.stick);
            self.pad.direct = self.bindings.controller(|b| reading.pressed(b), [0; 4]);
            self.pad.suppress();
        }
        let save = || {
            let path = self
                .bindings_path
                .as_ref()
                .ok_or("No controls storage available")?;
            let bytes = serde_json::to_vec_pretty(&self.bindings).map_err(|e| e.to_string())?;
            crate::storage::atomic_write(path, &bytes)
        };
        self.editor.notice = match save() {
            Ok(()) => "Saved. Release the new input before continuing.".into(),
            Err(error) => {
                format!("Bindings apply for this session, but could not be saved: {error}")
            }
        };
    }

    pub fn key_command(&self, code: KeyCode, playing: bool, paused: bool) -> Option<Command> {
        keyboard_command(code, playing).or_else(|| {
            gamepad_command(
                self.bindings.key(code) as u16,
                playing,
                paused,
                Default::default(),
            )
        })
    }

    pub fn key(&mut self, code: KeyCode, pressed: bool) -> bool {
        if self.keys.contains(&code) != pressed {
            self.controller_history
                .keyboard(self.bindings.key(code), pressed);
        }
        if pressed {
            let changed = self.keys.insert(code);
            if changed {
                self.pending_keys |= self.bindings.key(code);
            }
            changed
        } else {
            self.keys.remove(&code);
            false
        }
    }

    pub fn touch(&mut self, sample: crate::touch::Sample) {
        for (blocked, stick) in self.touch.axes_blocked.iter_mut().zip(sample.sticks) {
            *blocked &= stick.length_squared() > 0.4 * 0.4;
        }
        self.touch.directions =
            std::array::from_fn(|i| direction(sample.sticks[0][i], self.touch.directions[i]));
        let stick = if self.touch.axes_blocked[0] {
            glam::Vec2::ZERO
        } else {
            glam::Vec2::from_array(self.touch.directions.map(f32::from))
        };
        self.touch.raw = sample.buttons | u32::from(crate::navigation::directions(stick));
        self.touch.blocked &= self.touch.raw;
        let buttons = self.touch.raw & !self.touch.blocked;
        let pressed = buttons & !self.touch.buttons;
        self.touch.pending |= pressed;
        self.touch.pressed |= pressed as u16;
        self.touch.buttons = buttons;
        self.touch.sample = sample;
    }

    pub fn clear(&mut self) {
        self.test_keys.clear();
        self.touch.suppress();
        self.stop_rumble();
        self.keys.clear();
        self.pending_keys = 0;
        self.pad.suppress();
    }

    pub fn bandage(&self) -> bool {
        (self.pad.buttons()
            | self.pad.pending
            | self.pending_keys
            | self.touch.buttons
            | self.touch.pending)
            & bindings::BANDAGE
            != 0
            || self
                .keys
                .iter()
                .any(|&key| self.bindings.key(key) & bindings::BANDAGE != 0)
    }

    pub fn take_style_switch(&mut self) -> bool {
        let mut pressed = false;
        for pending in [
            &mut self.pending_keys,
            &mut self.pad.pending,
            &mut self.pad.direct_pending,
            &mut self.touch.pending,
        ] {
            pressed |= *pending & bindings::NEXT_STYLE != 0;
            *pending &= !bindings::NEXT_STYLE;
        }
        pressed
    }

    pub fn bandage_label(&self) -> String {
        let row = &self.bindings.0[&bindings::Target::Bandage];
        if self.pad.active.is_some()
            && let Some(button) = row.controller.iter().flatten().next()
        {
            return button.label();
        }
        row.keys.iter().flatten().next().map_or_else(
            || "F1 → Bleeding".into(),
            |key| format!("{key:?}").trim_start_matches("Key").to_owned(),
        )
    }

    pub fn buttons(&mut self) -> u16 {
        self.pad.menu_pending = 0;
        let buttons = self.touch.buttons
            | mem::take(&mut self.touch.pending)
            | self.pad.buttons()
            | mem::take(&mut self.pad.pending)
            | mem::take(&mut self.pending_keys);
        self.keys
            .iter()
            .fold(buttons, |buttons, key| buttons | self.bindings.key(*key)) as u16
    }

    /// Physical menu buttons, mapped to the Japanese screen's Circle-confirm convention.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn japanese_menu_buttons(&mut self, menu_buttons: crate::settings::MenuButtons) -> u16 {
        let physical =
            (self.pad.menu_raw & !self.pad.menu_blocked) | mem::take(&mut self.pad.menu_pending);
        self.pad.pending = 0;
        self.pad.direct_pending = 0;
        let keyboard_touch = self.touch.buttons
            | mem::take(&mut self.touch.pending)
            | mem::take(&mut self.pending_keys);
        let buttons = physical
            | self.keys.iter().fold(keyboard_touch, |buttons, key| {
                buttons | self.bindings.key(*key)
            }) as u16;
        menu_buttons.japanese(buttons)
    }

    pub fn relative_buttons(&mut self) -> u16 {
        self.pad.menu_pending = 0;
        self.pad.pending = 0;
        let directions = u32::from(input::UP | input::DOWN | input::LEFT | input::RIGHT);
        let buttons = (self.touch.sample.buttons & !self.touch.blocked)
            | (mem::take(&mut self.touch.pending) & !directions)
            | (self.pad.direct & !self.pad.direct_blocked)
            | mem::take(&mut self.pad.direct_pending)
            | mem::take(&mut self.pending_keys);
        self.keys
            .iter()
            .fold(buttons, |bits, key| bits | self.bindings.key(*key)) as u16
    }

    pub fn walking_buttons(&mut self, deadzone: u16) -> (u16, f32) {
        let pending =
            (self.pad.buttons() | self.pad.pending | self.touch.buttons | self.touch.pending)
                as u16;
        let directions = input::UP | input::DOWN | input::LEFT | input::RIGHT;
        let buttons = self.relative_buttons() | (pending & !directions);
        if buttons & directions != 0 {
            return (buttons, 1.0);
        }
        let analog = self.bindings.movement_stick(self.sticks(), deadzone);
        (
            buttons | crate::navigation::directions(analog.normalize_or_zero()),
            analog.length().min(1.0),
        )
    }

    pub fn touch_buttons(&self) -> u16 {
        (self.touch.sample.buttons & !self.touch.blocked) as u16
    }

    pub fn sticks(&self) -> [glam::Vec2; 2] {
        std::array::from_fn(|i| {
            if !self.touch.axes_blocked[i]
                && self.touch.sample.sticks[i] != glam::Vec2::ZERO
                && !self.editor.open
            {
                self.touch.sample.sticks[i]
            } else if self.pad.axes_blocked[i] || self.editor.open {
                glam::Vec2::ZERO
            } else {
                glam::Vec2::new(self.pad.axes[i * 2], self.pad.axes[i * 2 + 1])
            }
        })
    }

    pub fn take_recenter(&mut self) -> bool {
        mem::take(&mut self.pad.recenter)
    }

    pub fn poll(&mut self, focused: bool) -> Update {
        let pressed = mem::take(&mut self.touch.pressed);
        let mut update = Update {
            pressed,
            menu_pressed: pressed,
            disconnected: false,
        };
        if !focused {
            self.touch.suppress();
            update = Update::default();
            self.pad.suppress();
            self.stop_rumble();
        }
        self.controller_history.poll(focused);
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        self.update_sdl_rumble();
        let mut captured = None;
        while let Some(event) = self.next_controller_event() {
            self.controller_history.event(&event);
            let id = event.id;
            if matches!(event.change, Change::Connected | Change::Disconnected) {
                self.controller_history.devices(self.devices());
            }
            if matches!(event.change, Change::Disconnected) {
                if self.pad.disconnect(id) {
                    update.disconnected = true;
                    self.stop_rumble();
                }
                continue;
            }
            if self.selected_device.is_some_and(|selected| selected != id)
                || self.pad.active.is_some_and(|active| active != id)
            {
                continue;
            }
            let reading = event.reading;
            // Ignore idle drift when choosing between physical/virtual devices.
            if self.pad.active.is_none() {
                let deliberate = matches!(event.change, Change::Button(b, true) if b != Button::Unknown)
                    || reading.axes.iter().any(|v| v.abs() >= 0.55);
                if !focused || !deliberate {
                    continue;
                }
                self.pad.active = Some(id);
            }
            let previous_stick = self.pad.stick;
            self.pad.axes = reading.axes;
            for (blocked, axes) in self
                .pad
                .axes_blocked
                .iter_mut()
                .zip(self.pad.axes.chunks_exact(2))
            {
                *blocked &= axes.iter().any(|v| v.abs() > 0.05);
            }
            if focused
                && !self.editor.open
                && matches!(event.change, Change::Button(Button::RightThumb, true))
            {
                self.pad.recenter = true;
            }
            let previous_direct = self.pad.direct & !self.pad.direct_blocked;
            self.pad.direct = self.bindings.controller(|b| reading.pressed(b), [0; 4]);
            self.pad.direct_blocked &= self.pad.direct;
            self.pad.direct_pending |=
                (self.pad.direct & !self.pad.direct_blocked) & !previous_direct;
            self.pad.stick =
                std::array::from_fn(|i| direction(self.pad.axes[i], previous_stick[i]));
            if focused && self.editor.open && captured.is_none() {
                captured = match event.change {
                    Change::Button(button, true) if button != Button::Unknown => {
                        Some(PadInput::Button(button))
                    }
                    Change::Axis(i)
                        if self.pad.stick[i] != 0 && self.pad.stick[i] != previous_stick[i] =>
                    {
                        Some(PadInput::Stick {
                            axis: StickAxis::ALL[i],
                            positive: self.pad.stick[i] > 0,
                        })
                    }
                    _ => None,
                };
            }
            let buttons = self
                .bindings
                .controller(|b| reading.pressed(b), self.pad.stick);
            update.pressed |= self.pad.sample(id, buttons, focused) as u16;
            let menu_buttons = bindings::menu_buttons(|b| reading.pressed(b), self.pad.stick);
            update.menu_pressed |= self.pad.sample_menu(id, menu_buttons, focused);
        }
        if let Some(gilrs) = &mut self.gilrs {
            gilrs.inc();
        }
        if let Some(input) = captured {
            self.editor_button(input);
        }
        if self.editor.open {
            self.pad.suppress();
            update.pressed = 0;
            update.menu_pressed = 0;
        }
        update
    }
}

#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn check_controllers(selected: Option<&str>) -> Result<(), String> {
    let selection = selected
        .map(|name| {
            Kind::parse(name)
                .ok_or("Choose an available controller backend: gilrs, sdl, sdl-xinput, sdl-wgi")
        })
        .transpose()?;
    let mut controls = Controls::default();
    let mut failures = Vec::new();
    // Exercise teardown/reinitialization too, without a window or a disc.
    for _ in 0..2 {
        for &kind in Kind::choices()
            .iter()
            .filter(|&&kind| selection.is_none_or(|selected| selected == kind))
        {
            controls.start_backend(kind);
            for _ in 0..5 {
                controls.poll(false);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if let Some(error) = &controls.backend_error {
                failures.push(format!("{}: {error}", kind.label()));
            }
        }
    }
    let settings = crate::settings::Settings::default();
    let report = crate::diagnostics::report(&settings, &crate::debug::Debug::default(), &controls);
    let bytes = crate::diagnostics::finish(report, &settings, None)?;
    println!("{}", String::from_utf8(bytes).map_err(|e| e.to_string())?);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}
