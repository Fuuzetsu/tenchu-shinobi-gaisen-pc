use super::{Edition, executable, music, volume::Volume};
use crate::{
    ReadAt, Result, audio::stream::Track, disc::Disc, executable::Executable, frontend::Image,
    u16_le, u32_le,
};

pub struct Introduction {
    pub heading: Image,
    pub subtitle: Image,
    pub section: usize,
    pub narration: Option<Narration>,
}

pub struct Results {
    pub panel: Image,
    pub numbers: Image,
    pub ranks: Image,
}

impl Results {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        if Edition::detect(disc)? != Edition::Japan {
            return Err("Native Tenchu 2 results require the Japanese disc".into());
        }
        let volume = Volume::open(disc)?;
        let load = |name: &str, transparent: bool| -> Result<Image> {
            let bytes = volume.read(r"o:\cdimage\game\image", name)?;
            let mut image = Image::parse(&bytes)?;
            if transparent {
                let color = image.palette_color(15).ok_or("Missing results palette")?;
                image.recolor(15, color | 0x8000);
            }
            super::texture::enable_image_blending(&mut image);
            Ok(image)
        };
        let results = Self {
            panel: load("result.tim", true)?,
            numbers: load("rsltnum.tim", true)?,
            ranks: load("rank.tim", false)?,
        };
        if results.numbers.size[0] != 12
            || !results.numbers.size[1].is_multiple_of(12)
            || !results.ranks.size[1].is_multiple_of(7)
        {
            return Err("Invalid Japanese results image sheets".into());
        }
        Ok(results)
    }
}

pub struct Narration {
    pub background: Image,
    pub text: Image,
    pub audio: Track,
}

impl Introduction {
    pub fn load(disc: &Disc<impl ReadAt>, resource: u16) -> Result<Self> {
        let (edition, bytes) = executable(disc)?;
        if edition != Edition::Japan {
            return Err("Native Tenchu 2 introductions require the Japanese disc".into());
        }
        let exe = Executable::new(&bytes)?;
        let (section, cue) = match resource {
            2 => (0, Some(1)),
            3 => (1, Some(2)),
            4 => (2, Some(3)),
            8 => (2, Some(4)),
            9 => (0, None),
            10 => (1, Some(5)),
            11 => (2, Some(6)),
            _ => return Err(format!("Unknown Japanese Tenchu 2 introduction {resource}")),
        };
        let volume = Volume::open(disc)?;
        let image = |name: &str| -> Result<Image> {
            super::texture::image(&volume.read(r"o:\cdimage\game\image\intro", name)?)
        };
        let heading = image("introx.tim")?;
        let subtitle = image("introxx.tim")?;
        if heading.size[1] % 6 != 0 || subtitle.size[1] % 3 != 0 {
            return Err("Invalid Japanese Tenchu 2 title sheet".into());
        }
        let narration = if let Some(cue) = cue {
            let path = "TENCHU2/XA/NARAT_J.XA";
            let file = disc.find(path)?;
            let mut audio = None;
            for i in 0..64 {
                let row = exe.bytes(0x800812e4 + i * 8, 8)?;
                let id = u16_le(row, 0)?;
                if id == 0 {
                    break;
                }
                if id == cue {
                    let start = music::time(&row[4..6])?;
                    let end = music::time(&row[6..8])?;
                    if row[3] > 31 || start >= end || end > file.size.div_ceil(2048) {
                        return Err("Invalid Japanese Tenchu 2 narration cue".into());
                    }
                    audio = Some(Track {
                        path: path.into(),
                        channel: row[3],
                        start,
                        end,
                        repeat: false,
                    });
                    break;
                }
            }
            let text = image(&format!("intro{resource}.tim"))?;
            if text.size[0] != 256 {
                return Err("Invalid Japanese Tenchu 2 narration columns".into());
            }
            Some(Narration {
                background: image(&format!("nback{resource}.tim"))?,
                text,
                audio: audio.ok_or("Missing Japanese Tenchu 2 narration cue")?,
            })
        } else {
            None
        };
        Ok(Self {
            heading,
            subtitle,
            section,
            narration,
        })
    }
}

pub struct Movie {
    pub audio: Track,
    pub last_frame: u32,
}

impl Movie {
    pub fn load(disc: &Disc<impl ReadAt>, resource: u16) -> Result<Option<Self>> {
        let (edition, bytes) = executable(disc)?;
        if edition != Edition::Japan || resource > 12 {
            return Err("Unsupported Japanese Tenchu 2 movie".into());
        }
        let exe = Executable::new(&bytes)?;
        let last_frame = u32_le(exe.bytes(0x80085ba0 + u32::from(resource) * 4, 4)?, 0)?;
        if last_frame == 0 {
            return Ok(None);
        }
        if !(3..=15000).contains(&last_frame) {
            return Err("Invalid Japanese Tenchu 2 movie length".into());
        }
        let path = format!("TENCHU2/MOVIE/MOV{resource:02}S.STR");
        let file = disc.find(&path)?;
        Ok(Some(Self {
            audio: Track {
                path,
                channel: 1,
                start: 0,
                end: file.size.div_ceil(2048),
                repeat: false,
            },
            last_frame,
        }))
    }
}
