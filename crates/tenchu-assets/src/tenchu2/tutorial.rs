use super::{Edition, music, volume::Volume};
use crate::{
    ReadAt, Result, audio::stream::Track, disc::Disc, executable::Executable, i16_le, u16_le,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lesson {
    pub position: [i16; 3],
    pub radius: i16,
    pub cue: u16,
    pub repetitions: u8,
    pub command: Command,
    pub action: i16,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Command {
    Finished,
    Reach,
    ReachNext,
    Action,
    ActionRange,
    DefeatPracticeGuards,
    ThrowShuriken,
    CameraDirections,
    LeaveWall,
    SelectShuriken,
    Look,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tutorial {
    pub lessons: Vec<Lesson>,
    pub starts: [usize; 4],
    pub bounds: [[i16; 4]; 4],
    pub tracks: std::collections::BTreeMap<u16, Track>,
}

impl Tutorial {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, executable) = super::executable(disc)?;
        if edition != Edition::Japan {
            return Err("Training narration requires Japanese Tenchu 2".into());
        }
        let module = Volume::open(disc)?.game_module()?;
        // JP DATAMNG.BIN: lesson rows, course starts and course bounds used by 800d202c.
        let base = 0x800d0610;
        let mut lessons = Vec::new();
        for i in 0..27 {
            let at = 0x800d8d68 - base + i * 16;
            let command = match i16_le(&module, at + 12)? {
                -1 => Command::Finished,
                1 => Command::Reach,
                2 => Command::ReachNext,
                3 => Command::Action,
                4 => Command::ActionRange,
                5 => Command::DefeatPracticeGuards,
                6 => Command::ThrowShuriken,
                7 => Command::CameraDirections,
                8 => Command::LeaveWall,
                9 => Command::SelectShuriken,
                10 => Command::Look,
                _ => return Err("Unknown Tenchu 2 training command".into()),
            };
            lessons.push(Lesson {
                position: crate::short3(&module, at)?,
                radius: i16_le(&module, at + 6)?,
                cue: u16::from(module[at + 10]) + 7,
                repetitions: module[at + 11],
                command,
                action: i16_le(&module, at + 14)?,
            });
        }
        let mut starts = [0; 4];
        let mut bounds = [[0; 4]; 4];
        for i in 0..4 {
            starts[i] = usize::from(u16_le(&module, 0x800d8f18 - base + i * 2)?);
            if starts[i] >= lessons.len() {
                return Err("Invalid training course".into());
            }
            for (j, bound) in bounds[i].iter_mut().enumerate() {
                *bound = i16_le(&module, 0x800d8f24 - base + (i * 4 + j) * 2)?;
            }
        }
        let exe = Executable::new(&executable)?;
        let path = "TENCHU2/XA/NARAT_J.XA";
        let file = disc.find(path)?;
        let mut tracks = std::collections::BTreeMap::new();
        for i in 0..64 {
            let row = exe.bytes(0x800812e4 + i * 8, 8)?;
            if u16_le(row, 6)? == 0 {
                break;
            }
            let track = Track {
                path: path.into(),
                channel: row[3],
                start: music::time(&row[4..6])?,
                end: music::time(&row[6..8])?,
                repeat: false,
            };
            if track.channel > 31
                || track.start >= track.end
                || track.end > file.size.div_ceil(2048)
            {
                return Err("Invalid training narration cue".into());
            }
            tracks.insert(u16_le(row, 0)?, track);
        }
        if lessons.iter().any(|l| !tracks.contains_key(&l.cue)) {
            return Err("Missing training narration".into());
        }
        Ok(Self {
            lessons,
            starts,
            bounds,
            tracks,
        })
    }
}
