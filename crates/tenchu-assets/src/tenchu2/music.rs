use crate::{ReadAt, Result, audio::stream::Track, disc::Disc, executable::Executable, u16_le};

pub fn tracks(disc: &Disc<impl ReadAt>) -> Result<Vec<Track>> {
    let bytes = super::executable(disc)?;
    let exe = Executable::new(&bytes)?;
    let path = exe
        .string(0x8001323c)?
        .trim_start_matches('\\')
        .split(';')
        .next()
        .unwrap()
        .replace('\\', "/");
    let file = disc.find(&path)?;
    let mut tracks = Vec::new();
    for index in 0..64 {
        let row = exe.bytes(0x8007f354 + index * 8, 8)?;
        if u16_le(row, 6)? == 0 {
            return Ok(tracks);
        }
        if row[2] == 0 {
            continue;
        }
        let start = time(&row[4..6])?;
        let end = time(&row[6..8])?;
        if row[3] > 31 || start >= end || end > file.size.div_ceil(2048) {
            return Err("Invalid Tenchu 2 music cue".into());
        }
        tracks.push(Track {
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
