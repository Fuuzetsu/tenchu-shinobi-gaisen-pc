pub(super) mod overlay;
use super::{Debug, Overlay, ray_box, terrain};
use super::{
    camera::{self, Camera},
    inspection::{Actor, Scene},
};
use glam::Mat4;
use glam::Vec3;
use tenchu_assets::Result;
use tenchu_core::game::Game;

impl Scene for Game {
    fn camera(&self) -> Option<Camera> {
        Some(Camera::from_game(self))
    }

    fn player(&self) -> usize {
        self.player_id
    }

    fn actors(&self) -> impl Iterator<Item = Actor> {
        self.roster
            .order
            .iter()
            .map(|&id| actor(self, self.roster.get(id)))
    }
}

impl Debug {
    pub fn new(game: &Game) -> Result<Self> {
        let mut debug = Self {
            selected_actor: game.player_id,
            target: game.player().position,
            god_mode: game.player().invulnerable,
            stealth: game.intelligence.ignore_player,
            bandages: game.player().bleeding.as_ref().map(|s| s.bandages),
            armor: game.combat.armor,
            sight_percent: game.intelligence.sight_percent,
            surfaces: terrain::surfaces(&game.mission.terrain)?,
            last_tick: game.tick,
            ..Self::default()
        };
        debug.inventory(game);
        Ok(debug)
    }

    pub fn reload(&mut self, game: &Game) -> Result<()> {
        self.click_teleport = false;
        self.update(game);
        self.return_camera();
        self.actor_imports.clear();
        self.actors_edited = false;
        self.bookmark = None;
        self.light = None;
        self.light_follow = false;
        self.selected_actor = game.player_id;
        self.target = game.player().position;
        self.surfaces = terrain::surfaces(&game.mission.terrain)?;
        self.sight_occluders.take();
        self.trail.clear();
        self.last_tick = game.tick;
        Ok(())
    }

    pub fn matrix(&self, game: &Game, aspect: f32) -> Mat4 {
        self.camera.or(self.follow_camera).map_or_else(
            || crate::renderer::game_camera(game.rendered_view, aspect),
            |c| c.matrix(aspect),
        )
    }

    pub fn eye(&self, game: &Game) -> Vec3 {
        self.camera
            .or(self.follow_camera)
            .unwrap_or_else(|| Camera::from_game(game))
            .position
    }

    pub fn light_position(&self, game: &Game) -> Option<Vec3> {
        self.light
            .map(|p| if self.light_follow { self.eye(game) } else { p })
    }

    pub fn toggle_camera(&mut self, game: &Game) {
        self.toggle_inspection_camera(game).expect("Gaisen camera");
    }

    pub fn free_camera(&mut self, game: &Game) -> &mut Camera {
        self.inspection_camera(game).expect("Gaisen camera")
    }

    pub fn followed_actor<'a>(&self, game: &'a Game) -> Option<&'a tenchu_core::actor::Actor> {
        let subject = self.camera_actor?;
        game.roster
            .actors
            .get(subject.id)?
            .as_ref()
            .filter(|actor| actor.root_model == subject.root_model)
    }

    pub fn follow_selected(&mut self, game: &Game) -> Result<()> {
        let actor = game
            .roster
            .actors
            .get(self.selected_actor)
            .and_then(Option::as_ref)
            .ok_or("The selected actor is no longer in this mission")?;
        self.return_camera();
        self.camera_actor = Some(camera::Subject {
            id: actor.id,
            root_model: actor.root_model,
        });
        Ok(())
    }

    pub fn focus(&mut self, game: &Game, overhead: bool) {
        self.focus_inspection(game, overhead)
            .expect("Gaisen player");
    }

    pub fn pick_actor(&mut self, game: &Game, origin: Vec3, direction: Vec3) {
        let mut nearest = f32::INFINITY;
        for slot in &game.collision.slots[..game.collision.len] {
            let Some(actor) = slot
                .actor()
                .and_then(|id| game.roster.actors.get(id))
                .and_then(Option::as_ref)
            else {
                continue;
            };
            if slot.class & tenchu_core::collision::HIT != 0
                || actor.attribute & tenchu_core::ids::ATTR_SUSPEND != 0
            {
                continue;
            }
            if !slot.model.is_some_and(|id| {
                game.scene.models[id].attribute & tenchu_core::scene::COLLIDE != 0
            }) {
                continue;
            }
            let center = Vec3::from(slot.position.map(|v| v as f32).to_array());
            let half = Vec3::from(slot.size.map(f32::from));
            if let Some(near) = ray_box(origin, direction, center - half, center + half)
                && near < nearest
            {
                nearest = near;
                self.selected_actor = actor.id;
                self.inspect_actor = true;
            }
        }
    }

    pub fn update(&mut self, game: &Game) {
        if self.camera_actor.is_some() && self.followed_actor(game).is_none() {
            self.return_camera();
        }
        let actor = game
            .roster
            .actors
            .get(self.selected_actor)
            .and_then(Option::as_ref)
            .filter(|a| tenchu_core::ai::profiles::eligible(a, &game.mission, game.player_id));
        self.actor_behavior = actor.map(|a| a.behavior);
        self.actor_combat = actor
            .filter(|a| {
                game.intelligence.profiles.applies(
                    a,
                    &game.mission,
                    game.player_id,
                    game.intelligence.difficulty,
                )
            })
            .map(|a| game.intelligence.profiles.combat(a));
        self.god_mode = game.player().invulnerable;
        self.stealth = game.intelligence.ignore_player;
        self.inventory(game);
        self.bandages = game.player().bleeding.as_ref().map(|s| s.bandages);
        self.armor = game.combat.armor;
        if self.last_tick != game.tick {
            if game.tick < self.last_tick {
                self.trail.clear();
            }
            self.last_tick = game.tick;
            self.trail.push_back(Vec3::from(
                game.player().position.map(|v| v as f32).to_array(),
            ));
            if self.trail.len() > 1800 {
                self.trail.pop_front();
            }
        }
    }

    pub fn overlay(&self, game: &Game, aspect: f32) -> Overlay {
        overlay::build(self, game, aspect)
    }

    fn inventory(&mut self, game: &Game) {
        self.inventory.clear();
        self.inventory
            .extend(
                game.player().inventory[..19]
                    .iter()
                    .enumerate()
                    .map(|(id, &count)| super::InventorySlot {
                        id,
                        name: crate::ui::ITEMS[id],
                        count,
                    }),
            );
        self.inventory_unlimited = true;
    }
}

impl Camera {
    pub fn from_game(game: &tenchu_core::game::Game) -> Self {
        Self::from_view(
            game.rendered_view,
            f32::from(tenchu_core::projection::DISTANCE),
        )
    }
}

fn actor(game: &Game, actor: &tenchu_core::actor::Actor) -> Actor {
    Actor {
        id: actor.id,
        position: Vec3::from(actor.position.to_array().map(|v| v as f32)),
        kind: actor.kind,
        health: actor.health,
        height: f32::from(game.mission.characters[actor.definition].height),
        suspended: actor.attribute & tenchu_core::ids::ATTR_SUSPEND != 0,
    }
}
