use super::*;
use tenchu_assets::audio::{
    bank::Bank,
    voice::{Dialect, VoiceSet},
};
use tenchu_assets::script::Language;

#[cfg(test)]
mod tests;

pub(super) const LANGUAGES: [(Language, &str, &str); 4] = [
    (Language::English, "en", "English"),
    (Language::French, "fr", "French"),
    (Language::Italian, "it", "Italian"),
    (Language::Japanese, "ja", "Japanese"),
];

impl Library {
    pub(super) fn load_voices(&mut self, assets: &GameAssets<impl ReadAt>) -> Result<()> {
        let mut groups: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (i, body) in self.bodies.iter().enumerate() {
            if let Some(voice) = body.voice {
                groups
                    .entry((voice.stage, voice.character))
                    .or_default()
                    .push((i, voice.program));
            }
        }
        for ((stage, character), bodies) in groups {
            for (language, code, _) in LANGUAGES {
                let bank = Bank::load(assets, stage, character, language)?;
                let mut programs: BTreeMap<usize, Arc<VoiceSet>> = BTreeMap::new();
                for &(index, program) in &bodies {
                    let voice = if let Some(voice) = programs.get(&program) {
                        voice.clone()
                    } else {
                        let voice = Arc::new(VoiceSet::extract(&bank, program, Dialect::Gaisen)?);
                        programs.insert(program, voice.clone());
                        voice
                    };
                    self.voices
                        .insert((self.bodies[index].id.clone(), code.into()), voice.clone());
                    if language == Language::Japanese {
                        self.bodies[index].voice_set = Some(voice.clone());
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn voice_in(&self, id: &str, language: Language) -> Result<Option<&Arc<VoiceSet>>> {
        if id.is_empty() {
            return Ok(None);
        }
        let (code, body_id) = if let Some(selected) = id.strip_prefix("voice:") {
            selected.split_once(':').ok_or("Invalid voice selection")?
        } else {
            (
                LANGUAGES
                    .iter()
                    .find(|&&(l, _, _)| l == language)
                    .unwrap()
                    .1,
                id,
            )
        };
        let voice = self.get(body_id).and_then(|body| {
            if body.is_tenchu2() && body_id == id {
                body.moveset_voice()
            } else {
                self.voices.get(&(body.id.clone(), code.into()))
            }
        });
        voice
            .map(Some)
            .ok_or_else(|| format!("Voice unavailable: {id}"))
    }
}

impl Body {
    pub fn moveset_voice(&self) -> Option<&Arc<VoiceSet>> {
        self.imported_moveset
            .as_ref()
            .map_or(self.voice_set.as_ref(), |m| m.voice.as_ref())
    }
}
