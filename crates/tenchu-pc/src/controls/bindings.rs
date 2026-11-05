use gilrs::{Axis, Button};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use tenchu_assets::Result;
use tenchu_core::input;
use winit::keyboard::KeyCode;

pub const WALK: u16 = 0x0200;
pub const SHEATHE: u16 = 0x0400;
pub const BANDAGE: u32 = 1 << 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Up,
    Down,
    Left,
    Right,
    Cross,
    Square,
    Circle,
    Triangle,
    L1,
    R1,
    L2,
    R2,
    Select,
    Start,
    Walk,
    Sheathe,
    Bandage,
}

impl Target {
    pub const ALL: [Self; 17] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Cross,
        Self::Square,
        Self::Circle,
        Self::Triangle,
        Self::L1,
        Self::R1,
        Self::L2,
        Self::R2,
        Self::Select,
        Self::Start,
        Self::Walk,
        Self::Sheathe,
        Self::Bandage,
    ];

    pub fn description(self) -> (&'static str, u32) {
        match self {
            Self::Up => ("Forward / up", u32::from(input::UP)),
            Self::Down => ("Backward / down", u32::from(input::DOWN)),
            Self::Left => ("Turn left", u32::from(input::LEFT)),
            Self::Right => ("Turn right", u32::from(input::RIGHT)),
            Self::Cross => ("Jump · Cross", u32::from(input::CROSS)),
            Self::Square => ("Attack · Square", u32::from(input::SQUARE)),
            Self::Circle => ("Crouch · Circle", u32::from(input::CIRCLE)),
            Self::Triangle => ("Use item · Triangle", u32::from(input::TRIANGLE)),
            Self::L1 => ("Look / aim · L1", u32::from(input::L1)),
            Self::R1 => (
                "Crouch / modifier / optional guard · R1",
                u32::from(input::R1),
            ),
            Self::L2 => ("Previous item · L2", u32::from(input::L2)),
            Self::R2 => ("Next item · R2", u32::from(input::R2)),
            Self::Select => ("Map · Select", u32::from(input::SELECT)),
            Self::Start => ("Pause · Start", u32::from(input::START)),
            Self::Walk => ("Walk modifier (optional)", u32::from(WALK)),
            Self::Sheathe => ("Draw / sheathe (optional)", u32::from(SHEATHE)),
            Self::Bandage => ("Use bandage (optional)", BANDAGE),
        }
    }

    fn defaults(self) -> Binding {
        use KeyCode::*;
        let (keys, button, stick) = match self {
            Self::Up => (
                [Some(KeyW), Some(ArrowUp)],
                Button::DPadUp,
                Some((StickAxis::LeftY, true)),
            ),
            Self::Down => (
                [Some(KeyS), Some(ArrowDown)],
                Button::DPadDown,
                Some((StickAxis::LeftY, false)),
            ),
            Self::Left => (
                [Some(KeyA), Some(ArrowLeft)],
                Button::DPadLeft,
                Some((StickAxis::LeftX, false)),
            ),
            Self::Right => (
                [Some(KeyD), Some(ArrowRight)],
                Button::DPadRight,
                Some((StickAxis::LeftX, true)),
            ),
            Self::Cross => ([Some(Space), None], Button::South, None),
            Self::Square => ([Some(KeyJ), None], Button::West, None),
            Self::Circle => ([Some(ShiftLeft), Some(ShiftRight)], Button::East, None),
            Self::Triangle => ([Some(KeyE), None], Button::North, None),
            Self::L1 => ([Some(KeyQ), None], Button::LeftTrigger, None),
            Self::R1 => ([Some(KeyK), None], Button::RightTrigger, None),
            Self::L2 => ([Some(KeyZ), None], Button::LeftTrigger2, None),
            Self::R2 => ([Some(KeyC), None], Button::RightTrigger2, None),
            Self::Select => ([Some(KeyM), Some(Tab)], Button::Select, None),
            Self::Start => ([Some(KeyP), None], Button::Start, None),
            Self::Walk => ([Some(AltLeft), None], Button::LeftThumb, None),
            Self::Sheathe | Self::Bandage => {
                return Binding {
                    keys: [Some(if self == Self::Sheathe { KeyH } else { KeyB }), None],
                    controller: [None; 2],
                };
            }
        };
        Binding {
            keys,
            controller: [
                Some(PadInput::Button(button)),
                stick.map(|(axis, positive)| PadInput::Stick { axis, positive }),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StickAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
}

impl StickAxis {
    pub const ALL: [Self; 4] = [Self::LeftX, Self::LeftY, Self::RightX, Self::RightY];

    pub fn axis(self) -> Axis {
        match self {
            Self::LeftX => Axis::LeftStickX,
            Self::LeftY => Axis::LeftStickY,
            Self::RightX => Axis::RightStickX,
            Self::RightY => Axis::RightStickY,
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&a| a == self).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PadInput {
    Button(Button),
    Stick { axis: StickAxis, positive: bool },
}

impl PadInput {
    pub fn label(self) -> String {
        match self {
            Self::Button(button) => match button {
                Button::South => "South (A / Cross)".into(),
                Button::East => "East (B / Circle)".into(),
                Button::West => "West (X / Square)".into(),
                Button::North => "North (Y / Triangle)".into(),
                Button::LeftTrigger => "LB / L1".into(),
                Button::RightTrigger => "RB / R1".into(),
                Button::LeftTrigger2 => "LT / L2".into(),
                Button::RightTrigger2 => "RT / R2".into(),
                Button::Select => "Back / Select".into(),
                Button::Start => "Menu / Start".into(),
                other => format!("{other:?}"),
            },
            Self::Stick { axis, positive } => {
                let stick = match axis {
                    StickAxis::LeftX | StickAxis::LeftY => "Left",
                    _ => "Right",
                };
                let direction = match (axis, positive) {
                    (StickAxis::LeftX | StickAxis::RightX, false) => "left",
                    (StickAxis::LeftX | StickAxis::RightX, true) => "right",
                    (_, false) => "down",
                    (_, true) => "up",
                };
                format!("{stick} stick {direction}")
            }
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Binding {
    pub keys: [Option<KeyCode>; 2],
    pub controller: [Option<PadInput>; 2],
}

impl Binding {
    fn active(&self, pressed: &impl Fn(Button) -> bool, sticks: [i8; 4]) -> bool {
        self.controller.iter().flatten().any(|input| match *input {
            PadInput::Button(button) => pressed(button),
            PadInput::Stick { axis, positive } => {
                sticks[axis.index()] == if positive { 1 } else { -1 }
            }
        })
    }
}

pub fn menu_buttons(pressed: impl Fn(Button) -> bool, sticks: [i8; 4]) -> u16 {
    Target::ALL
        .into_iter()
        .filter(|target| !matches!(target, Target::Walk | Target::Sheathe | Target::Bandage))
        .fold(0, |held, target| {
            held | if target.defaults().active(&pressed, sticks) {
                target.description().1 as u16
            } else {
                0
            }
        })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bindings(pub BTreeMap<Target, Binding>);

impl Default for Bindings {
    fn default() -> Self {
        Self(Target::ALL.map(|target| (target, target.defaults())).into())
    }
}

impl Bindings {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut bindings: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let mut used_keys: HashSet<_> = bindings
            .0
            .values()
            .flat_map(|row| row.keys.iter().flatten().copied())
            .collect();
        let mut used_buttons: HashSet<_> = bindings
            .0
            .values()
            .flat_map(|row| row.controller.iter().flatten().copied())
            .collect();
        for target in Target::ALL {
            bindings.0.entry(target).or_insert_with(|| {
                let mut row = target.defaults();
                for key in &mut row.keys {
                    if key.is_some_and(|key| !used_keys.insert(key)) {
                        *key = None;
                    }
                }
                for button in &mut row.controller {
                    if button.is_some_and(|button| !used_buttons.insert(button)) {
                        *button = None;
                    }
                }
                row
            });
        }
        let mut keys = HashSet::new();
        let mut buttons = HashSet::new();
        for row in bindings.0.values() {
            for &key in row.keys.iter().flatten() {
                if reserved(key) || !keys.insert(key) {
                    return Err(format!("Reserved or repeated keyboard binding: {key:?}"));
                }
            }
            for &button in row.controller.iter().flatten() {
                if button == PadInput::Button(Button::Unknown) || !buttons.insert(button) {
                    return Err(format!(
                        "Unknown or repeated controller binding: {button:?}"
                    ));
                }
            }
        }
        Ok(bindings)
    }

    pub fn key(&self, code: KeyCode) -> u32 {
        self.0
            .iter()
            .find_map(|(target, row)| {
                row.keys
                    .contains(&Some(code))
                    .then_some(target.description().1)
            })
            .unwrap_or(0)
    }

    pub fn controller(&self, pressed: impl Fn(Button) -> bool, sticks: [i8; 4]) -> u32 {
        self.0.iter().fold(0, |held, (target, row)| {
            let active = row.active(&pressed, sticks);
            held | if active { target.description().1 } else { 0 }
        })
    }

    pub fn movement_stick(&self, sticks: [glam::Vec2; 2], deadzone: u16) -> glam::Vec2 {
        let axes = sticks.map(|s| crate::navigation::stick(s, deadzone));
        let axes = [axes[0].x, axes[0].y, axes[1].x, axes[1].y];
        let value = |target| {
            self.0
                .get(&target)
                .into_iter()
                .flat_map(|row| row.controller.iter().flatten())
                .filter_map(|binding| {
                    let PadInput::Stick { axis, positive } = *binding else {
                        return None;
                    };
                    Some((axes[axis.index()] * if positive { 1.0 } else { -1.0 }).max(0.0))
                })
                .fold(0.0_f32, f32::max)
        };
        glam::Vec2::new(
            value(Target::Right) - value(Target::Left),
            value(Target::Up) - value(Target::Down),
        )
    }

    pub fn bind_key(&mut self, target: Target, slot: usize, key: KeyCode) -> Result<()> {
        if reserved(key) {
            return Err("Escape, Enter, Backspace and F1–F12 are reserved for menus/tools".into());
        }
        for row in self.0.values_mut() {
            for value in &mut row.keys {
                if *value == Some(key) {
                    *value = None;
                }
            }
        }
        self.0.get_mut(&target).unwrap().keys[slot] = Some(key);
        Ok(())
    }

    pub fn bind_controller(&mut self, target: Target, slot: usize, input: PadInput) {
        for row in self.0.values_mut() {
            for value in &mut row.controller {
                if *value == Some(input) {
                    *value = None;
                }
            }
        }
        self.0.get_mut(&target).unwrap().controller[slot] = Some(input);
    }
}

fn reserved(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::Escape
            | KeyCode::Enter
            | KeyCode::Backspace
            | KeyCode::F1
            | KeyCode::F2
            | KeyCode::F3
            | KeyCode::F4
            | KeyCode::F5
            | KeyCode::F6
            | KeyCode::F7
            | KeyCode::F8
            | KeyCode::F9
            | KeyCode::F10
            | KeyCode::F11
            | KeyCode::F12
    )
}
