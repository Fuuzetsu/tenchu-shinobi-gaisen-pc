use crate::{ReadAt, Result, check_span, text, u32_be, u32_le};

const BLOCK: u64 = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectorFormat {
    Iso,
    Mode1,
    Mode2,
    Mode2_2336,
}

impl SectorFormat {
    fn layout(self) -> (u64, u64) {
        match self {
            Self::Iso => (2048, 0),
            Self::Mode1 => (2352, 16),
            Self::Mode2 => (2352, 24),
            Self::Mode2_2336 => (2336, 8),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub sector: u32,
    pub size: u32,
    pub directory: bool,
}

impl Entry {
    fn parse(record: &[u8]) -> Result<Self> {
        if record.len() < 34 || record[0] as usize != record.len() {
            return Err("Invalid ISO directory record length".into());
        }
        if record[1] != 0 || record[25] & 0x80 != 0 || record[26] != 0 || record[27] != 0 {
            return Err("Unsupported ISO extended, interleaved or multi-extent entry".into());
        }
        let sector = both_u32(record, 2)?;
        let size = both_u32(record, 10)?;
        let name = record
            .get(33..33 + record[32] as usize)
            .ok_or("Truncated ISO filename")?;
        let name = match name {
            [0] => ".".to_owned(),
            [1] => "..".to_owned(),
            _ => text(name)?.split(';').next().unwrap().to_ascii_uppercase(),
        };
        Ok(Self {
            name,
            sector,
            size,
            directory: record[25] & 2 != 0,
        })
    }
}

pub struct Disc<S> {
    source: S,
    format: SectorFormat,
    start_sector: u64,
    length: u64,
    root: Entry,
}

impl<S: ReadAt> Disc<S> {
    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn start_sector(&self) -> u64 {
        self.start_sector
    }

    pub fn open(source: S, format: SectorFormat, start_sector: u64) -> Result<Self> {
        let length = source.byte_len() / format.layout().0;
        let mut disc = Self {
            source,
            format,
            start_sector,
            length: length
                .checked_sub(start_sector)
                .ok_or("CUE index outside image")?
                * BLOCK,
            root: Entry {
                name: "/".into(),
                sector: 0,
                size: 0,
                directory: true,
            },
        };
        for sector in 16..48 {
            let data = disc.read(sector * BLOCK, BLOCK as usize)?;
            if &data[1..6] != b"CD001" || data[6] != 1 {
                return Err("Invalid ISO-9660 volume descriptor".into());
            }
            match data[0] {
                1 => {
                    if data[128..132] != [0, 8, 8, 0] {
                        return Err("Only 2048-byte ISO logical blocks are supported".into());
                    }
                    let volume_length = u64::from(both_u32(&data, 80)?) * BLOCK;
                    // Mixed-mode discs can include later audio tracks in the
                    // volume size. File extents must still fit this data track.
                    disc.length = volume_length.min(disc.length);
                    disc.root = Entry::parse(&data[156..156 + data[156] as usize])?;
                    if !disc.root.directory {
                        return Err("ISO root is not a directory".into());
                    }
                    return Ok(disc);
                }
                255 => break,
                _ => {}
            }
        }
        Err("No primary ISO-9660 volume descriptor".into())
    }

    pub fn detect(source: S) -> Result<Self> {
        for format in [
            SectorFormat::Iso,
            SectorFormat::Mode2,
            SectorFormat::Mode1,
            SectorFormat::Mode2_2336,
        ] {
            let (stride, header) = format.layout();
            let mut signature = [0; 7];
            if source
                .read_exact_at(16 * stride + header, &mut signature)
                .is_ok()
                && &signature[1..6] == b"CD001"
                && signature[6] == 1
            {
                return Self::open(source, format, 0);
            }
        }
        Err("Not a supported ISO or raw PlayStation disc image".into())
    }

    pub fn format(&self) -> SectorFormat {
        self.format
    }

    pub fn xa_sector(&self, sector: u32) -> Result<[u8; 2336]> {
        check_span(u64::from(sector) * BLOCK, BLOCK, self.length)?;
        let (stride, offset) = match self.format {
            SectorFormat::Mode2 => (2352, 16),
            SectorFormat::Mode2_2336 => (2336, 0),
            _ => return Err("XA audio requires a raw Mode 2 BIN/CUE image".into()),
        };
        let mut bytes = [0; 2336];
        self.source.read_exact_at(
            (self.start_sector + u64::from(sector)) * stride + offset,
            &mut bytes,
        )?;
        if bytes[..4] != bytes[4..8] {
            return Err(format!("Inconsistent XA subheaders at sector {sector}"));
        }
        Ok(bytes)
    }

    pub fn entries(&self, directory: &Entry) -> Result<Vec<Entry>> {
        if !directory.directory || directory.size > 16 * 1024 * 1024 {
            return Err("Invalid or excessive ISO directory".into());
        }
        let data = self.file(directory).read(0, directory.size as usize)?;
        let mut entries = Vec::new();
        let mut at = 0;
        while at < data.len() {
            let size = data[at] as usize;
            if size == 0 {
                at = (at / BLOCK as usize + 1) * BLOCK as usize;
                continue;
            }
            if at % BLOCK as usize + size > BLOCK as usize {
                return Err("ISO directory record crosses a sector".into());
            }
            let entry = Entry::parse(data.get(at..at + size).ok_or("Truncated ISO directory")?)?;
            if entry.name != "." && entry.name != ".." {
                entries.push(entry);
            }
            at += size;
        }
        Ok(entries)
    }

    pub fn find(&self, path: &str) -> Result<Entry> {
        let mut current = self.root.clone();
        for component in path.split('/').filter(|p| !p.is_empty()) {
            if component == "." || component == ".." {
                return Err("Disc paths cannot traverse parent directories".into());
            }
            current = self
                .entries(&current)?
                .into_iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(component))
                .ok_or_else(|| format!("Missing disc file {path}"))?;
        }
        Ok(current)
    }

    pub fn file<'a>(&'a self, entry: &'a Entry) -> DiscFile<'a, S> {
        DiscFile { disc: self, entry }
    }
}

impl<S: ReadAt> ReadAt for Disc<S> {
    fn byte_len(&self) -> u64 {
        self.length
    }

