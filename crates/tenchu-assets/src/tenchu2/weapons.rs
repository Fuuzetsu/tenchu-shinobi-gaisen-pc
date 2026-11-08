use super::{Edition, executable, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, executable::Executable, i16_le, short3, u32_le};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attack {
    pub clip: i16,
    pub power: u8,
    pub steering: u8,
    pub combo: u8,
    pub start: u8,
    pub end: u8,
    pub trail_start: u8,
    pub trail_end: u8,
    pub bones: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Part {
    pub kind: i16,
    pub model: Option<String>,
    pub bone: usize,
    pub slot: usize,
    pub trail: [[i16; 3]; 2],
    pub center: [i16; 3],
    pub radius: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weapons {
    pub edition: Edition,
    pub parts: Vec<Part>,
    pub attacks: Vec<Attack>,
}

impl Weapons {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, data) = executable(disc)?;
        let executable = Executable::new(&data)?;
        let mut parts = Vec::new();
        for index in 0..512 {
            let row = executable.bytes(edition.addresses().weapons + index * 36, 36)?;
            let kind = i16_le(row, 0)?;
            if kind == -1 {
                let data = Volume::open(disc)?.read(r"o:\cdimage\game\weapon", "atktbl.dat")?;
                return Ok(Self {
                    edition,
                    parts,
                    attacks: attacks(&data)?,
                });
            }
            let bone = i16_le(row, 14)?;
            let slot = i16_le(row, 22)?;
            if !(0..64).contains(&bone) || !(0..5).contains(&slot) {
                return Err("Invalid Tenchu 2 weapon attachment".into());
            }
            let name = u32_le(row, 4)?;
            parts.push(Part {
                kind,
                model: if name == 0 {
                    None
                } else {
                    let name = executable.string(name)?;
                    if name.is_empty()
                        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    {
                        return Err("Invalid Tenchu 2 weapon model name".into());
                    }
                    Some(name)
                },
                bone: bone as usize,
                slot: slot as usize,
                trail: [short3(row, 8)?, short3(row, 16)?],
                center: short3(row, 24)?,
                radius: i16_le(row, 30)?,
            });
        }
        Err("Unterminated Tenchu 2 weapon registry".into())
    }

    pub fn parts(&self, kind: i16) -> impl Iterator<Item = &Part> {
        self.parts.iter().filter(move |part| part.kind == kind)
    }

    pub fn attack(&self, clip: i16) -> Option<&Attack> {
        self.attacks.iter().find(|attack| attack.clip == clip)
    }
}

fn attacks(data: &[u8]) -> Result<Vec<Attack>> {
    let mut out = Vec::new();
    for row in data.chunks_exact(16) {
        let clip = i16_le(row, 0)?;
        if clip == -1 {
            return Ok(out);
        }
        out.push(Attack {
            clip,
            power: row[2],
            steering: row[3],
            combo: row[4],
            start: row[5],
            end: row[6],
            trail_start: row[7],
            trail_end: row[8],
            bones: u32_le(row, 12)?,
        });
    }
    Err("Unterminated Tenchu 2 attack table".into())
}
