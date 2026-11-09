use super::{Emitters, catalog, rules};
use crate::{
    lighting::Source,
    render_mesh,
    visuals::{interpolation::Frame, pack::Pack},
};
use tenchu_assets::model::Geometry;
use tenchu_core::game::Game;

impl Emitters {
    pub fn build(game: &Game, pack: &Pack, artwork: bool) -> Self {
        let mut out = Self::new(
            &game.mission.vram,
            catalog::models(&game.mission),
            &rules(pack, artwork),
        );
        render_mesh::visit_stage(&game.mission, &game.math, |geometry, matrix| {
            if let Some(source) = out.source(geometry, || matrix) {
                out.placed.push(source);
            }
        });
        out
    }

    pub fn sources(&self, game: &Game) -> Vec<Source> {
        self.sources_presented(game, None)
    }

    pub fn sources_presented(&self, game: &Game, frame: Option<&Frame>) -> Vec<Source> {
        let mut out = self.placed.clone();
        let mut append = |key, geometry: &[Geometry], matrix| {
            if let Some(source) = self.source(geometry, || Frame::transform(frame, key, matrix)) {
                out.push(source);
            }
        };
        render_mesh::visit_character_parts(game, true, &mut append);
        render_mesh::visit_hinged_parts(game, append);
        out
    }
}
