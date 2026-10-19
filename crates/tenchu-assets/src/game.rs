use crate::{
    ReadAt, Result,
    afs::Afs,
    disc::{Disc, Entry},
    i16_le, text, u32_le,
};
use sha2::{Digest, Sha256};

pub const EXE_SHA256: &str = "0690a5c14ff3e975ebcb3de26e196a4dbb6afc992677d0de2cbcf86af9993558";
const STAGE_CONFIG: u32 = 0x80011f18;

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
