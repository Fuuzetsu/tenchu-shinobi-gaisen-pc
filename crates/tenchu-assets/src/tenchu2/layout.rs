use super::{executable, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, executable::Executable, i16_le, slice, u16_le, u32_le};

#[derive(Clone)]
pub struct Layout {
    pub sequence: usize,
    pub map: usize,
    pub start: [i32; 3],
    pub facing: i16,
    pub actors: Vec<Actor>,
}

#[derive(Clone)]
pub struct Actor {
    pub kind: i16,
    pub position: [i32; 3],
    pub facing: i16,
    pub route: Vec<Waypoint>,
}

#[derive(Clone)]
pub struct Waypoint {
    pub x: i32,
    pub z: i32,
    pub action: i32,
    pub wait: i32,
}

impl Layout {
    pub fn load(disc: &Disc<impl ReadAt>, sequence: usize) -> Result<Self> {
        if sequence >= 300 {
            return Err("Tenchu 2 sequence must be 0–299".into());
        }
        let bytes = Volume::open(disc)?
            .read(r"o:\cdimage\game\layout", &format!("seq{sequence:03}.lay"))?;
        let (edition, executable) = executable(disc)?;
        Self::parse(&bytes, sequence, &executable, edition.addresses().sequences)
    }

    pub fn available(disc: &Disc<impl ReadAt>) -> Result<Vec<Self>> {
        let volume = Volume::open(disc)?;
        let (edition, executable) = executable(disc)?;
        let mut out = Vec::new();
        for sequence in 0..300 {
            if let Some(bytes) =
                volume.optional(r"o:\cdimage\game\layout", &format!("seq{sequence:03}.lay"))?
            {
                out.push(Self::parse(
                    &bytes,
                    sequence,
                    &executable,
                    edition.addresses().sequences,
                )?);
            }
        }
        Ok(out)
    }

    fn parse(data: &[u8], sequence: usize, executable: &[u8], sequences: u32) -> Result<Self> {
        let executable = Executable::new(executable)?;
        let descriptor = executable.bytes(sequences + sequence as u32 * 16, 16)?;
        let map = usize::from(u16_le(descriptor, 2)?);
        if u16_le(descriptor, 0)? & 0xf0 != 0x30 || map >= 24 {
            return Err("Unexpected Tenchu 2 gameplay sequence".into());
        }
        slice(data, 0, 3616)?;
        let position = |at| -> Result<[i32; 3]> {
            Ok([
                u32_le(data, at)? as i32,
                u32_le(data, at + 4)? as i32,
                u32_le(data, at + 8)? as i32,
            ])
        };
        let mut actors = Vec::new();
        for row in slice(data, 16, 40 * 24)?.chunks_exact(24) {
            let kind = i16_le(row, 0)?;
            if kind == -1 {
                continue;
            }
            let path = i16_le(row, 2)?;
            if !(0..20).contains(&path) {
                return Err("Invalid Tenchu 2 patrol index".into());
            }
            let mut route = Vec::new();
            if path != 0 {
                let path_at = 976 + path as usize * 132;
                let count = i16_le(data, path_at + 2)?;
                if !(0..=8).contains(&count) {
                    return Err("Invalid Tenchu 2 patrol length".into());
                }
                for step in slice(data, path_at + 4, count as usize * 16)?.chunks_exact(16) {
                    route.push(Waypoint {
                        x: u32_le(step, 0)? as i32,
                        action: u32_le(step, 4)? as i32,
                        z: u32_le(step, 8)? as i32,
                        wait: u32_le(step, 12)? as i32,
                    });
                }
            }
            actors.push(Actor {
                kind,
                position: [
                    u32_le(row, 8)? as i32,
                    u32_le(row, 12)? as i32,
                    u32_le(row, 16)? as i32,
                ],
                facing: i16_le(row, 6)?,
                route,
            });
        }
        Ok(Self {
            sequence,
            map,
            start: position(0)?,
            facing: u32_le(data, 12)? as i16,
            actors,
        })
    }
}
