use super::*;
use tenchu_assets::audio::{
    bank::Bank,
    voice::{Dialect, VoiceSet},
};

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
            let bank = Bank::load(
                assets,
                stage,
                character,
                tenchu_assets::script::Language::Japanese,
            )?;
            let mut programs = BTreeMap::new();
            for (index, program) in bodies {
                let voice = if let Some(voice) = programs.get(&program) {
                    Arc::clone(voice)
                } else {
                    let voice = Arc::new(VoiceSet::extract(&bank, program, Dialect::Gaisen)?);
                    programs.insert(program, voice.clone());
                    voice
                };
                self.bodies[index].voice_set = Some(voice);
            }
        }
        Ok(())
    }
}

impl Body {
    pub fn moveset_voice(&self) -> Option<&Arc<VoiceSet>> {
        self.imported_moveset
            .as_ref()
            .map_or(self.voice_set.as_ref(), |m| m.voice.as_ref())
    }
}
