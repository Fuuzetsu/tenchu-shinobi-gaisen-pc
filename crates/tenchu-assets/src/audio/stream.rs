use super::adpcm::Xa;
use crate::{ReadAt, Result, disc::Disc};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Track {
    pub path: String,
    pub channel: u8,
    pub start: u32,
    pub end: u32,
    pub repeat: bool,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub rate: u32,
    pub frames: Vec<[i16; 2]>,
}

pub struct Stream {
    decoder: Xa,
    sector: u32,
    end: u32,
    channel: u8,
    coding: Option<u8>,
}

impl Stream {
    pub fn open<S: ReadAt>(disc: &Disc<S>, track: &Track) -> Result<Self> {
        let file = disc.find(&track.path)?;
        if file.directory || track.start > track.end || track.end > file.size.div_ceil(2048) {
            return Err(format!("XA cue outside {}", track.path));
        }
        let sector = file
            .sector
            .checked_add(track.start)
            .ok_or("XA extent overflow")?;
        let end = file
            .sector
            .checked_add(track.end)
            .ok_or("XA extent overflow")?;
        Ok(Self {
            decoder: Xa::default(),
            sector,
            end,
            channel: track.channel,
            coding: None,
        })
    }

    pub fn next<S: ReadAt>(&mut self, disc: &Disc<S>) -> Result<Option<Clip>> {
        while self.sector < self.end {
            let sector = disc.xa_sector(self.sector)?;
            self.sector += 1;
            if sector[0] != 1 || sector[1] != self.channel || sector[2] & 0x24 != 0x24 {
                continue;
            }
            let coding = sector[3];
            if coding & !0x55 != 0 || coding & 0x40 != 0 {
                return Err(format!("Unsupported XA coding {coding:#04x}"));
            }
            if self.coding.is_some_and(|previous| previous != coding) {
                return Err("XA coding changes within a cue".into());
            }
            self.coding = Some(coding);
            let stereo = coding & 1 != 0;
            let eight_bit = coding & 16 != 0;
            let mut frames = Vec::new();
            for group in sector[8..8 + 18 * 128].chunks_exact(128) {
                frames.extend(self.decoder.group(group, stereo, eight_bit)?);
            }
            return Ok(Some(Clip {
                rate: if coding & 4 != 0 { 18900 } else { 37800 },
                frames,
            }));
        }
        Ok(None)
    }
}

impl Clip {
    pub fn load<S: ReadAt>(disc: &Disc<S>, track: &Track) -> Result<Self> {
        let mut stream = Stream::open(disc, track)?;
        let mut clip = Self {
            rate: 37800,
            frames: Vec::new(),
        };
        while let Some(sector) = stream.next(disc)? {
            clip.rate = sector.rate;
            clip.frames.extend(sector.frames);
        }
        if clip.frames.is_empty() && track.start != track.end {
            return Err(format!(
                "No XA audio in {} channel {}",
                track.path, track.channel
            ));
        }
        Ok(clip)
    }
}
