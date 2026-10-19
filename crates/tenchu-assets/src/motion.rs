use crate::{Result, i16_le, relative, short3, slice, u32_le};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub value: [i16; 3],
    pub time: i16,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub id: i16,
    pub duration: i16,
    pub sweep: i8,
    pub forward_speed: i8,
    pub side_speed: i8,
    pub root: Vec<Key>,
    pub bones: Vec<Vec<Key>>,
}

fn track(data: &[u8], at: usize, duration: i16) -> Result<Vec<Key>> {
    let mut keys = Vec::<Key>::new();
    for i in 0..32768 {
        let row = slice(data, at + i * 8, 8)?;
        let key = Key {
            value: short3(row, 0)?,
            time: i16_le(row, 6)?,
        };
        if keys.last().is_some_and(|p| p.time > key.time) || key.time < 0 {
            return Err("Unordered AMD key times".into());
        }
        if keys.is_empty() && key.time != 0 {
            return Err("AMD track does not begin at zero".into());
        }
        keys.push(key);
        if key.time >= duration {
            return Ok(keys);
        }
    }
    Err("Unterminated AMD track".into())
}

pub fn parse(data: &[u8]) -> Result<Vec<Clip>> {
    let count = u32_le(data, 0)? as usize;
    if count > 65536 {
        return Err("Excessive AMD clip count".into());
    }
    slice(data, 4, count * 4)?;
    let mut clips = Vec::new();
    for i in 0..count {
        let at = relative(data, 0, 4 + i * 4)?;
        let header = slice(data, at, 12)?;
        let bones = header[0] as usize;
        let duration = i16_le(header, 4)?;
        if duration < 0 {
            return Err("Negative AMD duration".into());
        }
        let root = if bones == 0 {
            Vec::new()
        } else {
            track(data, relative(data, at, at + 8)?, duration)?
        };
        let mut rotations = Vec::new();
        slice(data, at + 12, bones * 4)?;
        for b in 0..bones {
            rotations.push(track(data, relative(data, at, at + 12 + b * 4)?, duration)?);
        }
        clips.push(Clip {
            id: i16_le(header, 6)?,
            duration,
            sweep: header[1] as i8,
            forward_speed: header[2] as i8,
            side_speed: header[3] as i8,
            root,
            bones: rotations,
        });
    }
    Ok(clips)
}
