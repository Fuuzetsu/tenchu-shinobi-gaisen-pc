use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    tenchu2::{level::Level, math_tables, motions, story::Resources},
};
use tenchu_core::{
    math::Math,
    tenchu2::story::{Playback, TextureChange},
};

pub struct Story {
    pub sequence: usize,
    pub event: u16,
    pub resources: Resources,
    pub level: Level,
    pub math: Math,
    pub state: Playback,
    basis: [[i16; 4]; 33],
}

impl Story {
    pub fn load(disc: &Disc<impl ReadAt>, sequence: usize, event: u16, seed: u32) -> Result<Self> {
        let mut resources = Resources::load(disc, event)?;
        let level = Level::japanese_scene(disc, resources.area, sequence, false)?;
        level.shading.apply(
            resources
                .models
                .values_mut()
                .flat_map(|model| &mut model.parts)
                .chain(
                    resources
                        .weapons
                        .values_mut()
                        .flat_map(|model| &mut model.parts),
                ),
        );
        let state = Playback::new(&resources, seed);
        Ok(Self {
            sequence,
            event,
            resources,
            level,
            state,
            math: Math {
                tables: math_tables(disc)?,
            },
            basis: motions::spline_basis(disc)?,
        })
    }

    pub fn step(&mut self, draw: bool) -> Result<()> {
        let changes = self
            .state
            .step(&self.resources.script, &self.basis, &self.math, draw)?;
        for change in changes {
            if let TextureChange::Patch { model, index } = change {
                self.resources.faces[&model][usize::from(index)]
                    .apply(&mut self.resources.models.get_mut(&model).unwrap().vram);
            }
        }
        Ok(())
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
    }
}
