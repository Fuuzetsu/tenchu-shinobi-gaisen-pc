use crate::{ReadAt, Result, check_span, u32_be};
use encoding_rs::SHIFT_JIS;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Entry {
    pub offset: u64,
    pub size: u32,
}

pub struct Afs {
    pub files: BTreeMap<String, Entry>,
}

impl Afs {
    pub fn read(source: &impl ReadAt) -> Result<Self> {
        let header = source.read(0, 20)?;
        if !header.starts_with(b"AFS_VOL_200") {
            return Err("Not an AFS_VOL_200 volume".into());
        }
        let count = u32_be(&header, 12)? as usize;
        if count > 100_000 {
            return Err("Excessive AFS entry count".into());
        }
        let table = source.read(u64::from(u32_be(&header, 16)?), count * 36)?;
        let mut directories = BTreeMap::<usize, String>::new();
        let mut files = BTreeMap::new();
        for (i, row) in table.chunks_exact(36).enumerate() {
            if &row[..2] != b"IX" {
                return Err(format!("Invalid AFS record {i}"));
            }
            let name = &row[16..36];
            let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
            let name = SHIFT_JIS
                .decode_without_bom_handling_and_without_replacement(&name[..end])
                .ok_or("Invalid Shift-JIS AFS filename")?;
            let path = if let Some(rest) = name.strip_prefix('@') {
                let (parent, name) = rest.split_once('_').ok_or("Invalid AFS parent reference")?;
                let parent: usize = parent.parse().map_err(|_| "Invalid AFS parent index")?;
                format!(
                    "{}/{}",
                    directories
                        .get(&parent)
                        .ok_or("Missing AFS parent directory")?,
                    name
                )
            } else {
                name.into_owned()
            };
            let path = path.replace('\\', "/").to_ascii_uppercase();
            match u16::from_be_bytes([row[2], row[3]]) {
                2 => {
                    directories.insert(i, path);
                }
                1 => {
                    let offset = u64::from(u32_be(row, 4)?);
                    let stored = u32_be(row, 8)?;
                    let size = u32_be(row, 12)?;
                    if stored != size {
                        return Err(format!("Compressed AFS entry: {path}"));
                    }
                    check_span(offset, u64::from(size), source.byte_len())?;
                    if files.insert(path.clone(), Entry { offset, size }).is_some() {
                        return Err(format!("Duplicate AFS path {path}"));
                    }
                }
                flag => return Err(format!("Unknown AFS record type {flag}")),
            }
        }
        Ok(Self { files })
    }

    pub fn get(&self, relative: &str) -> Result<&Entry> {
        let relative = relative.replace('\\', "/").to_ascii_uppercase();
        let path = if relative.starts_with("K:/") {
            relative
        } else {
            format!("K:/WORK/CDIMAGE/{}", relative.trim_matches('/'))
        };
        self.files
            .get(&path)
            .ok_or_else(|| format!("Missing AFS asset {path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(data: &mut [u8], index: usize, kind: u16, name: &str, offset: u32, size: u32) {
        let p = 32 + index * 36;
        data[p..p + 2].copy_from_slice(b"IX");
        data[p + 2..p + 4].copy_from_slice(&kind.to_be_bytes());
        for (at, value) in [(4, offset), (8, size), (12, size)] {
            data[p + at..p + at + 4].copy_from_slice(&value.to_be_bytes());
        }
        data[p + 16..p + 16 + name.len()].copy_from_slice(name.as_bytes());
    }

    #[test]
    fn parent_numbers_are_record_indices_and_duplicate_basenames_are_distinct() {
        let mut data = vec![0; 512];
        data[..11].copy_from_slice(b"AFS_VOL_200");
        data[12..16].copy_from_slice(&5u32.to_be_bytes());
        data[16..20].copy_from_slice(&32u32.to_be_bytes());
        row(&mut data, 0, 2, "K:/WORK/CDIMAGE", 0, 0);
        row(&mut data, 1, 1, "@0_COMMON.AMD", 400, 2);
        row(&mut data, 2, 2, "@0_AKINDO", 0, 0);
        row(&mut data, 3, 1, "@2_COMMON.AMD", 402, 3);
        row(&mut data, 4, 1, "@0_LAST", 405, 1);
        let afs = Afs::read(&data).unwrap();
        assert_eq!(afs.get("common.amd").unwrap().offset, 400);
        assert_eq!(afs.get("AKINDO/COMMON.AMD").unwrap().offset, 402);
        let name = b"@0_\xba\xcb\xdf\xb0 \x81\x60 STAGE.";
        data[192..212].fill(0);
        data[192..192 + name.len()].copy_from_slice(name);
        assert_eq!(
            Afs::read(&data)
                .unwrap()
                .get("ｺﾋﾟｰ ～ STAGE.")
                .unwrap()
                .offset,
            405
        );
        row(&mut data, 4, 1, "@0_LAST", 511, 2);
        assert!(Afs::read(&data).is_err());
    }
}
