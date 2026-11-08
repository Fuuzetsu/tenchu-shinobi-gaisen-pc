use crate::{Result, i16_le, slice, u32_le};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedCommand {
    pub frame: i16,
    pub command: Command,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Effect {
        id: u8,
        position: [i32; 3],
        heading: i16,
        value: i32,
    },
    Fade {
        white: bool,
        outward: bool,
        duration: i16,
    },
    ClearFadeAndWarp,
    Tint {
        rgb: [u8; 3],
        subtract: bool,
    },
    ClearTint,
    Warp {
        variant: bool,
        phase: i32,
    },
    Portrait {
        index: usize,
        position: [i16; 2],
    },
    HidePortrait(usize),
    Picture {
        index: usize,
        duration: i16,
    },
    CameraRoll(i16),
    Weather(bool),
    Caption {
        text: String,
        duration: i16,
    },
    ClearCaption,
}

pub(super) fn parse(data: &[u8]) -> Result<Vec<TimedCommand>> {
    let mut at = u32_le(data, 80)? as usize;
    let mut caption = u32_le(data, 60)? as usize;
    let mut commands: Vec<TimedCommand> = Vec::new();
    let index = |value: i32, count: usize| {
        value
            .checked_sub(1)
            .and_then(|v| usize::try_from(v).ok())
            .filter(|&v| v < count)
            .ok_or_else(|| "Invalid Tenchu 2 story image reference".to_string())
    };
    loop {
        let frame = i16_le(data, at)?;
        if frame == 30000 {
            return Ok(commands);
        }
        if frame < 0 || commands.last().is_some_and(|c| c.frame > frame) {
            return Err("Invalid Tenchu 2 story command time".into());
        }
        let row = slice(data, at, 28)?;
        at += 28;
        let id = i16_le(row, 2)?;
        let value = u32_le(row, 16)? as i32;
        let heading = i16_le(row, 22)?;
        let duration = i16_le(row, 26)?;
        let command = match id {
            0 | 14 | 33 => continue,
            5..=8 | 22 => Command::Fade {
                white: id >= 7,
                outward: matches!(id, 6 | 8),
                duration: if id == 22 { 4 } else { 16 },
            },
            9 | 32 => Command::Warp {
                variant: id == 32,
                phase: value,
            },
            10 => Command::ClearFadeAndWarp,
            15 => Command::Portrait {
                index: index(value, 7)?,
                position: [i16_le(row, 4)?, i16_le(row, 8)?],
            },
            16 => Command::HidePortrait(index(value, 7)?),
            21 => Command::Picture {
                index: index(value as i16 as i32, 8)?,
                duration: duration.wrapping_mul(5),
            },
            23 => Command::CameraRoll(heading),
            27 => Command::Tint {
                rgb: [0x50, 0x40, 0x10],
                subtract: false,
            },
            28 => Command::ClearTint,
            30 | 31 => {
                let color = (i32::from(heading) << 16)
                    | (i32::from(i16_le(row, 20)?) << 8)
                    | i32::from(i16_le(row, 24)?);
                let [r, g, b, _] = color.to_le_bytes();
                Command::Tint {
                    rgb: [r, g, b],
                    subtract: id == 31,
                }
            }
            35 | 36 => Command::Weather(id == 36),
            37 => {
                let start = caption;
                while *data.get(caption).ok_or("Missing Tenchu 2 caption")? > 31 {
                    caption += 2;
                }
                let text = encoding_rs::SHIFT_JIS
                    .decode_without_bom_handling_and_without_replacement(slice(
                        data,
                        start,
                        caption - start,
                    )?)
                    .ok_or("Invalid Tenchu 2 caption encoding")?
                    .into_owned();
                slice(data, caption, 2)?;
                caption += 2;
                Command::Caption {
                    text,
                    duration: if duration == 1 { -1 } else { duration },
                }
            }
            38 => Command::ClearCaption,
            1..=4 | 11..=13 | 17..=20 | 24..=26 | 29 | 34 => Command::Effect {
                id: id as u8,
                position: [
                    u32_le(row, 4)? as i32,
                    u32_le(row, 8)? as i32,
                    u32_le(row, 12)? as i32,
                ],
                heading,
                value,
            },
            _ => return Err(format!("Unknown Tenchu 2 story command {id}")),
        };
        commands.push(TimedCommand { frame, command });
    }
}
