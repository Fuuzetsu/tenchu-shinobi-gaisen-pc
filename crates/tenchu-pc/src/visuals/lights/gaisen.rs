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
        Self::new(
            &game.mission.vram,
            catalog::models(&game.mission),
            &rules(pack, artwork),
        )
        .place(game)
    }

    pub(crate) fn build_with_textures(
        game: &Game,
        pack: &Pack,
        artwork: bool,
        textures: &catalog::Textures,
    ) -> Self {
        Self::with_textures(
            &game.mission.vram,
            catalog::models(&game.mission),
            &rules(pack, artwork),
            textures,
        )
        .place(game)
    }

    fn place(mut self, game: &Game) -> Self {
        render_mesh::visit_stage(&game.mission, &game.math, |geometry, matrix| {
            if let Some(source) = self.source(geometry, || matrix) {
                self.placed.push(source);
            }
        });
        self
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
