use crate::campaign::Action;
use gilrs::{Button, EventType, Gilrs};
use std::{collections::HashSet, mem, path::PathBuf};
use tenchu_core::input;
use winit::keyboard::KeyCode;
mod bindings;
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

pub fn menu_labels(circle_confirm: bool) -> (&'static str, &'static str) {
    if circle_confirm {
        ("East", "South")
    } else {
        ("South", "East")
    }
}

pub fn gamepad_command(
    pressed: u16,
    playing: bool,
    paused: bool,
    circle_confirm: bool,
) -> Option<Command> {
    let (confirm, back) = if circle_confirm {
        (input::CIRCLE, input::CROSS)
    } else {
        (input::CROSS, input::CIRCLE)
    };
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
        } else if buttons != 0 {
            self.active = Some(id);
        }
        (buttons & !self.menu_blocked) & !before
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
    pad: Pad,
    rumble: Option<gilrs::ff::Effect>,
    pub(super) vibration_error: Option<String>,
}

impl Controls {
    pub fn diagnostic_settings(&self) -> serde_json::Value {
        let controllers: Vec<_> = self
            .gilrs
            .iter()
            .flat_map(|g| g.gamepads())
            .map(|(_, pad)| pad.name().to_owned())
            .collect();
        let details: Vec<_> = self
            .gilrs
            .iter()
            .flat_map(|g| g.gamepads())
            .map(|(id, pad)| {
                serde_json::json!({
                    "name": pad.name(),
                    "active": self.pad.active == Some(usize::from(id)),
                    "vendor_id": pad.vendor_id(),
                    "product_id": pad.product_id(),
                    "mapping": format!("{:?}", pad.mapping_source()),
                    "power": format!("{:?}", pad.power_info()),
                    "vibration_supported": pad.is_ff_supported(),
                })
            })
            .collect();
        serde_json::json!({
            "bindings": self.bindings, "controllers": controllers,
            "controller_details": details, "controller_backend_available": self.gilrs.is_some(),
            "vibration_error": self.vibration_error,
        })
    }

    pub fn held(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),
            Err(error) => {
                eprintln!("Controller input unavailable: {error}");
                None
            }
        };
        let mut controls = Self {
            gilrs,
            ..Self::default()
        };
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
        controls
    }

    fn save_bindings(&mut self) {
        self.clear();
        if let Some(gilrs) = &self.gilrs
            && let Some((_, gamepad)) = gilrs
                .gamepads()
                .find(|(id, _)| Some(usize::from(*id)) == self.pad.active)
        {
            self.pad.raw = self
                .bindings
                .controller(|button| gamepad.is_pressed(button), self.pad.stick);
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
        keyboard_command(code, playing)
            .or_else(|| gamepad_command(self.bindings.key(code) as u16, playing, paused, false))
    }

    pub fn key(&mut self, code: KeyCode, pressed: bool) -> bool {
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
        let buttons = self.touch.buttons
            | mem::take(&mut self.touch.pending)
            | self.pad.buttons()
            | mem::take(&mut self.pad.pending)
            | mem::take(&mut self.pending_keys);
        self.keys
            .iter()
            .fold(buttons, |buttons, key| buttons | self.bindings.key(*key)) as u16
    }

    pub fn relative_buttons(&mut self) -> u16 {
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
        let Some(gilrs) = &mut self.gilrs else {
            return update;
        };
        let mut captured = None;
        while let Some(event) = gilrs.next_event() {
            let id = usize::from(event.id);
            if event.event == EventType::Disconnected {
                if self.pad.disconnect(id) {
                    update.disconnected = true;
                    self.rumble = None;
                }
                continue;
            }
            if !matches!(
                event.event,
                EventType::ButtonPressed(..)
                    | EventType::ButtonReleased(..)
                    | EventType::ButtonChanged(..)
                    | EventType::AxisChanged(..)
            ) {
                continue;
            }
            if self.pad.active.is_some_and(|active| active != id) {
                continue;
            }
            let gamepad = gilrs.gamepad(event.id);
            let before = self.pad.active;
            let previous_stick = self.pad.stick;
            self.pad.axes = std::array::from_fn(|i| {
                let value = gamepad.value(StickAxis::ALL[i].axis());
                if value.is_finite() {
                    value.clamp(-1.0, 1.0)
                } else {
                    0.0
                }
            });
            for (blocked, axes) in self
                .pad
                .axes_blocked
                .iter_mut()
                .zip(self.pad.axes.chunks_exact(2))
            {
                *blocked &= axes.iter().any(|v| v.abs() > 0.05);
            }
            if focused && self.pad.axes.iter().any(|v| v.abs() > 0.05) {
                self.pad.active = Some(id);
            }
            if focused
                && !self.editor.open
                && matches!(event.event, EventType::ButtonPressed(Button::RightThumb, _))
            {
                self.pad.recenter = true;
                self.pad.active = Some(id);
            }
            let previous_direct = self.pad.direct & !self.pad.direct_blocked;
            self.pad.direct = self
                .bindings
                .controller(|button| gamepad.is_pressed(button), [0; 4]);
            self.pad.direct_blocked &= self.pad.direct;
            self.pad.direct_pending |=
                (self.pad.direct & !self.pad.direct_blocked) & !previous_direct;
            self.pad.stick =
                std::array::from_fn(|i| direction(self.pad.axes[i], previous_stick[i]));
            if focused && self.editor.open && captured.is_none() {
                captured = match event.event {
                    EventType::ButtonPressed(button, _) if button != Button::Unknown => {
                        Some(PadInput::Button(button))
                    }
                    EventType::AxisChanged(axis, _, _) => {
                        StickAxis::ALL.iter().enumerate().find_map(|(i, &stick)| {
                            (stick.axis() == axis
                                && self.pad.stick[i] != 0
                                && self.pad.stick[i] != previous_stick[i])
                                .then_some(PadInput::Stick {
                                    axis: stick,
                                    positive: self.pad.stick[i] > 0,
                                })
                        })
                    }
                    _ => None,
                };
                if captured.is_some() {
                    self.pad.active = Some(id);
                }
            }
            let buttons = self
                .bindings
                .controller(|button| gamepad.is_pressed(button), self.pad.stick);
            update.pressed |= self.pad.sample(id, buttons, focused) as u16;
            let menu_buttons =
                bindings::menu_buttons(|button| gamepad.is_pressed(button), self.pad.stick);
            update.menu_pressed |= self.pad.sample_menu(id, menu_buttons, focused);
            if before != self.pad.active {
                println!("Controller: {}", gamepad.name());
            }
        }
        gilrs.inc();
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
