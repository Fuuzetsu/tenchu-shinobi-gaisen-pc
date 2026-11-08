use super::{Edition, cab_member, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, i16_le, short3, slice, u16_le, u32_le};

mod resources;
pub use resources::{Attachment, Model, Picture, Resources};
mod faces;
pub use faces::Patch;
mod commands;
pub use commands::{Command, TimedCommand};

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
    pub texture_model: i16,
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
    pub weapons: Vec<Weapon>,
    pub expressions: Vec<Expression>,
    pub commands: Vec<TimedCommand>,
    pub portraits: Vec<i16>,
    pub pictures: Vec<i16>,
    pub audio: [u8; 8],
}

#[derive(Clone, Copy)]
pub struct Expression {
    pub frame: i16,
    pub model: i16,
    pub channels: [u8; 2],
}

pub struct Weapon {
    pub actor: usize,
    pub part: usize,
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
        let mut portraits = Vec::new();
        let mut at = u32_le(data, 64)? as usize;
        loop {
            let model = i16_le(data, at)?;
            if model < 0 {
                break;
            }
            if portraits.len() == 7 || model >= 106 {
                return Err("Invalid Tenchu 2 story portrait".into());
            }
            portraits.push(model);
            at += 2;
        }
        let mut pictures = Vec::new();
        for i in 0..8 {
            let picture = i16_le(data, 48 + i * 2)?;
            if picture < 0 {
                break;
            }
            pictures.push(picture);
        }
        let commands = commands::parse(data)?;
        let mut weapons = Vec::new();
        let mut at = u32_le(data, 72)? as usize;
        loop {
            let actor = i16_le(data, at)?;
            let part = i16_le(data, at + 2)?;
            if actor < 0 || part < 0 {
                break;
            }
            if !(1..count).contains(&(actor as usize)) {
                return Err("Invalid Tenchu 2 story weapon owner".into());
            }
            weapons.push(Weapon {
                actor: actor as usize - 1,
                part: part as usize,
            });
            at += 4;
        }
        let mut expressions: Vec<Expression> = Vec::new();
        let mut at = u32_le(data, 76)? as usize;
        loop {
            let frame = i16_le(data, at)?;
            if frame == 30000 {
                break;
            }
            let channels: [u8; 2] = slice(data, at + 4, 2)?.try_into().unwrap();
            if frame < 0
                || expressions.last().is_some_and(|e| e.frame > frame)
                || channels.iter().any(|&v| v > 14 && v != 255)
            {
                return Err("Invalid Tenchu 2 facial animation".into());
            }
            expressions.push(Expression {
                frame,
                model: i16_le(data, at + 2)?,
                channels,
            });
            at += 8;
        }
        Ok(Self {
            duration,
            area,
            camera,
            actors,
            weapons,
            expressions,
            commands,
            portraits,
            pictures,
            audio: slice(data, 16, 8)?.try_into().unwrap(),
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
            texture_model: i16_le(data, 22)?,
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
