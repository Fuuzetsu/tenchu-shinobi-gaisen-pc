use super::{Edition, characters, motions, volume::Volume};
use crate::{
    ReadAt, Result, audio::stream::Track, disc::Disc, executable::Executable, frontend::Image,
    i16_le, model::arc_entry, u16_le, u32_le,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub struct Rules {
    pub names: [[Option<u8>; 11]; 3],
    pub descriptions: Vec<[String; 3]>,
    pub unlock_hero: [u16; 26],
    pub unlock_missions: [u16; 18],
}

pub struct Sprite {
    pub flags: u8,
    pub image: usize,
    pub position: [i16; 2],
    pub layer: i16,
    pub slices: usize,
    pub selected: usize,
}

pub struct Resources {
    pub rules: Rules,
    pub images: Vec<Image>,
    pub sprites: Vec<Sprite>,
    pub names: Image,
    pub buttons: Image,
    pub labels: Image,
    pub legend_y: i16,
    pub models: [characters::Rig; 3],
    pub motions: BTreeMap<i16, motions::Clip>,
    pub clips: [i16; 6],
    pub music: Track,
}

pub(super) fn executable(disc: &Disc<impl ReadAt>) -> Result<Vec<u8>> {
    if Edition::detect(disc)? != Edition::Japan {
        return Err("Native Tenchu 2 mission selection requires the Japanese disc".into());
    }
    let entry = disc.find("TENCHU2/MENU.EXE")?;
    if entry.size > 2 * 1024 * 1024 {
        return Err("Excessive Tenchu 2 menu executable size".into());
    }
    let bytes = disc.file(&entry).read(0, entry.size as usize)?;
    if format!("{:x}", Sha256::digest(&bytes))
        != "11ce0de09c116e1efc6b22bba13844953f45a65693dca09a11edba7989aa77a8"
    {
        return Err("Unsupported Japanese Tenchu 2 MENU.EXE revision".into());
    }
    Ok(bytes)
}

impl Rules {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        Self::parse(&Executable::new(&executable(disc)?)?)
    }

    fn parse(exe: &Executable<'_>) -> Result<Self> {
        let mut names = [[None; 11]; 3];
        for (hero, missions) in names.iter_mut().enumerate() {
            for (mission, name) in missions.iter_mut().enumerate() {
                let id = exe.bytes(0x80130800 + hero as u32 * 12 + mission as u32, 1)?[0];
                *name = match id {
                    0..40 => Some(id),
                    255 => None,
                    _ => return Err("Invalid Japanese mission name".into()),
                };
            }
        }
        let mut descriptions = Vec::with_capacity(33);
        for index in 0..33 {
            let mut lines = [String::new(), String::new(), String::new()];
            if names[index as usize / 11][index as usize % 11].is_none() {
                descriptions.push(lines);
                continue;
            }
            for (line, text) in lines.iter_mut().enumerate() {
                let pointer = u32_le(exe.bytes(0x80130cb0 + index * 12 + line as u32 * 4, 4)?, 0)?;
                if pointer == 0 {
                    continue;
                }
                let bytes = exe.bytes(pointer, 128)?;
                let end = bytes
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or("Unterminated Japanese mission text")?;
                let (decoded, _, malformed) = encoding_rs::SHIFT_JIS.decode(&bytes[..end]);
                if malformed {
                    return Err("Invalid Japanese mission text encoding".into());
                }
                *text = decoded.into_owned();
            }
            descriptions.push(lines);
        }
        let code = |address, length: usize| -> Result<Vec<u16>> {
            exe.bytes(address, length * 2)?
                .chunks_exact(2)
                .map(|row| u16_le(row, 0))
                .collect()
        };
        Ok(Self {
            names,
            descriptions,
            unlock_hero: code(0x80130f4c, 26)?.try_into().unwrap(),
            unlock_missions: code(0x80130f80, 18)?.try_into().unwrap(),
        })
    }
}

impl Resources {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let bytes = executable(disc)?;
        let exe = Executable::new(&bytes)?;
        let volume = Volume::open(disc)?;
        let read = |name| volume.read(r"o:\cdimage\menu\image", name);
        let data = read("stage.arc")?;
        let images = (0..7)
            .map(|i| super::texture::image(arc_entry(&data, i)?))
            .collect::<Result<Vec<_>>>()?;
        let mut sprites = Vec::new();
        for index in 0..64 {
            let row = exe.bytes(0x80130e54 + index * 10, 10)?;
            if row[0] == 255 {
                break;
            }
            if usize::from(row[1]) >= images.len() || row[0] & 0xc0 != 0 && row[8] == 0 {
                return Err("Invalid Japanese mission sprite".into());
            }
            sprites.push(Sprite {
                flags: row[0],
                image: usize::from(row[1]),
                position: [i16_le(row, 2)?, i16_le(row, 4)?],
                layer: i16_le(row, 6)?,
                slices: usize::from(row[8]),
                selected: usize::from(row[9]),
            });
        }
        if sprites.len() != 18 {
            return Err("Invalid Japanese mission layout".into());
        }
        let mut motions = motions::parse(&read("demo.mot")?)?;
        let mut clips = [0; 6];
        for (i, id) in clips.iter_mut().enumerate() {
            *id = i16_le(exe.bytes(0x80130f16 + i as u32 * 8, 2)?, 0)?;
            let clip = motions
                .get_mut(id)
                .ok_or("Missing Japanese selection motion")?;
            clip.blend = 5;
            for key in &mut clip.root {
                key.value[0] = 0;
                key.value[2] = -500;
            }
        }
        let names = super::texture::image(&read("dai.tim")?)?;
        let buttons = super::texture::image(&read("button.tim")?)?;
        let labels = super::texture::image(&read("select.tim")?)?;
        if names.size != [208, 240]
            || !buttons.size[1].is_multiple_of(5)
            || !labels.size[1].is_multiple_of(12)
        {
            return Err("Invalid Japanese mission image sheets".into());
        }
        let row = exe.bytes(0x8012f338, 8)?;
        let path = "TENCHU2/XA/MUSIC_J.XA";
        let start = super::music::time(&row[4..6])?;
        let end = super::music::time(&row[6..8])?;
        if u16_le(row, 0)? != 1
            || row[3] > 31
            || start >= end
            || end > disc.find(path)?.size.div_ceil(2048)
        {
            return Err("Invalid Japanese selection music".into());
        }
        Ok(Self {
            rules: Rules::parse(&exe)?,
            images,
            sprites,
            names,
            buttons,
            labels,
            legend_y: i16_le(exe.bytes(0x80139472, 2)?, 0)?,
            models: [
                characters::rig(disc, "rikimaru")?,
                characters::rig(disc, "ayame")?,
                characters::rig(disc, "tatumaru")?,
            ],
            motions,
            clips,
            music: Track {
                path: path.into(),
                channel: row[3],
                start,
                end,
                repeat: row[2] != 0,
            },
        })
    }
}
