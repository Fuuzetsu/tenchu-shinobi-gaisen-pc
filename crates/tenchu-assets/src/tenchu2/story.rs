use super::{Edition, cab_member, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, i16_le, short3, slice, u16_le, u32_le};

#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub value: [i16; 3],
    pub flags: u16,
}

impl Key {
    pub fn time(self) -> i16 {
        (self.flags & 0x1fff) as i16
    }

    pub fn linear(self) -> bool {
        self.flags & 0x2000 != 0
    }

    pub fn shift(self) -> u32 {
        u32::from(self.flags >> 14)
    }
}

pub struct Animation {
    pub model: i16,
    pub origin: [i32; 3],
    pub duration: i16,
    pub root: Vec<Key>,
    pub joints: Vec<Vec<Key>>,
}

pub struct Script {
    pub duration: i32,
    pub area: Option<usize>,
    pub camera: Animation,
    pub actors: Vec<Animation>,
}

impl Script {
    pub fn load(disc: &Disc<impl ReadAt>, event: u16) -> Result<Self> {
        let volume = Volume::open(disc)?;
        if volume.edition != Edition::Japan {
            return Err("Native Tenchu 2 story requires the Japanese disc".into());
        }
        let cab = volume.read(r"o:\cdimage\game\cab\event", &format!("E{event}.CAB"))?;
        Self::parse(cab_member(
            &cab,
            r"o:\cdimage\game\event",
            &format!("E{event}.EVT"),
        )?)
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let count = u32_le(data, 0)? as usize;
        let duration = u32_le(data, 8)? as i32;
        if !(1..=33).contains(&count) || !(1..=8191).contains(&duration) {
            return Err("Invalid Tenchu 2 story duration or actor count".into());
        }
        let area = match u32_le(data, 12)? as i32 {
            -1 => None,
            area @ 0..24 => Some(area as usize),
            _ => return Err("Invalid Tenchu 2 story area".into()),
        };
        let offset = |slot| -> Result<usize> { Ok(u32_le(data, 84 + slot * 4)? as usize) };
        let camera = Animation::parse(data, offset(0)?)?;
        if camera.joints.len() != 1 {
            return Err("Invalid Tenchu 2 story camera".into());
        }
        let actors = (1..count)
            .map(|slot| Animation::parse(data, offset(slot)?))
            .collect::<Result<_>>()?;
        Ok(Self {
            duration,
            area,
            camera,
            actors,
        })
    }
}

impl Animation {
    fn parse(data: &[u8], at: usize) -> Result<Self> {
        let data = slice(data, at, u32_le(data, at)? as usize)?;
        let duration = (u16_le(data, 18)? & 0x1fff) as i16;
        let count = u16_le(data, 20)? as usize;
        if !(1..=64).contains(&count) {
            return Err("Invalid Tenchu 2 story joint count".into());
        }
        let track = |slot| track(data, u32_le(data, 28 + slot * 4)? as usize, duration);
        Ok(Self {
            model: i16_le(data, 16)?,
            origin: [
                u32_le(data, 4)? as i32,
                u32_le(data, 8)? as i32,
                u32_le(data, 12)? as i32,
            ],
            duration,
            root: track(0)?,
            joints: (1..=count).map(track).collect::<Result<_>>()?,
        })
    }
}

fn track(data: &[u8], mut at: usize, duration: i16) -> Result<Vec<Key>> {
    let mut keys: Vec<Key> = Vec::new();
    loop {
        let key = Key {
            value: short3(data, at)?,
            flags: u16_le(data, at + 6)?,
        };
        if keys
            .last()
            .map_or(key.time() != 0, |previous| previous.time() > key.time())
        {
            return Err("Invalid Tenchu 2 story key times".into());
        }
        keys.push(key);
        if keys.len() >= 2 && key.time() >= duration {
            return Ok(keys);
        }
        at += 8;
    }
}
