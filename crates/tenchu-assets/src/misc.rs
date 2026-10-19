use crate::{
    ReadAt, Result, executable::Executable, game::GameAssets, i16_le, texture::Vram, u32_le,
};

#[derive(Clone, Copy, Debug)]
pub struct Hinged {
    pub models: [Option<usize>; 2],
    pub width: i16,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Fire,
    Puff([i32; 2]),
    Door {
        rotation: i16,
        kind: usize,
    },
    Pitfall {
        rotation: i16,
        kind: usize,
    },
    Snowfall,
    Sprite(usize),
    TextureScroll {
        rect: [u16; 4],
        source: [u16; 2],
        velocity: [i16; 2],
    },
    Bonfire(i32),
    Sound {
        index: u8,
        delay: [i16; 2],
    },
}

#[derive(Clone, Debug)]
pub struct Spawn {
    pub record: usize,
    pub position: [i32; 3],
    pub kind: Kind,
}

#[derive(Clone, Debug)]
pub struct Misc {
    pub spawns: Vec<Spawn>,
    pub doors: Vec<Hinged>,
    pub pitfalls: Vec<Hinged>,
    pub sprites: Vec<(u8, i32)>,
}

impl Misc {
    pub fn load<S: ReadAt>(
        game: &GameAssets<S>,
        construction: &[u8],
        vram: &mut Vram,
    ) -> Result<Self> {
        let exe = Executable::new(&game.executable)?;
        let hinged = |address: u32, count: usize| {
            exe.bytes(address, count * 12)?
                .chunks_exact(12)
                .map(|row| {
                    let model = |at| {
                        let index = u32_le(row, at)? as i32;
                        match index {
                            -1 => Ok(None),
                            0..=32 => Ok(Some(index as usize)),
                            _ => Err("Invalid environmental model index".to_owned()),
                        }
                    };
                    Ok(Hinged {
                        models: [model(0)?, model(4)?],
                        width: i16_le(row, 8)?,
                    })
                })
                .collect::<Result<Vec<_>>>()
        };
        let mut out = Self {
            spawns: Vec::new(),
            doors: hinged(0x8008e624, 11)?,
            pitfalls: hinged(0x8008e6a8, 3)?,
            sprites: exe
                .bytes(0x8008e6cc, 16)?
                .chunks_exact(8)
                .map(|row| {
                    let image = u32_le(row, 0)?;
                    if image >= 62 {
                        return Err("Invalid environmental sprite".into());
                    }
                    Ok((image as u8, u32_le(row, 4)? as i32))
                })
                .collect::<Result<_>>()?,
        };
        let mut scratch = [0x340u16, 0x100];
        let mut occupied = 0;
        for (record, row) in construction.chunks_exact(32).enumerate() {
            if i16_le(row, 0)? != 11 || occupied == 200 {
                continue;
            }
            let kind = u32_le(row, 4)? as i32;
            let p = [
                u32_le(row, 8)? as i32,
                u32_le(row, 12)? as i32,
                u32_le(row, 16)? as i32,
            ];
            let a = [
                u32_le(row, 20)? as i32,
                u32_le(row, 24)? as i32,
                u32_le(row, 28)? as i32,
            ];
            let index = |value: i32, limit: usize| -> Result<usize> {
                if value < 0 {
                    return Err("Negative environmental type".into());
                }
                Ok(if value as usize >= limit {
                    0
                } else {
                    value as usize
                })
            };
            let kind = match kind {
                0 if a[0] == 0 => Kind::Fire,
                0 => Kind::Puff([a[1], a[2]]),
                1 => Kind::Door {
                    rotation: a[0] as i16,
                    kind: index(a[1], out.doors.len())?,
                },
                2 => Kind::Pitfall {
                    rotation: a[0] as i16,
                    kind: index(a[1], out.pitfalls.len())?,
                },
                3 => Kind::Snowfall,
                4 => Kind::Sprite(index(a[0], out.sprites.len())?),
                5 => {
                    if !(0..7).contains(&p[0]) {
                        return Err("Invalid scrolling image index".into());
                    }
                    let name =
                        exe.string(u32_le(exe.bytes(0x80012788 + p[0] as u32 * 4, 4)?, 0)?)?;
                    let rect = vram
                        .upload_tim(&game.asset(&format!("IMAGE/{name}"))?)?
                        .word_rect();
                    if rect[2] == 0 || rect[3] == 0 {
                        return Err("Empty scrolling texture".into());
                    }
                    for y in 0..2 {
                        for x in 0..2 {
                            vram.copy_rect(
                                rect,
                                [scratch[0] + x * rect[2], scratch[1] + y * rect[3]],
                            )?;
                        }
                    }
                    let source = scratch;
                    scratch[1] += 64;
                    if scratch[1] > 512 {
                        scratch[1] = 256;
                        scratch[0] += 64;
                    }
                    Kind::TextureScroll {
                        rect,
                        source,
                        velocity: [p[1] as i16, p[2] as i16],
                    }
                }
                6 => Kind::Bonfire(a[0]),
                7 => Kind::Sound {
                    index: a[0] as u8,
                    delay: [a[1] as i16, a[2] as i16],
                },
                _ => return Err(format!("Unknown environmental kind {kind}")),
            };
            if !matches!(kind, Kind::TextureScroll { .. }) {
                occupied += 1;
            }
            out.spawns.push(Spawn {
                record,
                position: p,
                kind,
            });
        }
        Ok(out)
    }
}
