pub mod camera;
mod overlay;
mod sight;
pub use overlay::{Anchor, Label, Overlay, light_markers};
pub mod terrain;

#[cfg(test)]
mod tests;

use camera::Camera;
use glam::{Mat4, Vec3};
use std::collections::VecDeque;
use tenchu_assets::Result;
use tenchu_core::game::Game;

pub struct Debug {
    pub camera: Option<Camera>,
    pub bookmark: Option<Camera>,
    pub looking: bool,
    pub flight_speed: u16,
    pub hitboxes: bool,
    pub sight: bool,
    sight_occluders: std::cell::OnceCell<sight::Occluders>,
    pub sight_stance: u8,
    pub collision: bool,
    pub collision_only: bool,
    pub patrols: bool,
    pub events: bool,
    pub labels: bool,
    pub xray: bool,
    pub show_trail: bool,
    pub inspect_actor: bool,
    pub selected_actor: usize,
    pub target: tenchu_core::math::Position,
    pub snap_to_floor: bool,
    pub teleport_armed: bool,
    pub click_teleport: bool,
    pub god_mode: bool,
    pub stealth: bool,
    pub inventory: [u8; 19],
    pub armor: bool,
    pub speed: u16,
    pub sight_percent: u16,
    pub frame_step: bool,
    pub pause_view: bool,
    pub photo_mode: bool,
    pub light: Option<Vec3>,
    pub light_follow: bool,
    pub surfaces: Vec<terrain::Surface>,
    trail: VecDeque<Vec3>,
    last_tick: u64,
}

impl Debug {
    pub fn new(game: &Game) -> Result<Self> {
        Ok(Self {
            camera: None,
            bookmark: None,
            looking: false,
            flight_speed: 12,
            hitboxes: false,
            sight: false,
            sight_occluders: Default::default(),
            sight_stance: 0,
            collision: false,
            collision_only: false,
            patrols: false,
            events: false,
            labels: false,
            xray: false,
            show_trail: false,
            inspect_actor: false,
            selected_actor: game.player_id,
            target: game.player().position,
            snap_to_floor: true,
            teleport_armed: false,
            click_teleport: false,
            god_mode: game.player().invulnerable,
            stealth: game.intelligence.ignore_player,
            inventory: game.player().inventory[..19].try_into().unwrap(),
            armor: game.combat.armor,
            speed: 100,
            sight_percent: game.intelligence.sight_percent,
            frame_step: false,
            pause_view: false,
            photo_mode: false,
            light: None,
            light_follow: false,
            surfaces: terrain::surfaces(&game.mission.terrain)?,
            trail: VecDeque::new(),
            last_tick: game.tick,
        })
    }

    pub fn reload(&mut self, game: &Game) -> Result<()> {
        self.click_teleport = false;
        self.update(game);
        self.camera = None;
        self.looking = false;
        self.bookmark = None;
        self.light = None;
        self.light_follow = false;
        self.teleport_armed = false;
        self.selected_actor = game.player_id;
        self.target = game.player().position;
        self.surfaces = terrain::surfaces(&game.mission.terrain)?;
        self.sight_occluders.take();
        self.trail.clear();
        self.last_tick = game.tick;
        Ok(())
    }

    pub fn matrix(&self, game: &Game, aspect: f32) -> Mat4 {
        self.camera.map_or_else(
            || crate::renderer::game_camera(game.rendered_view, aspect),
            |c| c.matrix(aspect),
        )
    }

    pub fn eye(&self, game: &Game) -> Vec3 {
        self.camera
            .unwrap_or_else(|| Camera::from_game(game))
            .position
    }

    pub fn light_position(&self, game: &Game) -> Option<Vec3> {
        self.light
            .map(|p| if self.light_follow { self.eye(game) } else { p })
    }

    pub fn toggle_camera(&mut self, game: &Game) {
        self.camera = if self.camera.is_some() {
            None
        } else {
            Some(Camera::from_game(game))
        };
        self.looking = false;
        self.teleport_armed = false;
    }

    pub fn focus(&mut self, game: &Game, overhead: bool) {
        let actor = game
            .roster
            .actors
            .get(self.selected_actor)
            .and_then(Option::as_ref)
            .unwrap_or_else(|| game.player());
        let target = Vec3::from(actor.position.map(|v| v as f32).to_array()) - Vec3::Y * 900.0;
        self.camera
            .get_or_insert_with(|| Camera::from_game(game))
            .focus(target, overhead);
    }

    pub fn select(&mut self, game: &Game, direction: i32) {
        let order = &game.roster.order;
        let index = order
            .iter()
            .position(|&id| id == self.selected_actor)
            .unwrap_or(0);
        self.selected_actor =
            order[(index as i32 + direction).rem_euclid(order.len() as i32) as usize];
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
        self.god_mode = game.player().invulnerable;
        self.stealth = game.intelligence.ignore_player;
        self.inventory
            .copy_from_slice(&game.player().inventory[..19]);
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

    pub fn active(&self) -> bool {
        self.camera.is_some()
            || self.click_teleport
            || self.hitboxes
            || self.sight
            || self.collision
            || self.collision_only
            || self.patrols
            || self.events
            || self.labels
            || self.show_trail
            || self.inspect_actor
            || self.pause_view
            || self.photo_mode
    }
}

fn ray_box(origin: Vec3, direction: Vec3, low: Vec3, high: Vec3) -> Option<f32> {
    let mut near = 0.0f32;
    let mut far = f32::INFINITY;
    for axis in 0..3 {
        if direction[axis].abs() < 0.000001 {
            if origin[axis] < low[axis] || origin[axis] > high[axis] {
                return None;
            }
        } else {
            let a = (low[axis] - origin[axis]) / direction[axis];
            let b = (high[axis] - origin[axis]) / direction[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
        }
    }
    (far >= near).then_some(near)
}
