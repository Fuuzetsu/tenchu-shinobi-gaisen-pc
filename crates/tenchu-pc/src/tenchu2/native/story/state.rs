use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::texture::Vram;

const SIGNATURE: &str = "native-tenchu2-jp-story-1";

#[derive(Serialize, Deserialize)]
pub(in crate::tenchu2::native) struct State<P, V> {
    signature: String,
    pub(in crate::tenchu2::native) sequence: usize,
    event: u16,
    playback: P,
    textures: BTreeMap<i16, V>,
    interpolation: Option<crate::visuals::interpolation::History>,
    models: super::super::models::story::Override,
    contact: super::super::contact::History,
}

pub(in crate::tenchu2::native) type Owned = State<Playback, Vram>;
pub(in crate::tenchu2::native) type Borrowed<'a> = State<&'a Playback, &'a Vram>;

impl Story {
    pub(in crate::tenchu2::native) fn snapshot(&self) -> Borrowed<'_> {
        let Self {
            sequence,
            event,
            resources,
            level: _,
            math: _,
            state,
            interpolation,
            models,
            sprites: _,
            contact,
            rules: _,
            basis: _,
        } = self;
        State {
            signature: SIGNATURE.into(),
            sequence: *sequence,
            event: *event,
            playback: state,
            textures: resources
                .models
                .iter()
                .map(|(&id, model)| (id, &model.vram))
                .collect(),
            interpolation: interpolation.clone(),
            models: models.clone(),
            contact: contact.clone(),
        }
    }

    pub fn capture_state(&self) -> Result<Vec<u8>> {
        crate::savestate::encode(&self.snapshot())
    }

    pub fn restore_state(&mut self, disc: &Disc<impl ReadAt>, bytes: &[u8]) -> Result<()> {
        let saved: Owned = crate::savestate::decode(bytes)?;
        let campaign = tenchu_assets::tenchu2::campaign::Campaign::load(disc)?;
        *self = saved.restore(disc, &campaign)?;
        Ok(())
    }
}

impl Owned {
    pub(in crate::tenchu2::native) fn restore(
        self,
        disc: &Disc<impl ReadAt>,
        campaign: &tenchu_assets::tenchu2::campaign::Campaign,
    ) -> Result<Story> {
        if self.signature != SIGNATURE {
            return Err("Not a native T2 cutscene state".into());
        }
        if !campaign.sequences.get(self.sequence).is_some_and(|row| {
            row.kind == tenchu_assets::tenchu2::campaign::Kind::Event && row.resource == self.event
        }) {
            return Err("Invalid native T2 cutscene identity".into());
        }
        let mut restored = Story::load(disc, self.sequence, self.event, 1)?;
        self.validate(&restored.resources)?;
        let State {
            playback,
            textures,
            interpolation,
            models,
            contact,
            ..
        } = self;
        for (id, vram) in textures {
            restored.resources.models.get_mut(&id).unwrap().vram = vram;
        }
        restored.state = playback;
        restored.interpolation = interpolation;
        restored.models = models;
        restored.contact = contact;
        Ok(restored)
    }
}

impl Owned {
    fn validate(&self, resources: &Resources) -> Result<()> {
        let state = &self.playback;
        if !(-1..=resources.script.duration as i16).contains(&state.frame)
            || state.actors.len() != resources.script.actors.len()
            || !self.textures.keys().eq(resources.models.keys())
            || self.textures.values().any(|v| v.words.len() != 1024 * 512)
            || state
                .presentation
                .fade
                .is_some_and(|fade| fade.duration <= 0)
            || state
                .presentation
                .caption
                .is_some_and(|caption| caption.command >= resources.script.commands.len())
            || state
                .overlay
                .caption
                .is_some_and(|index| index >= resources.script.commands.len())
            || state
                .overlay
                .effects
                .iter()
                .any(|&index| index >= resources.script.commands.len())
        {
            return Err("Invalid native T2 cutscene playback or textures".into());
        }
        state.camera.validate(&resources.script.camera)?;
        for (motion, animation) in state.actors.iter().zip(&resources.script.actors) {
            motion.validate(animation)?;
        }
        let faces = tenchu_core::tenchu2::story::Faces::new(resources);
        if state.faces.actors.len() != faces.actors.len()
            || state
                .faces
                .actors
                .iter()
                .zip(&faces.actors)
                .any(|(saved, original)| {
                    saved.model != original.model
                        || saved.slot != original.slot
                        || saved.held.iter().enumerate().any(|(channel, &index)| {
                            usize::from(index) + channel * 16 >= resources.faces[&saved.model].len()
                        })
                })
            || state.patches.iter().any(|(model, indices)| {
                resources.faces.get(model).is_none_or(|patches| {
                    indices
                        .iter()
                        .flatten()
                        .any(|&id| usize::from(id) >= patches.len())
                })
            })
        {
            return Err("Invalid native T2 facial texture state".into());
        }
        self.models.validate_state(resources)
    }
}

#[cfg(test)]
mod tests;
