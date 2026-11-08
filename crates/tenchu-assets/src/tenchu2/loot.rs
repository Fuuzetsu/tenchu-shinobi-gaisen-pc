use super::{Edition, executable, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, executable::Executable, i16_le, slice};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loot {
    pub items: [usize; 6],
    pub actors: Vec<(i16, [u8; 6])>,
}

impl Loot {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, bytes) = executable(disc)?;
        if edition != Edition::Japan {
            return Err("Native Tenchu 2 inventories require the Japanese disc".into());
        }
        let executable = Executable::new(&bytes)?;
        let items = executable
            .bytes(0x8008e7e0, 6)?
            .iter()
            .map(|&item| usize::from(item))
            .collect::<Vec<_>>();
        if items.iter().any(|&item| item >= 20) {
            return Err("Invalid Tenchu 2 loot item".into());
        }
        let data = Volume::open(disc)?.read(r"o:\cdimage\game\module", "datamng.bin")?;
        let mut actors = Vec::new();
        for row in slice(&data, 0x7b1c, 0x180)?.chunks_exact(8) {
            let kind = i16_le(row, 0)?;
            if kind == -1 {
                return Ok(Self {
                    items: items.try_into().unwrap(),
                    actors,
                });
            }
            if !(0..256).contains(&kind) || actors.iter().any(|(id, _)| *id == kind) {
                return Err("Invalid Tenchu 2 loot actor".into());
            }
            actors.push((kind, row[2..].try_into().unwrap()));
        }
        Err("Unterminated Tenchu 2 loot table".into())
    }
}
