use std::collections::BTreeSet;
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::tenchu2::{
    environment::{
        GAISEN_PITFALL,
        scenery::{Frame, Model, Source},
    },
    world::World,
};

#[derive(Default)]
pub(super) struct Terrain {
    key: Option<TerrainKey>,
    surfaces: Vec<crate::render_effects::Surface>,
}

#[derive(PartialEq)]
struct TerrainKey {
    phases: Vec<i16>,
    atmosphere: bool,
}

impl Terrain {
    pub fn invalidate(&mut self) {
        self.key = None;
    }

    pub fn surfaces(&self) -> &[crate::render_effects::Surface] {
        &self.surfaces
    }

    pub fn prepare(
        &mut self,
        session: &super::super::Session,
        models: &super::Models,
        renderer: &mut crate::renderer::Renderer,
        mesh: &mut crate::renderer::Mesh,
    ) {
        let graphics = renderer.graphics();
        let key = TerrainKey {
            phases: session
                .assets
                .level
                .scene
                .statics
                .iter()
                .zip(&session.world.environment.static_phases)
                .filter_map(|(object, &phase)| {
                    matches!(
                        object.kind,
                        tenchu_assets::tenchu2::level::objects::StaticKind::Model(_)
                    )
                    .then_some(phase)
                })
                .collect(),
            atmosphere: super::atmosphere(graphics),
        };
        if self.key.as_ref() == Some(&key) {
            return;
        }
        // Without source fog, stationary geometry is independent of the camera.
        // Transparent faces still need their depth recomputed and sorted each frame.
        let mut output =
            super::Builder::new(session.world.camera.view, &session.assets.level, graphics);
        models.static_scenery(&mut output, &session.world, &session.assets, true);
        *mesh = renderer.upload(&output.terrain);
        self.surfaces = output.surfaces;
        self.key = Some(key);
    }
}

pub(super) fn distant_doors<'a>(
    world: &'a World,
    level: &'a Level,
    frame: &Frame,
) -> impl Iterator<Item = Model> + 'a {
    let drawn: BTreeSet<_> = frame
        .models
        .iter()
        .filter_map(|model| match model.source {
            Source::Object(index) => Some(index),
            Source::Static(_) => None,
        })
        .collect();
    world
        .environment
        .objects
        .iter()
        .enumerate()
        .filter_map(move |(index, object)| {
            (matches!(
                level.scene.dynamic[index].kind as i16,
                0..=2 | 6 | 7 | GAISEN_PITFALL
            ) && level.grid.contains(object.cell)
                && !drawn.contains(&index))
            .then(|| Model::object(index, object))
            .flatten()
        })
}

#[cfg(test)]
mod tests;
