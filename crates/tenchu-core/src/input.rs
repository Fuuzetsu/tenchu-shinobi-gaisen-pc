use tenchu_assets::executable::Command;

pub const SELECT: u16 = 0x0100;
pub const START: u16 = 0x0800;
pub const UP: u16 = 0x1000;
pub const RIGHT: u16 = 0x2000;
pub const DOWN: u16 = 0x4000;
pub const LEFT: u16 = 0x8000;
pub const L2: u16 = 0x0001;
pub const R2: u16 = 0x0002;
pub const L1: u16 = 0x0004;
pub const R1: u16 = 0x0008;
pub const TRIANGLE: u16 = 0x0010;
pub const CIRCLE: u16 = 0x0020;
pub const CROSS: u16 = 0x0040;
pub const SQUARE: u16 = 0x0080;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub held: u16,
    pub previous: u16,
    pub trigger: u16,
    pub time: i16,
    pub stream: [u16; 4],
    pub hold_counter: u32,
}

impl Pad {
    fn push(&mut self, value: u16) {
        self.stream.copy_within(0..3, 1);
        self.stream[0] = value;
    }

    pub fn update(&mut self, mut input: u16) {
        if self.time < 6 {
            self.time = self.time.wrapping_add(1);
            if self.time < 0 {
                input = 0;
            }
        }
        self.previous = self.held;
        self.held = input;
        self.trigger = input & !self.previous;
        if self.previous != input && input != 0 {
            if self.time > 5 {
                self.stream[0] = 0;
            }
            self.push(input);
            self.time = 0;
        }
    }

    pub fn consume(&mut self, commands: &[Command]) -> u16 {
        for command in commands {
            if self.stream[..command.inputs.len()] == command.inputs {
                self.push(0);
                return command.id;
            }
        }
        0
    }

    pub fn issue(&mut self, commands: &[Command], id: u16) -> u16 {
        if let Some(c) = commands.iter().find(|c| c.id == id) {
            self.stream[..c.inputs.len() - 1].copy_from_slice(&c.inputs[1..]);
            self.time = 1;
            c.inputs[0]
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn releases_expiry_and_source_priority_shape_commands() {
        let commands = [
            Command {
                id: 7,
                inputs: vec![CROSS, CROSS],
            },
            Command {
                id: 8,
                inputs: vec![CROSS],
            },
        ];
        let mut pad = Pad::default();
        pad.update(CROSS);
        pad.update(0);
        pad.update(CROSS);
        assert_eq!(pad.consume(&commands), 7);
        assert_eq!(pad.consume(&commands), 0);
        for _ in 0..6 {
            pad.update(0);
        }
        pad.update(CROSS);
        assert_eq!(pad.consume(&commands), 8);
        pad.time = -2;
        pad.update(UP);
        assert_eq!(pad.held, 0);
        pad.update(UP);
        assert_eq!(pad.trigger, UP);
        let input = pad.issue(&commands, 7);
        pad.update(input);
        assert_eq!(pad.consume(&commands), 7);
    }
}
