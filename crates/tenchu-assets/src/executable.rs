use crate::{Result, i16_le, short3, slice, text, u16_le, u32_le};

pub struct Executable<'a> {
    data: &'a [u8],
    base: u32,
}

#[derive(Clone, Debug)]
pub struct MotionRegistration {
    pub action: i16,
    pub clip: i16,
}

#[derive(Clone, Debug)]
pub struct Character {
    pub kind: i16,
    pub weapon: i16,
    pub turn: i16,
    pub health: i16,
    pub width: i16,
    pub height: i16,
    pub model: String,
    pub motions: Vec<MotionRegistration>,
}

pub struct MathTables {
    pub trig: Vec<[i16; 2]>,
    pub atan: Vec<i16>,
    pub sqrt: Vec<i16>,
    pub normal: Vec<i16>,
    pub hermite: [[i16; 4]; 33],
}

pub struct BodyTables {
    pub reflect_angle: [i16; 16],
    pub reflect_move: [[i16; 2]; 16],
}

#[derive(Clone, Debug)]
pub struct Command {
    pub id: u16,
    pub inputs: Vec<u16>,
}

#[derive(Clone, Debug)]
pub struct DeathEvent {
    pub frame: i16,
    pub action: i16,
    pub payload: [i16; 2],
}

impl<'a> Executable<'a> {
    pub fn appearance(&self, stage: usize) -> Result<Vec<i16>> {
        if stage >= 11 {
            return Err("Invalid sound stage".into());
        }
        let address = u32_le(self.bytes(0x8008f188 + (stage as u32 + 1) * 4, 4)?, 0)?;
        let mut kinds = Vec::new();
        for index in 0..128 {
            let kind = i16_le(self.bytes(address + index * 2, 2)?, 0)?;
            if kind == -1 {
                return Ok(kinds);
            }
            kinds.push(kind);
        }
        Err("Unterminated stage appearance table".into())
    }

    pub fn death_events(&self) -> Result<Vec<Vec<DeathEvent>>> {
        let mut tables = Vec::new();
        for i in 0..6 {
            let address = u32_le(self.bytes(0x80086b0c + i * 4, 4)?, 0)?;
            let mut events = Vec::new();
            for j in 0..256 {
                let row = self.bytes(address + j * 8, 8)?;
                let event = DeathEvent {
                    frame: i16_le(row, 0)?,
                    action: i16_le(row, 2)?,
                    payload: [i16_le(row, 4)?, i16_le(row, 6)?],
                };
                let end = event.action == 4;
                events.push(event);
                if end {
                    break;
                }
                if j == 255 {
                    return Err("Unterminated stealth death events".into());
                }
            }
            tables.push(events);
        }
        Ok(tables)
    }

    pub fn new(data: &'a [u8]) -> Result<Self> {
        if slice(data, 0, 8)? != b"PS-X EXE" {
            return Err("Invalid PS-X EXE header".into());
        }
        Ok(Self {
            data,
            base: u32_le(data, 0x18)?,
        })
    }

