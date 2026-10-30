use crate::campaign::Action;
use gilrs::{Button, EventType, Gilrs};
use std::{collections::HashSet, mem, path::PathBuf};
use tenchu_core::input;
use winit::keyboard::KeyCode;
mod bindings;
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

pub fn gamepad_command(pressed: u16, playing: bool, paused: bool) -> Option<Command> {
    if playing && pressed & input::START != 0 {
        return Some(Command::Pause);
    }
    if playing && !paused {
        return None;
    }
    if pressed & input::CIRCLE != 0 {
        return Some(Command::Menu(Action::Back));
    }
    if pressed & input::SQUARE != 0 {
        return Some(Command::Menu(Action::Replay));
    }
    if !playing && pressed & input::START != 0 {
        return Some(Command::Menu(Action::Start));
    }
    if pressed & (input::CROSS | input::START) != 0 {
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
    raw: u16,
    blocked: u16,
    pending: u16,
    stick: [i8; 4],
    axes: [f32; 4],
    axes_blocked: [bool; 2],
    direct: u16,
    direct_pending: u16,
    direct_blocked: u16,
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
    fn buttons(&self) -> u16 {
        self.raw & !self.blocked
    }

    fn suppress(&mut self) {
        self.blocked = self.raw;
        self.pending = 0;
        self.direct_blocked = self.direct;
        self.direct_pending = 0;
        self.recenter = false;
        for (blocked, axes) in self.axes_blocked.iter_mut().zip(self.axes.chunks_exact(2)) {
            *blocked |= axes.iter().any(|v| v.abs() > 0.05);
        }
    }

    fn sample(&mut self, id: usize, buttons: u16, focused: bool) -> u16 {
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
}

#[derive(Default)]
pub struct Update {
    pub pressed: u16,
    pub disconnected: bool,
}

#[derive(Default)]
pub struct Controls {
    bindings: Bindings,
    pub editor: editor::Editor,
    bindings_path: Option<PathBuf>,
    keys: HashSet<KeyCode>,
    pending_keys: u16,
    gilrs: Option<Gilrs>,
    pad: Pad,
    rumble: Option<gilrs::ff::Effect>,
}

impl Controls {
    pub fn diagnostic_settings(&self) -> serde_json::Value {
        let controllers: Vec<_> = self
            .gilrs
            .iter()
            .flat_map(|g| g.gamepads())
            .map(|(_, pad)| pad.name().to_owned())
            .collect();
        serde_json::json!({"bindings": self.bindings, "controllers": controllers})
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
            .or_else(|| gamepad_command(self.bindings.key(code), playing, paused))
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

    pub fn clear(&mut self) {
        self.stop_rumble();
        self.keys.clear();
        self.pending_keys = 0;
        self.pad.suppress();
    }

    pub fn buttons(&mut self) -> u16 {
        let buttons = self.pad.buttons()
            | mem::take(&mut self.pad.pending)
            | mem::take(&mut self.pending_keys);
        self.keys
            .iter()
            .fold(buttons, |buttons, key| buttons | self.bindings.key(*key))
    }

    pub fn relative_buttons(&mut self) -> u16 {
        self.pad.pending = 0;
        let buttons = (self.pad.direct & !self.pad.direct_blocked)
            | mem::take(&mut self.pad.direct_pending)
            | mem::take(&mut self.pending_keys);
        self.keys
            .iter()
            .fold(buttons, |bits, key| bits | self.bindings.key(*key))
    }

    pub fn sticks(&self) -> [glam::Vec2; 2] {
        std::array::from_fn(|i| {
            if self.pad.axes_blocked[i] || self.editor.open {
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
        let mut update = Update::default();
        if !focused {
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
            update.pressed |= self.pad.sample(id, buttons, focused);
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
        }
        update
    }
}

#[cfg(test)]
mod tests;
