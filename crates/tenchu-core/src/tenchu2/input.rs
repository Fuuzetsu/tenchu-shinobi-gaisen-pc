use crate::input as ps1;
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::commands::Command;

pub const UP: u16 = ps1::UP.swap_bytes();
pub const RIGHT: u16 = ps1::RIGHT.swap_bytes();
pub const DOWN: u16 = ps1::DOWN.swap_bytes();
pub const LEFT: u16 = ps1::LEFT.swap_bytes();
pub const ATTACK: u16 = ps1::SQUARE.swap_bytes();
pub const JUMP: u16 = ps1::CROSS.swap_bytes();
pub const GUARD: u16 = ps1::CIRCLE.swap_bytes();
pub const ITEM: u16 = ps1::TRIANGLE.swap_bytes();
pub const CROUCH: u16 = ps1::R1.swap_bytes();

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pad {
    pub held: u16,
    pub previous: u16,
    pub time: i16,
    pub history: [u16; 7],
}

impl Pad {
    pub fn update(&mut self, input: u16) {
        if self.held == input || input == 0 {
            self.time = self.time.wrapping_add(1);
            if self.time == 8 {
                self.history.fill(0);
            }
        } else {
            self.history.copy_within(0..6, 1);
            self.history[0] = input;
            self.time = 0;
        }
        self.previous = self.held;
        self.held = input;
    }

    pub fn pressed(&self, button: u16) -> bool {
        self.held & !self.previous & button != 0
    }

    pub fn consume(&mut self, commands: &[Command]) -> Option<i16> {
        if self.history[0] == 0 {
            return None;
        }
        let command = commands
            .iter()
            .find(|command| self.history.starts_with(&command.inputs))?;
        self.history[0] = 0;
        Some(command.action)
    }

    pub fn issue(&mut self, commands: &[Command], action: i16) -> u16 {
        if let Some(command) = commands.iter().find(|command| command.action == action) {
            for &input in command.inputs.iter().rev() {
                self.update(0);
                self.update(input);
            }
        }
        self.held
    }
}
