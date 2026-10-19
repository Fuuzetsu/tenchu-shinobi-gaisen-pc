use super::stream::Track;
use crate::{Result, executable::Executable, script::Language, u32_le};

type Voices = Vec<(u8, Track)>;

pub struct Catalog {
    music: Vec<Track>,
    ids: Vec<u8>,
    voices: Voices,
    intro: Voices,
    tora: Voices,
    common: Voices,
}

fn time(minute: u8, second: u8) -> Result<u32> {
    let bcd = |n: u8| -> Result<u32> {
        if n & 15 > 9 || n >> 4 > 9 {
            return Err("Invalid XA cue time".into());
        }
        Ok(u32::from(n >> 4) * 10 + u32::from(n & 15))
    };
    let second = bcd(second)?;
    if second >= 60 {
        return Err("XA cue seconds outside minute".into());
    }
    Ok((bcd(minute)? * 60 + second) * 150)
}

fn track(path: &str, row: &[u8], repeat: bool) -> Result<Track> {
    let track = Track {
        path: path
            .replace('\\', "/")
            .split(';')
            .next()
            .unwrap()
            .to_owned(),
        channel: row[0],
        start: time(row[1], row[2])?,
        end: time(row[3], row[4])?,
        repeat,
    };
    if track.start > track.end || (track.channel > 31 && track.start != track.end) {
        return Err("Invalid XA cue channel or range".into());
    }
    Ok(track)
}

fn voices(exe: &Executable<'_>, address: u32, path: &str) -> Result<Voices> {
    let mut voices = Vec::new();
    for i in 0..256 {
        let row = exe.bytes(address + i * 6, 6)?;
        if row[0] == 255 {
            return Ok(voices);
        }
        voices.push((row[0], track(path, &row[1..], false)?));
    }
    Err("Unterminated XA voice table".into())
}

impl Catalog {
    pub fn load(bytes: &[u8], language: Language) -> Result<Self> {
        let exe = Executable::new(bytes)?;
        let language = match language {
            Language::English => 0,
            Language::French => 1,
            Language::Italian => 2,
            Language::Japanese => 3,
        };
        let path = |i| exe.string(u32_le(exe.bytes(0x80097c98 + i * 4, 4)?, 0)?);
        let mut music = Vec::new();
        for i in 0..19 {
            let row = exe.bytes(0x80012980 + i * 12, 12)?;
            let file = exe.string(u32_le(row, 0)?)?;
            music.push(track(&format!("TENCHU/XA/{file}"), &row[4..9], true)?);
        }
        Ok(Self {
            music,
            ids: exe.bytes(0x8008ea2c, 19)?.to_vec(),
            voices: voices(
                &exe,
                u32_le(exe.bytes(0x800134e0 + language * 4, 4)?, 0)?,
                &path(2 + language)?,
            )?,
            intro: voices(&exe, 0x8008e82c, &path(0)?)?,
            tora: voices(&exe, 0x8008e930, &path(1)?)?,
            common: voices(&exe, 0x80012cbc, &path(2)?)?,
        })
    }

    pub fn event(&self, id: i16) -> Option<&Track> {
        if id < 100 {
            self.voice(i32::from(id))
        } else {
            let cue = i32::from(id) - 100;
            self.music(
                self.ids
                    .iter()
                    .position(|&id| i32::from(id) == cue)
                    .map_or(cue, |index| index as i32),
            )
        }
    }

    pub fn music(&self, cue: i32) -> Option<&Track> {
        match cue {
            0..=18 => self.music.get(cue as usize),
            19..=60 => self.voice(cue + 82),
            100.. => self.voice(cue + 100),
            _ => None,
        }
    }

    fn voice(&self, id: i32) -> Option<&Track> {
        let (table, id) = if id >= 200 {
            (&self.tora, id - 200)
        } else if id >= 100 {
            (&self.intro, id - 100)
        } else {
            (&self.voices, id)
        };
        table
            .iter()
            .chain(&self.common)
            .find(|(number, _)| i32::from(*number) == id)
            .map(|(_, track)| track)
    }

    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.music.iter().chain(
            self.voices
                .iter()
                .chain(&self.intro)
                .chain(&self.tora)
                .chain(&self.common)
                .map(|(_, track)| track),
        )
    }
}
