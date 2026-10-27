use crate::{
    ReadAt, Result,
    afs::Afs,
    disc::{Disc, Entry},
    i16_le, text, u32_le,
};
use sha2::{Digest, Sha256};

pub const EXE_SHA256: &str = "0690a5c14ff3e975ebcb3de26e196a4dbb6afc992677d0de2cbcf86af9993558";
const STAGE_CONFIG: u32 = 0x80011f18;

const CONTENTS: [(&str, u64, &str); 3] = [
    (
        "TENCHU/DATA.VOL",
        87_805_871,
        "aaaf61e8df377db929ff053bec855c282c525567583c2399292b9684ffab79bd",
    ),
    (
        "TENCHU/MENU.EXE",
        307_200,
        "5de759508d87fa3bd3122329eaaf00467ff5f4d964a1673699309f3b6983c5ab",
    ),
    (
        "TENCHU/ENDING.EXE",
        264_192,
        "b7ada48dbc9d78098bd2e9fbf8589bdbcb1c64d963abc6282cd65855bf2bb606",
    ),
];

#[derive(Clone, Debug)]
pub struct Stage {
    pub id: usize,
    pub order: i32,
    pub name: String,
    pub directory: String,
    pub start: [i32; 3],
    pub facing: i32,
    pub reward: usize,
}

pub struct GameAssets<S> {
    pub disc: Disc<S>,
    pub executable: Vec<u8>,
    pub archive: Afs,
    volume: Entry,
}

impl<S: ReadAt> GameAssets<S> {
    pub fn open(disc: Disc<S>) -> Result<Self> {
        let entry = disc.find("TENCHU/MAIN.EXE")?;
        if entry.size > 4 * 1024 * 1024 {
            return Err("Excessive executable size".into());
        }
        let executable = disc.file(&entry).read(0, entry.size as usize)?;
        let hash = format!("{:x}", Sha256::digest(&executable));
        if hash != EXE_SHA256 {
            return Err(format!(
                "Unsupported MAIN.EXE {hash}; expected original SLPS-019.01"
            ));
        }
        let volume = disc.find("TENCHU/DATA.VOL")?;
        let archive = Afs::read(&disc.file(&volume))?;
        Ok(Self {
            disc,
            executable,
            archive,
            volume,
        })
    }

    pub fn asset(&self, name: &str) -> Result<Vec<u8>> {
        let entry = self.archive.get(name)?;
        self.disc
            .file(&self.volume)
            .read(entry.offset, entry.size as usize)
    }

    /// Full base-artwork verification, for native startup and browser asset preparation.
    /// Browser startup uses verified pages as they stream in instead.
    pub fn verify_contents(&self) -> Result<()> {
        for (name, size, hash) in CONTENTS {
            let entry = self.disc.find(name)?;
            verify_file(&self.disc.file(&entry), size, hash)
                .map_err(|error| format!("{name}: {error}"))?;
        }
        Ok(())
    }

    pub fn stages(&self) -> Result<Vec<Stage>> {
        let exe = &self.executable;
        let base = u32_le(exe, 0x18)?;
        let offset = |address: u32| -> Result<usize> {
            let at = address.checked_sub(base).ok_or("EXE address before base")? as usize + 0x800;
            if at >= exe.len() {
                return Err("EXE address outside file".into());
            }
            Ok(at)
        };
        let string = |address| text(&exe[offset(address)?..]);
        let start = offset(STAGE_CONFIG)?;
        (0..11)
            .map(|id| {
                let p = start + id * 28;
                Ok(Stage {
                    id,
                    order: u32_le(exe, p)? as i32,
                    name: string(u32_le(exe, p + 4)?)?
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                    directory: string(u32_le(exe, p + 8)?)?
                        .replace('\\', "/")
                        .trim_end_matches('/')
                        .rsplit('/')
                        .next()
                        .unwrap()
                        .to_owned(),
                    start: [
                        u32_le(exe, p + 12)? as i32,
                        u32_le(exe, p + 16)? as i32,
                        u32_le(exe, p + 20)? as i32,
                    ],
                    facing: u32_le(exe, p + 24)? as i32,
                    reward: i16_le(exe, offset(0x8008ed50)? + id * 2)? as usize,
                })
            })
            .collect()
    }
}

fn verify_file(source: &impl ReadAt, size: u64, expected: &str) -> Result<()> {
    if source.byte_len() != size {
        return Err(format!(
            "Unexpected size {}; expected {size} bytes for SLPS-019.01",
            source.byte_len()
        ));
    }
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut offset = 0;
    while offset < size {
        let count = (size - offset).min(buffer.len() as u64) as usize;
        source.read_exact_at(offset, &mut buffer[..count])?;
        hash.update(&buffer[..count]);
        offset += count as u64;
    }
    let actual = format!("{:x}", hash.finalize());
    if actual != expected {
        return Err(format!(
            "SHA-256 mismatch: {actual}; expected {expected}. Use an unmodified Shinobi Gaisen SLPS-019.01 disc; put custom artwork in a replacement pack."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_detects_damage_beyond_the_first_read_and_truncation() {
        let mut bytes: Vec<_> = (0..150_001).map(|i| (i % 251) as u8).collect();
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let size = bytes.len() as u64;
        verify_file(&bytes, size, &hash).unwrap();
        bytes[140_000] ^= 1;
        assert!(
            verify_file(&bytes, size, &hash)
                .unwrap_err()
                .contains("SHA-256")
        );
        bytes.pop();
        assert!(
            verify_file(&bytes, size, &hash)
                .unwrap_err()
                .contains("size")
        );
    }
}
