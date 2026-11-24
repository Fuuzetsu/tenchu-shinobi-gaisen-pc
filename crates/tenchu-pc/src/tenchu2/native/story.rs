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
    tenchu2::story::{Playback, TextureChange},
};

#[cfg(not(target_arch = "wasm32"))]
pub(super) mod execution;
#[cfg(not(target_arch = "wasm32"))]
pub(super) mod state;

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
    phase: u8,
}

impl Story {
    pub fn load(disc: &Disc<impl ReadAt>, sequence: usize, event: u16, seed: u32) -> Result<Self> {
        let resources = Resources::load(disc, event)?;
        let level = Level::japanese_scene(disc, resources.area, sequence, true)?;
        Self::new(
            disc,
            sequence,
            event,
            seed,
            resources,
            level,
            Rules::load(disc)?,
        )
    }

    fn new(
        disc: &Disc<impl ReadAt>,
        sequence: usize,
        event: u16,
        seed: u32,
        resources: Resources,
        level: Level,
        rules: Rules,
    ) -> Result<Self> {
        let mut state = Playback::new(&resources, seed);
        state.effects.particles.weather =
            tenchu_core::tenchu2::effects::weather::Weather::for_map(level.id, event, true);
        Ok(Self {
            sequence,
            event,
            resources,
            level,
            state,
            interpolation: None,
            models: Default::default(),
            sprites: Sprites::load(disc)?,
            rules,
            contact: Default::default(),
            math: Math {
                tables: math_tables(disc)?,
            },
            basis: motions::spline_basis(disc)?,
            phase: 0,
        })
    }

    pub fn step(&mut self, draw: bool) -> Result<()> {
        if self.state.frame >= 0 {
            self.phase += self.frames_per_second();
            if self.phase < 30 {
                return Ok(());
            }
            self.phase -= 30;
        }
        let changes = self.state.step(
            &self.resources,
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

    pub fn frames_per_second(&self) -> u8 {
        match self.resources.edition {
            tenchu_assets::tenchu2::Edition::Japan => 30,
            tenchu_assets::tenchu2::Edition::Europe => 25,
        }
    }

    pub fn seconds(&self) -> f64 {
        (f64::from(self.state.frame.max(0)) + f64::from(self.phase) / 30.0)
            / f64::from(self.frames_per_second())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn interpolation_alpha(&self, alpha: f32) -> f32 {
        // A held EU source tick must not replay the previous interpolation pair.
        if self.phase == 25 { 1.0 } else { alpha }
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
        Ok(Some(
            motion
                .pose(&model.parts)?
                .matrices(&self.math, Matrix::default()),
        ))
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
