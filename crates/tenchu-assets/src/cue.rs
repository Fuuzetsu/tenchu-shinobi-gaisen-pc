use crate::{Result, disc::SectorFormat};

#[derive(Debug, PartialEq, Eq)]
pub struct DataTrack {
    pub file: String,
    pub format: SectorFormat,
    pub start_sector: u64,
}

pub fn data_track(cue: &str) -> Result<DataTrack> {
    let mut file = None;
    let mut format = None;
    for line in cue.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("FILE ") {
            let (name, kind) = if let Some(quoted) = value.strip_prefix('"') {
                quoted.split_once('"').ok_or("Unclosed CUE filename")?
            } else {
                value.rsplit_once(' ').ok_or("Missing CUE file type")?
            };
            file = (kind.trim() == "BINARY").then(|| name.to_owned());
            format = None;
        } else if let Some(value) = line.strip_prefix("TRACK ") {
            let kind = value
                .split_whitespace()
                .nth(1)
                .ok_or("Missing track mode")?;
            format = match kind {
                "MODE1/2048" => Some(SectorFormat::Iso),
                "MODE1/2352" => Some(SectorFormat::Mode1),
                "MODE2/2352" => Some(SectorFormat::Mode2),
                "MODE2/2336" => Some(SectorFormat::Mode2_2336),
                "AUDIO" => None,
                _ => return Err(format!("Unsupported CUE track mode {kind}")),
            };
        } else if let Some(time) = line.strip_prefix("INDEX 01 ")
            && let Some(format) = format
        {
            let parts = time
                .trim()
                .split(':')
                .map(str::parse::<u64>)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|_| "Invalid CUE index time")?;
            let [minutes, seconds, frames] = parts.as_slice() else {
                return Err("CUE index must be minutes:seconds:frames".into());
            };
            if *seconds >= 60 || *frames >= 75 {
                return Err("CUE index time out of range".into());
            }
            let start_sector = minutes
                .checked_mul(60)
                .and_then(|m| m.checked_add(*seconds))
                .and_then(|s| s.checked_mul(75))
                .and_then(|f| f.checked_add(*frames))
                .ok_or("CUE index overflow")?;
            return Ok(DataTrack {
                file: file.ok_or("Data track has no BINARY file")?,
                format,
                start_sector,
            });
        }
    }
    Err("CUE has no supported data track with INDEX 01".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_track_keeps_quoted_filename_and_skips_audio() {
        let track = data_track(
            "FILE \"audio.bin\" BINARY\nTRACK 01 AUDIO\nINDEX 01 00:00:00\n\
             FILE \"game track.bin\" BINARY\nTRACK 02 MODE2/2352\nINDEX 01 01:02:03\n",
        )
        .unwrap();
        assert_eq!(track.file, "game track.bin");
        assert_eq!(track.start_sector, 4653);
        assert_eq!(track.format, SectorFormat::Mode2);
        assert!(data_track("FILE x.bin BINARY\nTRACK 01 MODE2/2352\nINDEX 01 00:60:00").is_err());
    }
}
