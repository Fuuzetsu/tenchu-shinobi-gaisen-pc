use super::{Faces, Motion, Presentation, PresentationFrame, TextureChange};
use crate::{
    Result,
    math::{Math, Matrix},
    random::Random,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::tenchu2::{
    campaign::{Kind, Objective, Sequence},
    story::{Resources, Script},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Ending {
    pub age: i16,
    pub skip: bool,
    pub gameplay: bool,
    pub complete: bool,
}

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
    pub ending: Option<Ending>,
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
            ending: None,
        }
    }

    pub fn finished(&self, script: &Script) -> bool {
        i32::from(self.frame) >= script.duration
    }

    pub fn begin_exit(&mut self, next: Option<Sequence>, skip: bool, results_fade: i16) {
        let gameplay = next.is_some_and(|s| matches!(s.kind, Kind::Gameplay(_)));
        let immediate = next.is_some_and(|s| {
            s.resource != 1
                && matches!(
                    s.kind,
                    Kind::Gameplay(Objective::Weaken { .. } | Objective::Defeat { .. })
                )
        });
        let mut age = if immediate { 16 } else { 0 };
        if next.is_none_or(|s| matches!(s.kind, Kind::Introduction | Kind::Movie | Kind::Results)) {
            if self.presentation.fade.is_none() {
                age = 16;
            } else {
                self.presentation.fade = Some(super::presentation::Fade {
                    white: false,
                    outward: true,
                    duration: if next.is_some_and(|s| s.kind == Kind::Results) {
                        results_fade
                    } else {
                        16
                    },
                    age: 0,
                });
            }
        }
        self.presentation.caption = None;
        self.overlay.caption = None;
        self.ending = Some(Ending {
            age,
            skip,
            gameplay,
            complete: false,
        });
        self.update_exit();
    }

    fn update_exit(&mut self) {
        if let Some(ending) = &mut self.ending {
            self.overlay.caption = None;
            self.overlay.letterbox_offset = if ending.gameplay { ending.age * 2 } else { 0 };
            ending.complete = ending.age > 15;
            if !ending.complete {
                ending.age += 1;
            }
        }
    }

    pub fn step(
        &mut self,
        script: &Script,
        basis: &[[i16; 4]; 33],
        math: &Math,
        draw: bool,
    ) -> Result<Vec<TextureChange>> {
        if self.ending.is_some_and(|e| e.complete)
            || (self.ending.is_none() && self.finished(script))
        {
            return Ok(Vec::new());
        }
        if self.ending.is_none() {
            self.frame += 1;
        }
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
        self.update_exit();
        Ok(changes)
    }
}
