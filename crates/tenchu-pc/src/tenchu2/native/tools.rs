use super::campaign::Content;
use crate::{
    controls::Controls,
    debug::{Debug, camera::Camera, inspection::Snapshot},
    settings::menu::Item,
};
use tenchu_assets::Result;
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};

#[derive(Default)]
pub struct Tools {
    pub debug: Debug,
    scene: Snapshot,
    generation: Option<u64>,
    cursor: Option<(f64, f64)>,
    click: Option<(f64, f64)>,
    teleport: Option<(tenchu_core::math::Position, bool)>,
}

impl Tools {
    pub fn sync(&mut self, content: &Content, generation: u64) {
        let changed = self.generation != Some(generation);
        if changed {
            self.debug.return_camera();
            self.debug.bookmark = None;
            self.debug.selected_actor = 0;
            self.generation = Some(generation);
            self.cursor = None;
            self.click = None;
            self.teleport = None;
            self.debug.click_teleport = false;
        }
        self.scene.actors.clear();
        let view = match content {
            Content::Gameplay(session) => {
                self.scene.player = session.world.player;
                if changed {
                    self.debug.target = session.world.actors[session.world.player].actor.position;
                }
                self.scene.actors.extend(super::inspection::actors(session));
                Some(session.world.camera.view)
            }
            Content::Story { story, .. } => {
                self.scene.player = 0;
                Some(story.state.view)
            }
            _ => None,
        };
        self.scene.camera = view.map(|v| Camera::from_view(v, 300.0));
        self.debug.selected_actor = self
            .debug
            .selected_actor
            .min(self.scene.actors.len().saturating_sub(1));
    }

    pub fn advance(&mut self, controls: &Controls, elapsed: f32, active: bool) {
        if !active {
            self.debug.looking = false;
            return;
        }
        if let Some(camera) = &mut self.debug.camera {
            camera.advance(
                controls,
                elapsed,
                f32::from(self.debug.flight_speed) * 1000.0,
            );
        }
    }

    pub fn event(&mut self, event: &WindowEvent, active: bool) {
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } if active => {
                self.click = self.cursor;
            }
            WindowEvent::Touch(touch)
                if active
                    && touch.phase == TouchPhase::Started
                    && (self.debug.teleport_armed || self.debug.click_teleport) =>
            {
                self.click = Some((touch.location.x, touch.location.y));
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.debug.looking = false;
                self.cursor = None;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state,
                ..
            } => {
                self.debug.looking = active && *state == ElementState::Pressed;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if active
                    && self.debug.looking
                    && let Some((x, y)) = self.cursor
                    && let Some(camera) = &mut self.debug.camera
                {
                    camera.look(position.x - x, position.y - y);
                }
                self.cursor = Some((position.x, position.y));
            }
            _ => {}
        }
    }

    pub fn change(&mut self, item: Item, delta: i32) -> Result<()> {
        match item {
            Item::Teleport | Item::PickTeleport | Item::ClickTeleport => {
                if self.scene.actors.is_empty() {
                    return Err("Start a mission before teleporting".into());
                }
                match item {
                    Item::Teleport => {
                        self.teleport = Some((self.debug.target, self.debug.snap_to_floor))
                    }
                    Item::PickTeleport => {
                        self.debug.inspection_camera(&self.scene)?;
                        self.debug.teleport_armed = true;
                    }
                    _ => self.debug.click_teleport = !self.debug.click_teleport,
                }
                return Ok(());
            }
            _ => {}
        }
        if self.debug.change_inspection(&self.scene, item, delta)? {
            Ok(())
        } else {
            Err("This control is not connected to native Tenchu 2 JP yet.".into())
        }
    }

    pub fn reset(&mut self, item: Item) -> Result<()> {
        if item == Item::ClickTeleport {
            self.debug.click_teleport = false;
            return Ok(());
        }
        if self.debug.reset_inspection(&self.scene, item)? {
            Ok(())
        } else {
            Err("This action has no default value.".into())
        }
    }

    pub fn apply(&mut self, content: &mut Content, size: [u32; 2]) -> Result<()> {
        if let Some((x, y)) = self.click.take()
            && let Content::Gameplay(session) = content
            && let Some(camera) = self.debug.camera.or(self.scene.camera)
        {
            let matrix = camera.matrix(size[0].max(1) as f32 / size[1].max(1) as f32);
            let ndc = [
                (x / f64::from(size[0].max(1)) * 2.0 - 1.0) as f32,
                (1.0 - y / f64::from(size[1].max(1)) * 2.0) as f32,
            ];
            let (origin, direction) = crate::debug::camera::ray(matrix, ndc);
            if self.debug.teleport_armed || self.debug.click_teleport {
                let point = super::inspection::pick(&session.world.terrain, origin, direction)
                    .ok_or("No collision floor at that point")?;
                self.debug.target = point;
                self.teleport = Some((point, false));
            } else if self.debug.camera.is_some() {
                use tenchu_core::tenchu2::collision::{BODY, DISABLED, REMOVED};
                if let Some((_, actor)) = session
                    .world
                    .collision
                    .slots()
                    .iter()
                    .filter_map(|slot| {
                        if slot.flags & (BODY | DISABLED | REMOVED) != BODY {
                            return None;
                        }
                        let p = glam::Vec3::from(slot.position.to_array().map(|v| v as f32));
                        let h = glam::Vec3::from(slot.size.to_array().map(f32::from));
                        Some((
                            crate::debug::ray_box(origin, direction, p - h, p + h)?,
                            slot.owner?,
                        ))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                {
                    self.debug.selected_actor = actor;
                    self.debug.inspect_actor = true;
                }
            }
        }
        if let Some((position, snap)) = self.teleport.take() {
            let Content::Gameplay(session) = content else {
                return Err("Start a mission before teleporting".into());
            };
            session
                .world
                .teleport_player(position, snap, &session.assets)?;
            self.debug.teleport_armed = false;
        }
        Ok(())
    }

    pub fn hint(&self, ctx: &egui::Context, menu_open: bool) {
        if (self.debug.teleport_armed || self.debug.click_teleport)
            && !menu_open
            && !self.debug.photo_mode
        {
            crate::debug::overlay::paint_labels(
                ctx,
                &[crate::debug::Label(
                    crate::debug::Anchor::BottomLeft([12.0, 96.0]),
                    "TELEPORT · click/tap a floor · Esc/Back: cancel".into(),
                    [1.0, 0.85, 0.2],
                )],
            );
        }
        if let Some((message, at)) = &self.debug.notice
            && at.elapsed().as_secs() < 5
            && !menu_open
            && !self.debug.photo_mode
        {
            crate::debug::overlay::paint_labels(
                ctx,
                &[crate::debug::Label(
                    crate::debug::Anchor::BottomLeft([12.0, 72.0]),
                    message.clone(),
                    [1.0, 0.85, 0.2],
                )],
            );
        }
        if self.debug.camera.is_some() && !self.debug.photo_mode && !menu_open {
            egui::Area::new("native-camera-help".into())
                .anchor(egui::Align2::LEFT_BOTTOM, [12.0, -12.0])
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(if cfg!(target_os = "android") {
                        "FREE CAMERA · sticks move/look · Back: return"
                    } else {
                        "FREE CAMERA · WASD/QE: fly · Shift: boost · RMB/stick: look · F2: return"
                    });
                    });
                });
        }
    }
}

#[cfg(test)]
mod tests;
