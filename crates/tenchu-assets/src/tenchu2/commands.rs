use crate::{ReadAt, Result, disc::Disc, executable::Executable, i16_le, u16_le};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Command {
    pub action: i16,
    pub inputs: Vec<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commands {
    pub standing: Vec<Command>,
    pub crouched: Vec<Command>,
    pub jumping: Vec<Command>,
}

impl Commands {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, data) = super::executable(disc)?;
        let executable = Executable::new(&data)?;
        let [standing, crouched, jumping] = edition.addresses().commands;
        Ok(Self {
            standing: table(&executable, standing)?,
            crouched: table(&executable, crouched)?,
            jumping: table(&executable, jumping)?,
        })
    }
}

fn table(executable: &Executable<'_>, address: u32) -> Result<Vec<Command>> {
    let mut out = Vec::new();
    for i in 0..256 {
        let row = executable.bytes(address + i * 16, 16)?;
        let action = i16_le(row, 0)?;
        if action == -1 {
            return Ok(out);
        }
        let mut inputs = Vec::new();
        for i in 1..8 {
            let input = u16_le(row, i * 2)?;
            if input == 0 {
                break;
            }
            inputs.push(input);
        }
        if action < 0 || inputs.is_empty() {
            return Err("Invalid Tenchu 2 command sequence".into());
        }
        out.push(Command { action, inputs });
    }
    Err("Unterminated Tenchu 2 command table".into())
}