    pub fn bytes(&self, address: u32, count: usize) -> Result<&'a [u8]> {
        let at = address
            .checked_sub(self.base)
            .ok_or("EXE address before base")?;
        slice(
            self.data,
            (at as usize)
                .checked_add(0x800)
                .ok_or("EXE offset overflow")?,
            count,
        )
    }

    pub fn string(&self, address: u32) -> Result<String> {
        let at = address
            .checked_sub(self.base)
            .ok_or("EXE string before base")? as usize;
        let at = at.checked_add(0x800).ok_or("EXE string offset overflow")?;
        text(self.data.get(at..).ok_or("EXE string outside file")?)
    }

    pub fn math_tables(&self) -> Result<MathTables> {
        let shorts = |address, count: usize| -> Result<Vec<i16>> {
            Ok(self
                .bytes(address, count * 2)?
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect())
        };
        let trig = self
            .bytes(0x80090ec8, 4096 * 4)?
            .chunks_exact(4)
            .map(|b| Ok([i16_le(b, 0)?, i16_le(b, 2)?]))
            .collect::<Result<_>>()?;
        // Signed-overflow inputs to SquareRoot0 address the preceding table.
        let mut sqrt = shorts(0x80090618, 192)?;
        sqrt.extend([0; 128]);
        sqrt.extend(shorts(0x80090898, 192)?);
        let mut hermite = [[0; 4]; 33];
        for (out, bytes) in hermite
            .iter_mut()
            .zip(self.bytes(0x800868a4, 33 * 8)?.chunks_exact(8))
        {
            for (i, v) in out.iter_mut().enumerate() {
                *v = i16_le(bytes, i * 2)?;
            }
        }
        Ok(MathTables {
            trig,
            atan: shorts(0x80094ec8, 1025)?,
            sqrt,
            // VectorNormalSS can index before its table for zero or overflowing square sums.
            normal: shorts(0x800907ac, 512)?,
            hermite,
        })
    }

    pub fn body_tables(&self) -> Result<BodyTables> {
        let mut reflect_angle = [0; 16];
        let mut reflect_move = [[0; 2]; 16];
        let angles = self.bytes(0x80086774, 32)?;
        let movement = self.bytes(0x80086794, 64)?;
        for i in 0..16 {
            reflect_angle[i] = i16_le(angles, i * 2)?;
            reflect_move[i] = [i16_le(movement, i * 4)?, i16_le(movement, i * 4 + 2)?];
        }
        Ok(BodyTables {
            reflect_angle,
            reflect_move,
        })
    }

    pub fn motion_registry(&self, address: u32) -> Result<Vec<MotionRegistration>> {
        let mut out = Vec::new();
        for i in 0..2048 {
            let row = self.bytes(address + i * 8, 8)?;
            let action = i16_le(row, 0)?;
            if action == -1 {
                return Ok(out);
            }
            out.push(MotionRegistration {
                action,
                clip: i16_le(row, 2)?,
            });
        }
        Err("Unterminated motion registry".into())
    }

    pub fn common_motions(&self) -> Result<Vec<MotionRegistration>> {
        self.motion_registry(0x80086b94)
    }

    pub fn characters(&self) -> Result<Vec<Character>> {
        self.characters_at(0x80088a8c)
    }

    pub(crate) fn characters_at(&self, address: u32) -> Result<Vec<Character>> {
        let mut out = Vec::new();
        for i in 0..256 {
            let row = self.bytes(address + i * 24, 24)?;
            let kind = i16_le(row, 0)?;
            if kind == -1 {
                return Ok(out);
            }
            out.push(Character {
                kind,
                weapon: i16_le(row, 2)?,
                turn: i16_le(row, 4)?,
                health: i16_le(row, 6)?,
                width: i16_le(row, 8)?,
                height: i16_le(row, 10)?,
                motions: self.motion_registry(u32_le(row, 12)?)?,
                model: self.string(u32_le(row, 16)?)?,
            });
        }
        Err("Unterminated character table".into())
    }

    pub fn commands(&self) -> Result<Vec<Command>> {
        let mut out = Vec::new();
        for i in 0..32 {
            let ptr = u32_le(self.bytes(0x8008686c + i * 4, 4)?, 0)?;
            if ptr == 0 {
                return Ok(out);
            }
            let id = u16_le(self.bytes(ptr, 2)?, 0)?;
            let mut inputs = Vec::new();
            for j in 1..=5 {
                let input = u16_le(self.bytes(ptr + j * 2, 2)?, 0)?;
                if input == 0xffff {
                    break;
                }
                inputs.push(input);
            }
            if !(1..=4).contains(&inputs.len()) {
                return Err("Invalid controller command pattern".into());
            }
            out.push(Command { id, inputs });
        }
        Err("Unterminated controller command table".into())
    }

    pub fn story_actors(&self, stage: usize) -> Result<Vec<StoryActor>> {
        let mut out = Vec::new();
        for i in 0..256 {
            let row = self.bytes(0x8008e6dc + i * 14, 14)?;
            let id = i16_le(row, 0)?;
            if id == -1 {
                return Ok(out);
            }
            if id as usize == stage + 1 {
                out.push(StoryActor {
                    kind: i16_le(row, 2)?,
                    position: short3(row, 4)?.map(|v| i32::from(v) * 1000),
                    facing: i16_le(row, 10)?,
                    think: i16_le(row, 12)?,
                });
            }
        }
        Err("Unterminated story actor table".into())
    }
}

#[derive(Clone, Debug)]
pub struct StoryActor {
    pub kind: i16,
    pub position: [i32; 3],
    pub facing: i16,
    pub think: i16,
}
