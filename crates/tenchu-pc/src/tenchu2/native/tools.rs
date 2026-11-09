use super::campaign::Content;
use crate::{
    controls::Controls,
    debug::{Debug, camera::Camera, inspection::Snapshot},
    settings::menu::Item,
};
use tenchu_assets::Result;
use winit::event::{ElementState, MouseButton, WindowEvent};

#[derive(Default)]
pub struct Tools {
    pub debug: Debug,
    scene: Snapshot,
    generation: Option<u64>,
    cursor: Option<(f64, f64)>,
}

impl Tools {
    pub fn sync(&mut self, content: &Content, generation: u64) {
        if self.generation != Some(generation) {
            self.debug.return_camera();
            self.debug.bookmark = None;
            self.debug.selected_actor = 0;
            self.generation = Some(generation);
            self.cursor = None;
        }
        self.scene.actors.clear();
        let view = match content {
            Content::Gameplay(session) => {
                self.scene.player = session.world.player;
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
        if self.debug.change_inspection(&self.scene, item, delta)? {
            Ok(())
        } else {
            Err("This control is not connected to native Tenchu 2 JP yet.".into())
        }
    }

    pub fn reset(&mut self, item: Item) -> Result<()> {
        if self.debug.reset_inspection(&self.scene, item)? {
            Ok(())
        } else {
            Err("This action has no default value.".into())
        }
    }

    pub fn hint(&self, ctx: &egui::Context, menu_open: bool) {
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
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    #[test]
    #[ignore = "requires Japanese TENCHU2_DISC; camera input must not change native simulation"]
    fn inspection_is_independent_of_native_gameplay_and_scene_transitions() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let session = crate::tenchu2::native::Session::load(&disc, 9, 1).unwrap();
        let mut content = Content::Gameplay(Box::new(session));
        let mut tools = Tools::default();
        tools.sync(&content, 0);
        let Content::Gameplay(session) = &content else {
            unreachable!()
        };
        let before = serde_json::to_vec(&session.world).unwrap();
        tools.change(Item::FreeCamera, 1).unwrap();
        let start = tools.debug.camera.unwrap().position;
        let mut controls = Controls::default();
        controls.key(KeyCode::KeyW, true);
        for _ in 0..60 {
            tools.advance(&controls, 1.0 / 60.0, true);
        }
        assert!(tools.debug.camera.unwrap().position.distance(start) > 11000.0);
        tools.advance(&controls, 1.0, false);
        tools.change(Item::Actor, 1).unwrap();
        tools.change(Item::FocusActor, 1).unwrap();
        tools.change(Item::SaveCamera, 1).unwrap();
        tools.change(Item::Fov, 1).unwrap();
        tools.change(Item::RestoreCamera, 1).unwrap();
        assert_eq!(
            tools.debug.camera.unwrap().fov,
            tools.debug.bookmark.unwrap().fov
        );
        for item in [Item::Hitboxes, Item::Labels, Item::InspectActor] {
            tools.change(item, 1).unwrap();
        }
        let matrix = tools.debug.camera.unwrap().matrix(16.0 / 9.0);
        let overlay = super::super::inspection::overlay(session, &tools.debug, matrix, 16.0 / 9.0);
        assert!(!overlay.geometry.vertices.is_empty());
        assert!(overlay.labels.iter().any(|label| label.1.contains("HP")));
        assert!(
            overlay
                .geometry
                .vertices
                .iter()
                .all(|vertex| vertex.position.iter().all(|v| v.is_finite()))
        );
        tools.change(Item::PhotoMode, 1).unwrap();
        let hidden = super::super::inspection::overlay(session, &tools.debug, matrix, 16.0 / 9.0);
        assert!(hidden.geometry.vertices.is_empty() && hidden.labels.is_empty());
        tools.reset(Item::PhotoMode).unwrap();
        assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
        tools.change(Item::FreeCamera, 1).unwrap();
        assert!(tools.debug.camera.is_none());
        let Content::Gameplay(session) = &mut content else {
            unreachable!()
        };
        session.step(0).unwrap();
        tools.sync(&content, 0);
        tools.change(Item::FreeCamera, 1).unwrap();
        tools.sync(&Content::Failed, 1);
        assert!(tools.debug.camera.is_none() && tools.debug.bookmark.is_none());
        assert!(tools.change(Item::FreeCamera, 1).is_err());
    }
}
