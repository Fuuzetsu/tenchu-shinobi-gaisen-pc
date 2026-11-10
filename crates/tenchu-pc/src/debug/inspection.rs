use super::{Debug, camera::Camera};
use crate::settings::menu::Item;
use glam::Vec3;
use tenchu_assets::Result;

#[derive(Clone, Copy)]
pub struct Actor {
    pub id: usize,
    pub root: usize,
    pub yaw: i16,
    pub position: Vec3,
    pub kind: i16,
    pub health: i16,
    pub height: f32,
    pub suspended: bool,
}

/// Read-only facts needed by inspection tools, independent of either simulation.
pub trait Scene {
    fn camera(&self) -> Option<Camera>;
    fn player(&self) -> usize;
    fn actors(&self) -> impl Iterator<Item = Actor>;

    fn actor(&self, id: usize) -> Option<Actor> {
        self.actors().find(|actor| actor.id == id)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub struct Snapshot {
    pub camera: Option<Camera>,
    pub player: usize,
    pub actors: Vec<Actor>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Scene for Snapshot {
    fn camera(&self) -> Option<Camera> {
        self.camera
    }
    fn player(&self) -> usize {
        self.player
    }
    fn actors(&self) -> impl Iterator<Item = Actor> {
        self.actors.iter().copied()
    }
}

impl From<Actor> for crate::navigation::Target {
    fn from(actor: Actor) -> Self {
        Self {
            id: actor.id,
            root: actor.root,
            position: actor.position,
            yaw: actor.yaw,
        }
    }
}

impl Debug {
    pub fn inspection_eye(&self, scene: &impl Scene) -> Result<Vec3> {
        self.camera
            .or(self.follow_camera)
            .or_else(|| scene.camera())
            .map(|camera| camera.position)
            .ok_or_else(|| "Start a mission or a 3D cutscene before placing a light.".into())
    }

    pub fn light_position(&self, scene: &impl Scene) -> Option<Vec3> {
        if self.light_follow {
            self.light_at(self.inspection_eye(scene).ok()?)
        } else {
            self.light
        }
    }

    pub fn light_at(&self, eye: Vec3) -> Option<Vec3> {
        self.light
            .map(|position| if self.light_follow { eye } else { position })
    }

    pub fn focus_light(
        &mut self,
        scene: &impl Scene,
        source: &crate::lighting::Source,
    ) -> Result<()> {
        let mut camera = self
            .camera
            .or(self.follow_camera)
            .or_else(|| scene.camera())
            .ok_or("Start a mission or a 3D cutscene before inspecting lights.")?;
        let direction = (source.light.position - camera.position).normalize_or_zero();
        camera.yaw = direction.x.atan2(direction.z);
        camera.focus(source.light.position, false);
        camera.position = source.light.position - camera.forward() * 4500.0;
        self.return_camera();
        self.camera = Some(camera);
        Ok(())
    }

    pub fn followed_inspection(&self, scene: &impl Scene) -> Option<Actor> {
        let subject = self.camera_actor?;
        scene
            .actor(subject.id)
            .filter(|actor| actor.root == subject.root_model)
    }

    pub fn follow_selected(&mut self, scene: &impl Scene) -> Result<()> {
        let actor = scene
            .actor(self.selected_actor)
            .ok_or("The selected actor is no longer in this mission")?;
        self.return_camera();
        self.camera_actor = Some(super::camera::Subject {
            id: actor.id,
            root_model: actor.root,
        });
        Ok(())
    }

    pub fn inspection_camera(&mut self, scene: &impl Scene) -> Result<&mut Camera> {
        let source = self
            .camera
            .or(self.follow_camera)
            .or_else(|| scene.camera())
            .ok_or("Start a mission or a 3D cutscene before using the camera.")?;
        self.camera_actor = None;
        Ok(self.camera.get_or_insert(source))
    }

    pub fn toggle_inspection_camera(&mut self, scene: &impl Scene) -> Result<()> {
        let returning = self.camera.is_some() || self.camera_actor.is_some();
        let current = self.follow_camera.or_else(|| scene.camera());
        self.return_camera();
        if !returning {
            self.camera =
                Some(current.ok_or("Start a mission or a 3D cutscene before using the camera.")?);
        }
        Ok(())
    }

    pub fn focus_inspection(&mut self, scene: &impl Scene, overhead: bool) -> Result<()> {
        let actor = scene
            .actor(self.selected_actor)
            .or_else(|| scene.actor(scene.player()))
            .ok_or("No gameplay actor selected.")?;
        self.inspection_camera(scene)?
            .focus(actor.position - Vec3::Y * 900.0, overhead);
        Ok(())
    }

    pub fn select_inspection(&mut self, scene: &impl Scene, direction: i32) -> Result<()> {
        let count = scene.actors().count();
        if count == 0 {
            return Err("No gameplay actors in this scene.".into());
        }
        let index = scene
            .actors()
            .position(|a| a.id == self.selected_actor)
            .unwrap_or(0);
        let next = (index as i32 + direction).rem_euclid(count as i32) as usize;
        self.selected_actor = scene.actors().nth(next).unwrap().id;
        Ok(())
    }

    pub fn change_tool(&mut self, scene: &impl Scene, item: Item, delta: i32) -> Result<bool> {
        match item {
            Item::DebugLight => {
                self.light = if self.light.is_some() {
                    None
                } else {
                    Some(self.inspection_eye(scene)?)
                };
                self.light_follow = false;
            }
            Item::PlaceLight => {
                self.light = Some(self.inspection_eye(scene)?);
                self.light_follow = false;
            }
            Item::PlayerLight => {
                self.light = Some(
                    scene
                        .actor(scene.player())
                        .ok_or("No player in this scene")?
                        .position
                        + Vec3::new(2000.0, -4000.0, 2000.0),
                );
                self.light_follow = false;
            }
            Item::FollowLight => {
                let position = self
                    .light_position(scene)
                    .unwrap_or(self.inspection_eye(scene)?);
                self.light = Some(position);
                self.light_follow = !self.light_follow;
            }
            Item::LightAxis(axis) => {
                let eye = self.inspection_eye(scene)?;
                self.light_follow = false;
                let light = self.light.get_or_insert(eye);
                light[axis] = (light[axis] + delta as f32 * 500.0).clamp(-320_000.0, 320_000.0);
            }
            Item::Speed => {
                self.speed =
                    crate::settings::edit::cycle(self.speed, &[25, 50, 100, 200, 400, 0], delta);
            }
            Item::Target(axis) => {
                self.target[axis] = self.target[axis]
                    .saturating_add(delta * 100)
                    .clamp(-320_000, 320_000);
            }
            Item::PlayerPosition => {
                self.target = scene
                    .actor(scene.player())
                    .ok_or("No player in this scene")?
                    .position
                    .to_array()
                    .map(|v| v.round() as i32)
                    .into();
            }
            Item::CameraPosition => {
                self.target = self
                    .camera
                    .ok_or("Enable free camera first")?
                    .position
                    .to_array()
                    .map(|v| v.round() as i32)
                    .into();
            }
            Item::SnapFloor => self.snap_to_floor = !self.snap_to_floor,
            Item::FreeCamera => self.toggle_inspection_camera(scene)?,
            Item::FlightSpeed => {
                self.flight_speed = (i32::from(self.flight_speed) + delta * 2).clamp(2, 100) as u16;
            }
            Item::Fov => {
                let camera = self.inspection_camera(scene)?;
                camera.fov = (camera.fov + delta as f32 * 5.0).clamp(20.0, 110.0);
            }
            Item::Actor => {
                self.select_inspection(scene, delta)?;
                if self.camera_actor.is_some() {
                    self.follow_selected(scene)?;
                }
            }
            Item::FollowActor => {
                if self.camera_actor.is_some() {
                    self.return_camera();
                } else {
                    self.follow_selected(scene)?;
                }
            }
            Item::FocusPlayer | Item::FocusActor | Item::Overhead => {
                if item == Item::FocusPlayer {
                    self.selected_actor = scene.player();
                }
                self.focus_inspection(scene, item == Item::Overhead)?;
            }
            Item::SaveCamera => {
                self.bookmark = Some(
                    self.camera
                        .or(self.follow_camera)
                        .or_else(|| scene.camera())
                        .ok_or("Start a mission or a 3D cutscene before using the camera.")?,
                );
            }
            Item::RestoreCamera => {
                let camera = self.bookmark.ok_or("No camera bookmark saved")?;
                self.return_camera();
                self.camera = Some(camera);
            }
            Item::PhotoMode => self.photo_mode = !self.photo_mode,
            Item::Hitboxes => self.hitboxes = !self.hitboxes,
            Item::Sight => self.sight = !self.sight,
            Item::SightStance => {
                self.sight_stance = (i32::from(self.sight_stance) + delta).rem_euclid(3) as u8
            }
            Item::Collision => self.collision = !self.collision,
            Item::CollisionOnly => self.collision_only = !self.collision_only,
            Item::Patrols => self.patrols = !self.patrols,
            Item::Events => self.events = !self.events,
            Item::Labels => self.labels = !self.labels,
            Item::Xray => self.xray = !self.xray,
            Item::Trail => self.show_trail = !self.show_trail,
            Item::InspectActor => self.inspect_actor = !self.inspect_actor,
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub fn reset_tool(&mut self, scene: &impl Scene, item: Item) -> Result<bool> {
        match item {
            Item::DebugLight => {
                self.light = None;
                self.light_follow = false;
            }
            Item::FollowLight => {
                self.light = self.light_position(scene);
                self.light_follow = false;
            }
            Item::LightAxis(axis) => {
                if let Some(light) = &mut self.light {
                    light[axis] = 0.0;
                }
            }
            Item::Speed => self.speed = Self::default().speed,
            Item::Target(axis) => {
                self.target[axis] = scene
                    .actor(scene.player())
                    .map_or(0, |actor| actor.position[axis].round() as i32);
            }
            Item::SnapFloor => self.snap_to_floor = true,
            Item::FreeCamera | Item::FollowActor => self.return_camera(),
            Item::FlightSpeed => self.flight_speed = Self::default().flight_speed,
            Item::Fov => {
                if let Some(camera) = &mut self.camera {
                    camera.fov = scene.camera().ok_or("No camera in this scene")?.fov;
                }
            }
            Item::Actor => {
                self.selected_actor = scene.player();
                if self.camera_actor.is_some() {
                    self.follow_selected(scene)?;
                }
            }
            Item::PhotoMode => self.photo_mode = false,
            Item::Hitboxes => self.hitboxes = false,
            Item::Sight => self.sight = false,
            Item::SightStance => self.sight_stance = 0,
            Item::Collision => self.collision = false,
            Item::CollisionOnly => self.collision_only = false,
            Item::Patrols => self.patrols = false,
            Item::Events => self.events = false,
            Item::Labels => self.labels = false,
            Item::Xray => self.xray = false,
            Item::Trail => self.show_trail = false,
            Item::InspectActor => self.inspect_actor = false,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