    fn read_exact_at(&self, offset: u64, mut output: &mut [u8]) -> Result<()> {
        check_span(offset, output.len() as u64, self.length)?;
        let (stride, header) = self.format.layout();
        let mut logical = offset;
        while !output.is_empty() {
            let within = logical % BLOCK;
            let count = output.len().min((BLOCK - within) as usize);
            let physical = (self.start_sector + logical / BLOCK) * stride;
            if matches!(self.format, SectorFormat::Mode2 | SectorFormat::Mode2_2336) {
                let mut submode = [0];
                self.source
                    .read_exact_at(physical + header - 6, &mut submode)?;
                if submode[0] & 0x20 != 0 {
                    return Err("XA Form 2 sector requires the audio/movie stream reader".into());
                }
            }
            self.source
                .read_exact_at(physical + header + within, &mut output[..count])?;
            output = &mut output[count..];
            logical += count as u64;
        }
        Ok(())
    }
}

pub struct DiscFile<'a, S> {
    disc: &'a Disc<S>,
    entry: &'a Entry,
}

impl<S: ReadAt> ReadAt for DiscFile<'_, S> {
    fn byte_len(&self) -> u64 {
        u64::from(self.entry.size)
    }
    fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        check_span(offset, output.len() as u64, self.byte_len())?;
        self.disc
            .read_exact_at(u64::from(self.entry.sector) * BLOCK + offset, output)
    }
}

fn both_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let little = u32_le(bytes, at)?;
    if little != u32_be(bytes, at + 4)? {
        return Err("Inconsistent ISO byte orders".into());
    }
    Ok(little)
}

#[cfg(test)]
mod tests;
