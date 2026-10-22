use crate::{ReadAt, Result, check_span};
use std::collections::BTreeMap;

pub const PAGE_SIZE: usize = 64 * 1024;
const MAGIC: &[u8; 8] = b"TNCHPAGE";

/// A sparse, immutable-page disc cache. Missing pages fail instead of fetching in a game tick.
pub struct Cache {
    length: u64,
    pages: BTreeMap<u32, Vec<u8>>,
}

impl Cache {
    pub fn new(length: u64) -> Result<Self> {
        if length == 0 || length > 1024 * 1024 * 1024 {
            return Err("Invalid cached disc size".into());
        }
        Ok(Self {
            length,
            pages: BTreeMap::new(),
        })
    }

    pub fn insert(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.get(..8) != Some(MAGIC) {
            return Err("Invalid disc page bundle".into());
        }
        let mut at = 8;
        let mut pending = BTreeMap::new();
        while at < bytes.len() {
            let header = crate::slice(bytes, at, 8)?;
            let index = crate::u32_le(header, 0)?;
            let size = crate::u32_le(header, 4)? as usize;
            let offset = u64::from(index) * PAGE_SIZE as u64;
            check_span(offset, size as u64, self.length)?;
            if size != PAGE_SIZE.min((self.length - offset) as usize) {
                return Err("Invalid cached page length".into());
            }
            at += 8;
            let page = crate::slice(bytes, at, size)?;
            if self.pages.get(&index).is_some_and(|old| old != page)
                || pending.insert(index, page).is_some_and(|old| old != page)
            {
                return Err("Conflicting disc page".into());
            }
            at += size;
        }
        for (index, page) in pending {
            self.pages.entry(index).or_insert_with(|| page.to_vec());
        }
        Ok(())
    }

    pub fn bundle(source: &impl ReadAt, pages: impl IntoIterator<Item = u32>) -> Result<Vec<u8>> {
        let mut output = MAGIC.to_vec();
        for index in pages {
            let offset = u64::from(index) * PAGE_SIZE as u64;
            check_span(offset, 1, source.byte_len())?;
            let size = PAGE_SIZE.min((source.byte_len() - offset) as usize);
            output.extend(index.to_le_bytes());
            output.extend((size as u32).to_le_bytes());
            output.extend(source.read(offset, size)?);
        }
        Ok(output)
    }
}

impl ReadAt for Cache {
    fn byte_len(&self) -> u64 {
        self.length
    }

    fn read_exact_at(&self, mut offset: u64, mut output: &mut [u8]) -> Result<()> {
        check_span(offset, output.len() as u64, self.length)?;
        while !output.is_empty() {
            let index = (offset / PAGE_SIZE as u64) as u32;
            let page = self
                .pages
                .get(&index)
                .ok_or_else(|| format!("Disc page {index} is not ready"))?;
            let within = offset as usize % PAGE_SIZE;
            let size = output.len().min(page.len() - within);
            output[..size].copy_from_slice(&page[within..within + size]);
            output = &mut output[size..];
            offset += size as u64;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_reads_require_complete_pages_and_reject_corruption_atomically() {
        let bytes: Vec<_> = (0..PAGE_SIZE * 2 + 17).map(|i| (i * 31) as u8).collect();
        let mut cache = Cache::new(bytes.len() as u64).unwrap();
        cache
            .insert(&Cache::bundle(&bytes, [0, 2]).unwrap())
            .unwrap();
        assert!(cache.read((PAGE_SIZE - 2) as u64, 8).is_err());
        let mut second = Cache::bundle(&bytes, [1]).unwrap();
        second.push(0);
        assert!(cache.insert(&second).is_err());
        assert!(cache.read(PAGE_SIZE as u64, 1).is_err());
        second.pop();
        cache.insert(&second).unwrap();
        assert_eq!(cache.read(0, bytes.len()).unwrap(), bytes);
        let mut conflicting = Cache::bundle(&bytes, [0]).unwrap();
        conflicting[16] ^= 1;
        assert!(cache.insert(&conflicting).is_err());
        let mut duplicate = Cache::bundle(&bytes, [0]).unwrap();
        duplicate.extend_from_slice(&conflicting[8..]);
        let mut empty = Cache::new(bytes.len() as u64).unwrap();
        assert!(empty.insert(&duplicate).is_err());
        assert!(empty.read(0, 1).is_err());
        assert!(cache.read(bytes.len() as u64, 1).is_err());
    }
}
