use crate::campaign::Action;
use gilrs::{Axis, Button, EventType, Gilrs};
use std::{collections::HashSet, mem};
use tenchu_core::input;
use winit::keyboard::KeyCode;

const BUTTONS: [(Button, u16); 14] = [
    (Button::South, input::CROSS),
    (Button::East, input::CIRCLE),
    (Button::West, input::SQUARE),
    (Button::North, input::TRIANGLE),
    (Button::LeftTrigger, input::L1),
    (Button::RightTrigger, input::R1),
    (Button::LeftTrigger2, input::L2),
    (Button::RightTrigger2, input::R2),
    (Button::Select, input::SELECT),
    (Button::Start, input::START),
    (Button::DPadUp, input::UP),
    (Button::DPadDown, input::DOWN),
    (Button::DPadLeft, input::LEFT),
    (Button::DPadRight, input::RIGHT),
];

pub enum Command {
    Pause,
    Menu(Action),
}

fn key_button(code: KeyCode) -> u16 {
    match code {
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
}

pub fn keyboard_command(code: KeyCode, playing: bool) -> Option<Command> {
    Some(match code {
        KeyCode::Escape => Command::Menu(Action::Back),
        KeyCode::F5 => Command::Menu(Action::Replay),
        KeyCode::KeyP if playing => Command::Pause,
        _ if !playing => Command::Menu(match code {
            KeyCode::Enter => Action::Confirm,
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
    stick: [i8; 2],
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
    }

    fn sample(&mut self, id: usize, buttons: u16, axes: [f32; 2], focused: bool) -> u16 {
        if self.active.is_some_and(|active| active != id) || self.active.is_none() && !focused {
            return 0;
        }
        let before = self.buttons();
        self.stick = std::array::from_fn(|i| direction(axes[i], self.stick[i]));
        self.raw = buttons
            | match self.stick[0] {
                -1 => input::LEFT,
                1 => input::RIGHT,
                _ => 0,
            }
            | match self.stick[1] {
                -1 => input::DOWN,
                1 => input::UP,
                _ => 0,
            };
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
    keys: HashSet<KeyCode>,
    pending_keys: u16,
    gilrs: Option<Gilrs>,
    pad: Pad,
}

impl Controls {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),
            Err(error) => {
                eprintln!("Controller input unavailable: {error}");
                None
            }
        };
        Self {
            gilrs,
            ..Self::default()
        }
    }

    pub fn key(&mut self, code: KeyCode, pressed: bool) -> bool {
        if pressed {
            let changed = self.keys.insert(code);
            if changed {
                self.pending_keys |= key_button(code);
            }
            changed
        } else {
            self.keys.remove(&code);
            false
        }
    }

    pub fn clear(&mut self) {
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
            .fold(buttons, |buttons, key| buttons | key_button(*key))
    }

    pub fn poll(&mut self, focused: bool) -> Update {
        let mut update = Update::default();
        if !focused {
            self.pad.suppress();
        }
        let Some(gilrs) = &mut self.gilrs else {
            return update;
        };
        while let Some(event) = gilrs.next_event() {
            let id = usize::from(event.id);
            if event.event == EventType::Disconnected {
                update.disconnected |= self.pad.disconnect(id);
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
            let gamepad = gilrs.gamepad(event.id);
            let before = self.pad.active;
            let buttons = BUTTONS.iter().fold(0, |held, &(button, bit)| {
                held | if gamepad.is_pressed(button) { bit } else { 0 }
            });
            update.pressed |= self.pad.sample(
                id,
                buttons,
                [
                    gamepad.value(Axis::LeftStickX),
                    gamepad.value(Axis::LeftStickY),
                ],
                focused,
            );
            if before != self.pad.active {
                println!("Controller: {}", gamepad.name());
            }
        }
        gilrs.inc();
        update
    }
}

#[cfg(test)]
mod tests;
