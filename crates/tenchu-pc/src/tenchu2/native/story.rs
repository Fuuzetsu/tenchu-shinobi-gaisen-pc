use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    tenchu2::{
        effects::{Rules, Sprites},
        level::Level,
        math_tables, motions,
        story::Resources,
    },
};
use tenchu_core::{
    math::{Math, Matrix},
    motion::Pose,
    tenchu2::story::{Playback, TextureChange},
};

#[cfg(test)]
mod state;

pub struct Story {
    pub sequence: usize,
    pub event: u16,
    pub resources: Resources,
    pub level: Level,
    pub math: Math,
    pub state: Playback,
    pub interpolation: Option<crate::visuals::interpolation::History>,
    pub models: super::models::story::Override,
    pub sprites: Sprites,
    pub contact: super::contact::History,
    rules: Rules,
    basis: [[i16; 4]; 33],
}

impl Story {
    pub fn load(disc: &Disc<impl ReadAt>, sequence: usize, event: u16, seed: u32) -> Result<Self> {
        let resources = Resources::load(disc, event)?;
        let level = Level::japanese_scene(disc, resources.area, sequence, true)?;
        let state = Playback::new(&resources, seed);
        Ok(Self {
            sequence,
            event,
            resources,
            level,
            state,
            interpolation: None,
            models: Default::default(),
            sprites: Sprites::load(disc)?,
            rules: Rules::load(disc)?,
            contact: Default::default(),
            math: Math {
                tables: math_tables(disc)?,
            },
            basis: motions::spline_basis(disc)?,
        })
    }

    pub fn step(&mut self, draw: bool) -> Result<()> {
        let changes = self.state.step(
            &self.resources.script,
            &self.basis,
            &self.math,
            &self.rules,
            &self.level,
            draw,
        )?;
        if draw {
            self.contact
                .record(self.state.effects.tick, &self.state.effects.frame);
        } else {
            self.contact.record(
                self.state.effects.tick,
                &tenchu_core::tenchu2::effects::Frame {
                    quads: self.state.effects.frame.quads.clone(),
                    ..Default::default()
                },
            );
        }
        for change in changes {
            if let TextureChange::Patch { model, index } = change {
                self.resources.faces[&model][usize::from(index)]
                    .apply(&mut self.resources.models.get_mut(&model).unwrap().vram);
            }
        }
        if let Some(mut history) = self.interpolation.take() {
            let snapshot = super::interpolation::story(self)?;
            history.record(snapshot);
            self.interpolation = Some(history);
        }
        Ok(())
    }

    pub fn interpolate(&mut self, enabled: bool) -> Result<()> {
        if enabled && self.interpolation.is_none() {
            let mut history = crate::visuals::interpolation::History::default();
            history.reset_to(super::interpolation::story(self)?);
            self.interpolation = Some(history);
        } else if !enabled {
            self.interpolation = None;
        }
        Ok(())
    }

    pub(super) fn matrices(&self, slot: usize) -> Result<Option<Vec<Matrix>>> {
        let animation = &self.resources.script.actors[slot];
        let Some(model) = self.resources.models.get(&animation.model) else {
            return Ok(None);
        };
        let motion = &self.state.actors[slot];
        let mut pose = Pose::new(&model.parts)?;
        pose.translation[0] = motion.position;
        for (rotation, &value) in pose.rotation.iter_mut().zip(&motion.rotation) {
            *rotation = value;
        }
        Ok(Some(pose.matrices(&self.math, Matrix::default())))
    }

    #[cfg(test)]
    pub fn restore(&mut self, state: Playback) {
        for (&model, indices) in &state.patches {
            for &index in indices.iter().flatten() {
                self.resources.faces[&model][usize::from(index)]
                    .apply(&mut self.resources.models.get_mut(&model).unwrap().vram);
            }
        }
        self.state = state;
        self.contact = Default::default();
        self.contact
            .record(self.state.effects.tick, &self.state.effects.frame);
        self.interpolation = None;
    }
}
