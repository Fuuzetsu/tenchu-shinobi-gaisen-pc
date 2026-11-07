use crate::{
    ReadAt, Result,
    disc::{Disc, Entry},
    u32_le,
};
use std::collections::BTreeMap;

pub(super) struct Volume<'a, S> {
    pub edition: super::Edition,
    disc: &'a Disc<S>,
    entry: Entry,
    files: BTreeMap<(u32, u32), (u32, u32)>,
}

impl<'a, S: ReadAt> Volume<'a, S> {
    pub fn open(disc: &'a Disc<S>) -> Result<Self> {
        let edition = super::Edition::detect(disc)?;
        let entry = disc.find("TENCHU2/VOLUME.DAT")?;
        let volume = disc.file(&entry);
        let header = volume.read(0, 16)?;
        let files = u32_le(&header, 8)? as usize;
        let directories = u32_le(&header, 12)? as usize;
        if u32_le(&header, 0)? != edition.addresses().volume_tag
            || files > 16384
            || directories > 1024
        {
            return Err("Invalid Tenchu 2 volume header".into());
        }
        let table = volume.read(
            (16 + directories * 8).next_multiple_of(2048) as u64,
            files * 16,
        )?;
        let mut files = BTreeMap::new();
        for row in table.chunks_exact(16) {
            let sector = u32_le(row, 0)?;
            let size = u32_le(row, 4)?;
            crate::check_span(u64::from(sector) * 2048, u64::from(size), volume.byte_len())?;
            let key = (u32_le(row, 8)?, u32_le(row, 12)?);
            if files.insert(key, (sector, size)).is_some() {
                return Err("Duplicate Tenchu 2 resource identity".into());
            }
        }
        Ok(Self {
            edition,
            disc,
            entry,
            files,
        })
    }

    pub fn optional(&self, directory: &str, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(&(sector, size)) = self.files.get(&(hash(directory), hash(name))) else {
            return Ok(None);
        };
        if size > 64 * 1024 * 1024 {
            return Err("Excessive Tenchu 2 resource size".into());
        }
        self.disc
            .file(&self.entry)
            .read(u64::from(sector) * 2048, size as usize)
            .map(Some)
    }

    pub fn read(&self, directory: &str, name: &str) -> Result<Vec<u8>> {
        self.optional(directory, name)?
            .ok_or_else(|| format!("Missing Tenchu 2 resource {directory}/{name}"))
    }
}

pub(super) fn hash(name: &str) -> u32 {
    let mut hash = 0u32;
    for (i, c) in name.bytes().enumerate() {
        for bit in 0..8 {
            hash = hash
                .wrapping_add(u32::from((c.to_ascii_uppercase() >> bit) & 1) << (bit * 4 + i % 4));
        }
    }
    hash
}
