use super::{Faces, Motion, Presentation, PresentationFrame, TextureChange};
use crate::{
    Result,
    math::{Math, Matrix},
    random::Random,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::tenchu2::story::{Resources, Script};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Playback {
    pub frame: i16,
    pub view: Matrix,
    pub camera: Motion,
    pub actors: Vec<Motion>,
    pub faces: Faces,
    pub presentation: Presentation,
    pub overlay: PresentationFrame,
    pub patches: BTreeMap<i16, [Option<u8>; 2]>,
    pub random: Random,
}

impl Playback {
    pub fn new(resources: &Resources, seed: u32) -> Self {
        Self {
            frame: -1,
            view: Matrix::default(),
            camera: Motion::new(&resources.script.camera),
            actors: resources.script.actors.iter().map(Motion::new).collect(),
            faces: Faces::new(resources),
            presentation: Presentation::default(),
            overlay: PresentationFrame::default(),
            patches: BTreeMap::new(),
            random: Random::new(seed),
        }
    }

    pub fn finished(&self, script: &Script) -> bool {
        i32::from(self.frame) + 1 >= script.duration
    }

    pub fn step(
        &mut self,
        script: &Script,
        basis: &[[i16; 4]; 33],
        math: &Math,
        draw: bool,
    ) -> Result<Vec<TextureChange>> {
        if self.finished(script) {
            return Ok(Vec::new());
        }
        self.frame += 1;
        self.camera.update(&script.camera, self.frame, basis)?;
        self.view = self
            .camera
            .camera_view(i32::from(self.presentation.camera_roll), math)?;
        self.overlay = self.presentation.update(script, self.frame, draw);
        let changes = self
            .faces
            .update(&script.expressions, self.frame, draw, &mut self.random);
        for &change in &changes {
            if let TextureChange::Patch { model, index } = change {
                self.patches.entry(model).or_default()[usize::from(index / 16)] = Some(index);
            }
        }
        for (motion, animation) in self.actors.iter_mut().zip(&script.actors) {
            motion.update(animation, self.frame, basis)?;
        }
        Ok(changes)
    }
}
