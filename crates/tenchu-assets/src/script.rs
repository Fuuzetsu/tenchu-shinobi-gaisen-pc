mod command;
pub use command::{ActorCommand, CameraCommand, Command, EffectCommand, Pan, Placement};

use crate::{ReadAt, Result, game::GameAssets, i16_le, short3, slice, u32_le};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug)]
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

pub struct Scripts {
    pub language: Language,
    pub events: Vec<Event>,
    pub cutscenes: Cutscenes,
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
        Ok(Self {
            language,
            events: events(&game.asset(&format!("ANIM/STAGE{number}.ESD"))?)?,
            cutscenes: Cutscenes::parse(game.asset(&format!(
                "{}/STAGE{number}{suffix}.CAD",
                language.directory()
            ))?)?,
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
