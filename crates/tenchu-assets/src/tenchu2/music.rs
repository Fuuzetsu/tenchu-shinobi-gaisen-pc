use super::{Edition, volume::Volume};
use crate::{
    ReadAt, Result,
    audio::{bank::Bank, stream::Track},
    disc::Disc,
    executable::Executable,
    i16_le, u16_le,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct Ambience {
    pub bank: Arc<Bank>,
    pub interval: i16,
    pub ticks_per_second: u32,
    pub ship_variants: bool,
}

pub fn ambience(disc: &Disc<impl ReadAt>, map: usize) -> Result<Ambience> {
    if map >= 24 {
        return Err("Invalid Tenchu 2 ambience map".into());
    }
    let (edition, bytes) = super::executable(disc)?;
    let exe = Executable::new(&bytes)?;
    let mut interval = i16_le(
        exe.bytes(edition.addresses().ambience_intervals + map as u32 * 2, 2)?,
        0,
    )?;
    let ticks_per_second = match edition {
        Edition::Europe => {
            // GAME.EXE converts the whole table, including negative entries.
            interval = interval * 5 / 6;
            25
        }
        Edition::Japan => 30,
    };
    let volume = Volume::open(disc)?;
    // The environment tones are identical in all three heroes' banks.
    let source =
        Bank::parse(&volume.read("o:\\cdimage\\game\\sound", &format!("R_E_{map:02}.vab"))?)?;
    let mut bank = Bank {
        volume: source.volume,
        pan: source.pan,
        programs: Vec::new(),
        samples: Vec::new(),
    };
    bank.import_program(&source, 2)?;
    Ok(Ambience {
        bank: Arc::new(bank),
        interval,
        ticks_per_second,
        ship_variants: edition == Edition::Japan && matches!(map, 21 | 23),
    })
}

pub const THEMES: [(u16, &str); 8] = [
    (1, "Character selection"),
    (2, "Options"),
    (3, "Editor menu"),
    (9, "Boss battle"),
    (10, "Final battle"),
    (11, "Editor — landscape"),
    (12, "Editor — settlement"),
    (13, "Editor — cave / ship"),
];

pub fn tracks(disc: &Disc<impl ReadAt>) -> Result<Vec<Track>> {
    let (edition, bytes) = super::executable(disc)?;
    let addresses = edition.addresses();
    let exe = Executable::new(&bytes)?;
    let path = exe
        .string(addresses.music_path)?
        .trim_start_matches('\\')
        .split(';')
        .next()
        .unwrap()
        .replace('\\', "/");
    let file = disc.find(&path)?;
    let mut tracks = vec![None; THEMES.len()];
    for index in 0..64 {
        let row = exe.bytes(addresses.music_cues + index * 8, 8)?;
        if u16_le(row, 6)? == 0 {
            return tracks
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| "Missing Tenchu 2 music cue".into());
        }
        let id = u16_le(row, 0)?;
        // Retail restarts some songs outside the XA callback, despite repeat = 0.
        let Some(selected) = THEMES.iter().position(|&(cue, _)| cue == id) else {
            continue;
        };
        let start = time(&row[4..6])?;
        let end = time(&row[6..8])?;
        if row[3] > 31 || start >= end || end > file.size.div_ceil(2048) {
            return Err("Invalid Tenchu 2 music cue".into());
        }
        tracks[selected] = Some(Track {
            path: path.clone(),
            channel: row[3],
            start,
            end,
            repeat: true,
        });
    }
    Err("Unterminated Tenchu 2 music table".into())
}

fn time(bytes: &[u8]) -> Result<u32> {
    let bcd = |n: u8| -> Result<u32> {
        if n & 15 > 9 || n >> 4 > 9 {
            return Err("Invalid Tenchu 2 music time".into());
        }
        Ok(u32::from(n >> 4) * 10 + u32::from(n & 15))
    };
    let seconds = bcd(bytes[0])?;
    if seconds >= 60 {
        return Err("Tenchu 2 music seconds outside minute".into());
    }
    Ok((bcd(bytes[1])? * 60 + seconds) * 150)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_times_use_double_speed_sectors_and_reject_invalid_bcd() {
        assert_eq!(time(&[0x34, 0x01]).unwrap(), 94 * 150);
        for value in [[0x60, 0], [0x1a, 0], [0, 0xfa]] {
            assert!(time(&value).is_err());
        }
    }
}
