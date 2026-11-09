pub mod camera;
mod gaisen;
pub mod inspection;
pub(crate) mod overlay;
mod sight;
pub use gaisen::overlay::light_markers;
pub use overlay::{Anchor, Label, Overlay};
pub mod terrain;

#[cfg(test)]
mod tests;

use camera::Camera;
use glam::Vec3;
use std::collections::VecDeque;

pub struct Debug {
    pub state_slot: u8,
    pub rewind: Option<crate::rewind::Timeline>,
    pub rewind_bytes: usize,
    pub notice: Option<(String, web_time::Instant)>,
    pub camera: Option<Camera>,
    pub follow_camera: Option<Camera>,
    pub camera_actor: Option<camera::Subject>,
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
    pub actor_behavior: Option<tenchu_core::ai::profiles::Overrides>,
    pub actor_combat: Option<tenchu_core::ai::profiles::Combat>,
    pub target: tenchu_core::math::Position,
    pub snap_to_floor: bool,
    pub teleport_armed: bool,
    pub pick_actor_position: bool,
    pub actor_imports:
        std::collections::BTreeMap<String, crate::visuals::appearance::library::Imported>,
    pub actors_edited: bool,
    pub click_teleport: bool,
    pub god_mode: bool,
    pub stealth: bool,
    pub inventory: [u8; 19],
    pub bandages: Option<u16>,
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

impl Default for Debug {
    fn default() -> Self {
        Self {
            state_slot: 0,
            rewind: None,
            rewind_bytes: 0,
            notice: None,
            camera: None,
            follow_camera: None,
            camera_actor: None,
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
            selected_actor: 0,
            actor_behavior: None,
            actor_combat: None,
            target: Default::default(),
            snap_to_floor: true,
            teleport_armed: false,
            pick_actor_position: false,
            actor_imports: Default::default(),
            actors_edited: false,
            click_teleport: false,
            god_mode: false,
            stealth: false,
            inventory: [0; 19],
            bandages: None,
            armor: false,
            speed: 100,
            sight_percent: 100,
            frame_step: false,
            pause_view: false,
            photo_mode: false,
            light: None,
            light_follow: false,
            surfaces: Vec::new(),
            trail: VecDeque::new(),
            last_tick: 0,
        }
    }
}

impl Debug {
    pub fn return_camera(&mut self) {
        self.camera = None;
        self.camera_actor = None;
        self.follow_camera = None;
        self.looking = false;
        self.teleport_armed = false;
        self.pick_actor_position = false;
    }

    pub fn active(&self) -> bool {
        self.camera.is_some()
            || self.camera_actor.is_some()
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
