use super::*;
use crate::render_mesh;
use tenchu_core::game::Game;

impl Snapshot {
    fn new(game: &Game) -> Self {
        let mut parts = BTreeMap::new();
        for inspection in [false, true] {
            render_mesh::visit_character_parts(game, inspection, |key, _, matrix| {
                parts.insert(key, matrix);
            });
        }
        render_mesh::visit_hinged_parts(game, |key, _, matrix| {
            parts.insert(key, matrix);
        });
        Self {
            tick: game.tick,
            parts,
            roots: game
                .roster
                .order
                .iter()
                .map(|&id| {
                    let a = game.roster.get(id);
                    (a.root_model, a.position)
                })
                .collect(),
            view: game.rendered_view,
            cinematic: (game.cinematic.active, game.cinematic.sequence),
            camera_mode: game.camera.mode.source_id(),
            death_blood: None,
        }
    }
}

impl History {
    pub fn reset(&mut self, game: &Game) {
        self.reset_to(Snapshot::new(game));
    }

    pub fn tick(&mut self, game: &Game) {
        self.record(Snapshot::new(game));
    }
}
