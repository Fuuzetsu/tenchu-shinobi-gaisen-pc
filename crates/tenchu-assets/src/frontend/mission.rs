use crate::{
    Result, audio::catalog::Catalog, audio::stream::Track, executable::Executable, i16_le,
    script::Language, u16_le, u32_le,
};

pub struct Briefing {
    pub background: String,
    pub text: String,
    pub narration: Track,
    pub hold: i16,
    pub scroll_step: i16,
}

pub struct Screens {
    pub shop: String,
    pub help: String,
    pub results: String,
    pub ranks: String,
    pub training: String,
    pub training_ranks: String,
}

fn language_index(language: Language) -> u32 {
    match language {
        Language::English => 0,
        Language::French => 1,
        Language::Italian => 2,
        Language::Japanese => 3,
    }
}

impl Screens {
    pub fn load(bytes: &[u8], language: Language) -> Result<Self> {
        let exe = Executable::new(bytes)?;
        let path = |table| {
            exe.string(u32_le(
                exe.bytes(table + language_index(language) * 4, 4)?,
                0,
            )?)
        };
        Ok(Self {
            shop: path(0x8008ef28)?,
            help: path(0x800138f8)?,
            results: path(0x8008ef38)?,
            ranks: path(0x8008ef48)?,
            training: path(0x8008ef68)?,
            training_ranks: path(0x8008ef58)?,
        })
    }
}

impl Briefing {
    pub fn load(bytes: &[u8], language: Language, stage: usize, character: i16) -> Result<Self> {
        if stage >= 11 || !(0..=1).contains(&character) {
            return Err("Invalid mission briefing selection".into());
        }
        let exe = Executable::new(bytes)?;
        let index = language_index(language) * 11 + stage as u32;
        let row = exe.bytes(0x8008ea90 + index * 12, 12)?;
        let path = |offset| -> Result<String> {
            Ok(format!("DEMO/{}", exe.string(u32_le(row, offset)?)?))
        };
        let mut cue = i32::from(u16_le(row, 8)?);
        if character == 1 && language == Language::Japanese && (6..=7).contains(&stage) {
            cue += 1;
        }
        let narration = Catalog::load(bytes, language)?
            .music(cue)
            .cloned()
            .ok_or_else(|| format!("Missing briefing narration cue {cue}"))?;
        Ok(Self {
            background: path(0)?,
            text: path(4)?,
            narration,
            hold: i16_le(exe.bytes(0x8008eca0 + index * 2, 2)?, 0)?,
            scroll_step: i16_le(exe.bytes(0x8008ecf8 + index * 2, 2)?, 0)?,
        })
    }
}
