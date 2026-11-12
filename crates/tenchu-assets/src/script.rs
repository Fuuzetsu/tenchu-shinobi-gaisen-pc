mod command;
pub use command::{ActorCommand, CameraCommand, Command, EffectCommand, Pan, Placement};

use crate::{ReadAt, Result, game::GameAssets, i16_le, short3, slice, u32_le};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[default]
    English,
    French,
    Italian,
    Japanese,
}

impl Language {
    pub fn directory(self) -> &'static str {
        match self {
            Self::English => "ANIM/ENGLISH",
            Self::French => "ANIM/FRENCH",
            Self::Italian => "ANIM/ITALIAN",
            Self::Japanese => "ANIM",
        }
    }

    pub fn decode(self, bytes: &[u8]) -> String {
        let encoding = if self == Self::Japanese {
            encoding_rs::SHIFT_JIS
        } else {
            encoding_rs::WINDOWS_1252
        };
        encoding.decode(bytes).0.into_owned()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: u8,
    pub sequence: u8,
    pub next: [u8; 2],
    pub target: u8,
    pub mode: u8,
    pub value: i16,
    pub bounds: [[i16; 2]; 3],
}

pub fn events(data: &[u8]) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    let mut at = 0;
    loop {
        if u32_le(data, at)? == u32::MAX {
            return Ok(events);
        }
        let row = slice(data, at, 20)?;
        if row[5] > 8 {
            return Err(format!("Unknown stage event mode {}", row[5]));
        }
        events.push(Event {
            id: row[0],
            sequence: row[1],
            next: [row[2], row[3]],
            target: row[4],
            mode: row[5],
            value: i16_le(row, 6)?,
            bounds: [
                [i16_le(row, 8)?, i16_le(row, 10)?],
                [i16_le(row, 12)?, i16_le(row, 14)?],
                [i16_le(row, 16)?, i16_le(row, 18)?],
            ],
        });
        at += 20;
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Cutscenes {
    pub commands: Vec<Command>,
    data: Vec<u8>,
}

impl Cutscenes {
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        let mut commands = Vec::new();
        let mut at = 0;
        while i16_le(&data, at)? != -1 {
            let row = slice(&data, at, 12)?;
            commands.push(Command::decode(
                i16_le(row, 0)?,
                i16_le(row, 2)?,
                short3(row, 4)?,
                i16_le(row, 10)?,
            )?);
            at += 12;
        }
        let result = Self { commands, data };
        for command in &result.commands {
            if let Command::Caption {
                offset: Some(offset),
            } = *command
            {
                result.text(offset)?;
            }
        }
        Ok(result)
    }

    pub fn sequence(&self, id: i16) -> Option<(usize, i16)> {
        self.commands
            .iter()
            .enumerate()
            .find_map(|(index, command)| match *command {
                Command::Sequence { id: found, music } if found == id => Some((index, music)),
                _ => None,
            })
    }

    pub fn text(&self, offset: usize) -> Result<&[u8]> {
        let bytes = self
            .data
            .get(offset..)
            .ok_or("CVA text offset outside asset")?;
        let end = bytes
            .iter()
            .position(|b| *b == 0)
            .ok_or("Unterminated CVA text")?;
        Ok(&bytes[..end])
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Scripts {
    pub language: Language,
    pub events: Vec<Event>,
    pub cutscenes: Cutscenes,
    pub subtitles: [Subtitles; 4],
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Subtitles(std::collections::BTreeMap<i16, Vec<Subtitle>>);

#[derive(serde::Serialize, serde::Deserialize)]
struct Subtitle {
    frame: i16,
    text: String,
}

impl Subtitles {
    fn from_script(script: &Cutscenes, language: Language, poem: bool) -> Result<Self> {
        let mut tracks = Self::default();
        for (start, command) in script.commands.iter().enumerate() {
            let Command::Sequence { id, .. } = *command else {
                continue;
            };
            if tracks.0.contains_key(&id) {
                continue;
            }
            let mut track = Vec::new();
            let mut frame: i16 = 0;
            for command in &script.commands[start + 1..] {
                match *command {
                    Command::Wait { until: 0 } => break,
                    // Each wait yields a tick, including consecutive waits at the same time.
                    Command::Wait { until } => frame = until.max(frame.saturating_add(1)),
                    Command::Caption { offset } => {
                        let bytes = offset
                            .map(|offset| script.text(offset))
                            .transpose()?
                            .unwrap_or_default();
                        let bytes = if language == Language::Japanese {
                            &bytes[..bytes.len().min(32)]
                        } else {
                            bytes
                        };
                        let text = if poem && bytes.first().is_some_and(u8::is_ascii_digit) {
                            String::new()
                        } else {
                            language.decode(bytes)
                        };
                        track.push(Subtitle { frame, text });
                    }
                    _ => {}
                }
            }
            tracks.0.insert(id, track);
        }
        Ok(tracks)
    }

    pub fn text(&self, sequence: i16, frame: i16) -> &str {
        self.0
            .get(&sequence)
            .and_then(|track| track.iter().rev().find(|cue| cue.frame <= frame))
            .map_or("", |cue| cue.text.as_str())
    }
}

impl Scripts {
    pub fn load<S: ReadAt>(
        game: &GameAssets<S>,
        stage: usize,
        character: i16,
        language: Language,
    ) -> Result<Self> {
        let suffix = match character {
            0 => 'R',
            1 => 'A',
            _ => return Err("Invalid script character".into()),
        };
        let number = stage + 1;
        let load = |language: Language| {
            Cutscenes::parse(game.asset(&format!(
                "{}/STAGE{number}{suffix}.CAD",
                language.directory()
            ))?)
        };
        let cutscenes = load(language)?;
        let mut subtitles = std::array::from_fn(|_| Subtitles::default());
        for alternate in [
            Language::English,
            Language::French,
            Language::Italian,
            Language::Japanese,
        ] {
            subtitles[alternate as usize] = if alternate == language {
                Subtitles::from_script(&cutscenes, alternate, stage == 10 && character == 0)?
            } else {
                Subtitles::from_script(&load(alternate)?, alternate, stage == 10 && character == 0)?
            };
        }
        Ok(Self {
            language,
            events: events(&game.asset(&format!("ANIM/STAGE{number}.ESD"))?)?,
            cutscenes,
            subtitles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutscene_terminator_and_text_offsets_are_byte_boundaries() {
        let mut bytes = Vec::new();
        for row in [
            [0i16, 42, 0, 0, 0, 17],
            [8, 38, 0, 0, 0, 0],
            [1, 0, 0, 0, 0, 0],
        ] {
            bytes.extend(row.into_iter().flat_map(i16::to_le_bytes));
        }
        bytes.extend((-1i16).to_le_bytes());
        bytes.extend(b"hello\0");
        let parsed = Cutscenes::parse(bytes.clone()).unwrap();
        assert_eq!(parsed.sequence(42), Some((0, 17)));
        assert_eq!(parsed.commands.len(), 3);
        assert_eq!(parsed.text(38).unwrap(), b"hello");
        assert!(Cutscenes::parse(bytes[..43].to_vec()).is_err());
        bytes[14..16].copy_from_slice(&(-2i16).to_le_bytes());
        assert!(Cutscenes::parse(bytes).is_err());
        assert!(Cutscenes::parse(vec![0; 11]).is_err());
    }

    #[test]
    fn event_table_keeps_signed_bounds_and_requires_its_sentinel() {
        let mut bytes = vec![42, 12, 7, 255, 255, 1];
        bytes.extend(
            [-1i16, -12, -4, -7, 0, -8, 31]
                .into_iter()
                .flat_map(i16::to_le_bytes),
        );
        assert!(events(&bytes).is_err());
        bytes.extend(u32::MAX.to_le_bytes());
        let parsed = events(&bytes).unwrap();
        assert_eq!(parsed[0].bounds, [[-12, -4], [-7, 0], [-8, 31]]);
        assert_eq!(parsed[0].value, -1);
        assert_eq!(parsed[0].next, [7, 255]);
    }
}
