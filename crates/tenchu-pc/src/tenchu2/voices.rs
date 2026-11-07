use tenchu_assets::{Result, executable::Executable, stage::imports::Voice};

// Sandbox combat still uses Gaisen's sound events. Match the body rather than
// the selected sword/spear/archer AI, which otherwise gives every actor a man's voice.
pub fn donor_kind(model: &str) -> Option<i16> {
    match model {
        "rikimaru" | "rikia" => Some(0),
        "ayame" | "ayamea" => Some(1),
        "yasha" | "karakuno" | "mizukuno" | "roukuno" | "kagami" | "kagami2" | "kagami3"
        | "byakko" | "yuki" | "kei" | "kiku" | "jochu" | "muraa" | "ol" | "kizoku" | "rouba" => {
            Some(0x22)
        }
        _ => None,
    }
}

pub fn donor(executable: &Executable<'_>, model: &str) -> Result<Option<Voice>> {
    let Some(kind) = donor_kind(model) else {
        return Ok(None);
    };
    if kind < 2 {
        return Ok(Some(Voice {
            stage: 0,
            character: kind,
            program: 5,
        }));
    }
    for stage in 0..11 {
        if let Some(program) = executable
            .appearance(stage)?
            .iter()
            .position(|&k| k == kind)
        {
            return Ok(Some(Voice {
                stage,
                character: 0,
                program: program + 6,
            }));
        }
    }
    Err(format!("Missing Gaisen voice donor {kind:#x}"))
}
