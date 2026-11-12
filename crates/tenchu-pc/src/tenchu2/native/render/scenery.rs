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
        source: Static<'_>,
        view: tenchu_core::math::Matrix,
        renderer: &mut crate::renderer::Renderer,
        mesh: &mut crate::renderer::Mesh,
    ) {
        let graphics = renderer.graphics();
        let key = TerrainKey {
            phases: source
                .level
                .scene
                .statics
                .iter()
                .zip(source.phases.unwrap_or_default())
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
        // Source fog changes vertex colours with the camera; only fog-free terrain is retained.
        // Transparent faces retain their geometry but are sorted at the current view depth.
        let mut output = super::Builder::new(view, source.level, graphics);
        source.draw(&mut output, true);
        *mesh = renderer.upload(&output.terrain);
        self.surfaces = output.surfaces;
        self.key = Some(key);
    }
}

pub(super) struct Static<'a> {
    pub level: &'a Level,
    pub meshes: &'a crate::visuals::models::Meshes,
    pub math: &'a tenchu_core::math::Math,
    pub phases: Option<&'a [i16]>,
}

impl Static<'_> {
    pub fn draw(&self, output: &mut super::Builder, props: bool) {
        for part in &self.level.map {
            output.model(
                self.meshes,
                part,
                tenchu_core::math::Matrix {
                    translation: part.position.map(|v| i32::from(v) * 10).into(),
                    ..Default::default()
                },
                super::ModelStyle::Terrain,
            );
        }
        if props {
            for (part, matrix) in super::static_models(self.level, self.math, self.phases) {
                output.model(self.meshes, part, matrix, super::ModelStyle::Terrain);
            }
        }
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
