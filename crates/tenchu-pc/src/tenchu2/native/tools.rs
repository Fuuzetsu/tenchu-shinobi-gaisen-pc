use super::campaign::Content;
use crate::{
    controls::Controls,
    debug::{
        Debug,
        camera::Camera,
        inspection::{Actor, Scene, Snapshot},
    },
    settings::menu::Item,
};
use tenchu_assets::Result;
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};

mod actors;
mod experiment;

#[derive(Default)]
pub struct Tools {
    pub debug: Debug,
    pub actor_models: Vec<(String, String)>,
    scene: Snapshot,
    orbit: crate::navigation::Orbit,
    tick: u64,
    generation: Option<u64>,
    cursor: Option<(f64, f64)>,
    click: Option<(f64, f64)>,
    teleport: Option<(tenchu_core::math::Position, bool)>,
    kill: Option<usize>,
    actor_edit: Option<actors::Edit>,
    pub open_actor_editor: bool,
    edits: Vec<experiment::Edit>,
    focus_light: bool,
    relative: bool,
    player_orbit: bool,
    menu_input: Option<bool>,
}

impl Tools {
    pub fn sync(&mut self, content: &Content, generation: u64) {
        let changed = self.generation != Some(generation);
        if changed {
            self.actor_models = match content {
                Content::Gameplay(session) => session
                    .assets
                    .resources
                    .registry
                    .actors
                    .iter()
                    .filter(|a| a.kind >= 0)
                    .map(|a| {
                        (
                            format!("native:{}", a.kind),
                            format!("{} · T2 JP {:#04x}", a.model, a.kind),
                        )
                    })
                    .collect(),
                _ => Vec::new(),
            };
            self.debug.return_camera();
            self.orbit = Default::default();
            self.debug.bookmark = None;
            self.debug.selected_actor = 0;
            self.generation = Some(generation);
            self.cursor = None;
            self.click = None;
            self.teleport = None;
            self.kill = None;
            self.actor_edit = None;
            self.open_actor_editor = false;
            self.edits.clear();
            self.debug.click_teleport = false;
            self.debug.click_kill = false;
            self.debug.actors_edited = false;
            self.debug.light = None;
            self.debug.light_follow = false;
            self.debug.clear_trail();
            self.focus_light = false;
        }
        self.scene.actors.clear();
        self.player_orbit = false;
        experiment::sync(&mut self.debug, content);
        let view = match content {
            Content::Gameplay(session) if session.cinematic() => {
                self.tick = session.presentation_tick();
                self.debug.follow_camera = None;
                self.debug.clear_trail();
                Some(
                    session
                        .gaisen
                        .as_ref()
                        .unwrap()
                        .director
                        .source
                        .rendered_view,
                )
            }
            Content::Gameplay(session) => {
                self.player_orbit = session.world.player_can_orbit()
                    && session.world.actors[session.world.player].actor.pad.held
                        & tenchu_core::input::L1.swap_bytes()
                        == 0;
                self.tick = u64::from(session.world.tick);
                self.debug.record_trail(
                    self.tick,
                    session.world.actors[session.world.player]
                        .actor
                        .position
                        .to_array()
                        .map(|v| v as f32)
                        .into(),
                );
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

    fn orbit_actor(&self) -> Option<Actor> {
        self.debug.followed_inspection(&self.scene).or_else(|| {
            (self.relative && self.player_orbit && self.debug.camera.is_none())
                .then(|| self.scene.actor(self.scene.player))
                .flatten()
        })
    }

    pub fn input(
        &mut self,
        content: &mut Content,
        controls: &mut Controls,
        settings: &crate::settings::Settings,
        active: bool,
    ) -> u16 {
        let menu = content.menu_controls();
        if self
            .menu_input
            .replace(menu)
            .is_some_and(|previous| previous != menu)
        {
            controls.clear();
        }
        let navigation = &settings.navigation;
        self.relative = navigation.camera_relative;
        let enabled = active && self.debug.camera.is_none();
        if let Content::Gameplay(session) = content {
            session.navigation = None;
            if let Some(state) = &mut session.world.actors[session.world.player].actor.bleeding {
                state.input(enabled && controls.bandage());
            }
            if enabled && navigation.camera_relative && !session.cinematic() {
                let target = self.debug.followed_inspection(&self.scene).or_else(|| {
                    super::inspection::actors(session).find(|a| a.id == session.world.player)
                });
                if let Some(target) = target {
                    self.orbit.observe(u64::from(session.world.tick), target);
                }
                session.navigation = Some(super::navigation::Input {
                    yaw: self.orbit.yaw,
                    pitch: self.orbit.aim_pitch(),
                    sticks: controls.sticks(),
                    deadzone: navigation.deadzone,
                    align_aim: navigation.align_aim,
                });
                return (controls.relative_buttons()
                    & !(crate::controls::WALK | crate::controls::SHEATHE))
                    .swap_bytes();
            }
        }
        if enabled {
            if menu {
                controls
                    .japanese_menu_buttons(settings.circle_confirm)
                    .swap_bytes()
            } else {
                controls.buttons().swap_bytes()
            }
        } else {
            0
        }
    }

    pub fn update_camera(
        &mut self,
        content: &Content,
        drawing: &mut super::presentation::ContentDrawing,
        settings: &crate::settings::Navigation,
        elapsed: f32,
        renderer: &mut crate::renderer::Renderer,
    ) {
        self.debug.follow_camera = None;
        self.relative = settings.camera_relative;
        let Some(actor) = self.orbit_actor() else {
            self.debug.camera_actor = None;
            if !self.relative {
                self.orbit = Default::default();
            }
            return;
        };
        self.orbit.observe(self.tick, actor);
        if let (
            Content::Gameplay(session),
            super::presentation::ContentDrawing::Gameplay(drawing),
        ) = (content, drawing)
        {
            let target = session
                .interpolation
                .as_ref()
                .and_then(|history| history.root(self.tick, actor.root, drawing.alpha))
                .unwrap_or(actor.position)
                - glam::Vec3::Y * 1100.0;
            self.debug.follow_camera = Some(self.orbit.camera_at(
                target,
                elapsed,
                settings,
                drawing.navigation_surfaces(session, renderer),
            ));
        }
    }

    pub fn advance(
        &mut self,
        controls: &mut Controls,
        elapsed: f32,
        active: bool,
        settings: &crate::settings::Navigation,
    ) {
        if !active {
            self.debug.looking = false;
            return;
        }
        self.relative = settings.camera_relative;
        if let Some(actor) = self.orbit_actor() {
            self.orbit.observe(self.tick, actor);
            if controls.take_recenter() {
                self.orbit.recenter(actor);
            }
            self.orbit
                .analog_look(controls.sticks()[1], elapsed, settings);
        }
        if let Some(camera) = &mut self.debug.camera {
            camera.advance(
                controls,
                elapsed,
                f32::from(self.debug.flight_speed) * 1000.0,
            );
        }
    }

    pub fn event(
        &mut self,
        event: &WindowEvent,
        active: bool,
        settings: &crate::settings::Navigation,
    ) {
        if !active {
            self.click = None;
        }
        match event {
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } if active && !self.debug.photo_mode => {
                self.click = self.cursor;
            }
            WindowEvent::Touch(touch)
                if active
                    && !self.debug.photo_mode
                    && touch.phase == TouchPhase::Started
                    && (self.debug.teleport_armed
                        || self.debug.pick_actor_position
                        || self.debug.click_teleport
                        || self.debug.click_kill) =>
            {
                self.click = Some((touch.location.x, touch.location.y));
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.debug.looking = false;
                self.cursor = None;
                self.click = None;
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
                {
                    if let Some(camera) = &mut self.debug.camera {
                        camera.look(position.x - x, position.y - y);
                    } else if let Some(actor) = self.orbit_actor() {
                        self.orbit.observe(self.tick, actor);
                        self.orbit.mouse(position.x - x, position.y - y, settings);
                    }
                }
                self.cursor = Some((position.x, position.y));
            }
            _ => {}
        }
    }

    pub fn change(&mut self, item: Item, delta: i32) -> Result<()> {
        if experiment::change(&mut self.debug, &mut self.edits, item, delta, false)? {
            return Ok(());
        }
        match item {
            Item::KillActor | Item::ClickKill => {
                if self.scene.actors.is_empty() {
                    return Err("Start a mission before killing actors".into());
                }
                if item == Item::KillActor {
                    self.kill = Some(self.debug.selected_actor);
                } else {
                    self.debug.click_kill = !self.debug.click_kill;
                    self.debug.click_teleport = false;
                    self.debug.teleport_armed = false;
                    self.debug.pick_actor_position = false;
                    self.click = None;
                }
                return Ok(());
            }
            Item::FocusLight => {
                self.debug.inspection_eye(&self.scene)?;
                self.focus_light = true;
                return Ok(());
            }
            Item::RecenterOrbit => {
                let actor = self
                    .orbit_actor()
                    .ok_or("Enable camera-relative controls or follow an actor first")?;
                self.orbit.recenter(actor);
                return Ok(());
            }
            Item::Teleport | Item::PickTeleport | Item::ClickTeleport => {
                if self.scene.actors.is_empty() {
                    return Err("Start a mission before teleporting".into());
                }
                self.debug.click_kill = false;
                self.debug.pick_actor_position = false;
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
        if self.debug.change_tool(&self.scene, item, delta)? {
            Ok(())
        } else {
            Err("This control is not connected to native Tenchu 2 JP yet.".into())
        }
    }

    pub fn reset(&mut self, item: Item) -> Result<()> {
        if experiment::change(&mut self.debug, &mut self.edits, item, 0, true)? {
            return Ok(());
        }
        if item == Item::ClickTeleport {
            self.debug.click_teleport = false;
            return Ok(());
        }
        if item == Item::ClickKill {
            self.debug.click_kill = false;
            self.click = None;
            return Ok(());
        }
        if self.debug.reset_tool(&self.scene, item)? {
            Ok(())
        } else {
            Err("This action has no default value.".into())
        }
    }

    pub fn apply(&mut self, content: &mut Content, size: [u32; 2]) -> Result<()> {
        if std::mem::take(&mut self.focus_light) {
            let (level, math) = match content {
                Content::Gameplay(session) => (
                    &session.assets.level,
                    session.assets.resources.math.as_ref(),
                ),
                Content::Story { story, .. } => (&story.level, &story.math),
                _ => {
                    return Err("Start a mission or a 3D cutscene before inspecting lights.".into());
                }
            };
            let source = super::render::lighting::nearest(
                level,
                math,
                self.debug.inspection_eye(&self.scene)?,
            )
            .ok_or("No lamp or flame emitters found in this scene")?;
            self.debug.focus_light(&self.scene, &source)?;
        }
        if !self.edits.is_empty() {
            let edits = self.edits.drain(..);
            let Content::Gameplay(session) = content else {
                return Err("Start a mission before editing the player or equipment.".into());
            };
            if session.cinematic() {
                return Err("Wait for the cinematic before editing the player or equipment".into());
            }
            experiment::apply(&mut session.world, edits)?;
        }
        if let Some((x, y)) = self.click.take()
            && !self.debug.photo_mode
            && let Content::Gameplay(session) = content
            && let Some(camera) = self
                .debug
                .camera
                .or(self.debug.follow_camera)
                .or(self.scene.camera)
        {
            let matrix = camera.matrix(size[0].max(1) as f32 / size[1].max(1) as f32);
            let ndc = [
                (x / f64::from(size[0].max(1)) * 2.0 - 1.0) as f32,
                (1.0 - y / f64::from(size[1].max(1)) * 2.0) as f32,
            ];
            let (origin, direction) = crate::debug::camera::ray(matrix, ndc);
            if self.debug.teleport_armed
                || self.debug.click_teleport
                || self.debug.pick_actor_position
            {
                let point = super::inspection::pick(&session.world.terrain, origin, direction)
                    .ok_or("No collision floor at that point")?;
                self.debug.target = point;
                if self.debug.pick_actor_position {
                    self.debug.pick_actor_position = false;
                    self.debug.looking = false;
                    self.open_actor_editor = true;
                } else {
                    self.teleport = Some((point, false));
                }
            } else if self.debug.camera.is_some() || self.debug.click_kill {
                use tenchu_core::tenchu2::collision::{BODY, DISABLED, REMOVED};
                let picked = session
                    .world
                    .collision
                    .slots()
                    .iter()
                    .filter_map(|slot| {
                        if slot.flags & (BODY | DISABLED | REMOVED) != BODY {
                            return None;
                        }
                        if self.debug.click_kill && slot.owner == Some(session.world.player) {
                            return None;
                        }
                        let p = glam::Vec3::from(slot.position.to_array().map(|v| v as f32));
                        let h = glam::Vec3::from(slot.size.to_array().map(f32::from));
                        Some((
                            crate::debug::ray_box(origin, direction, p - h, p + h)?,
                            slot.owner?,
                        ))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                if self.debug.click_kill {
                    self.kill = Some(picked.ok_or("No NPC at that point")?.1);
                }
                if let Some((_, actor)) = picked {
                    self.debug.selected_actor = actor;
                    self.debug.inspect_actor = true;
                }
            }
        }
        self.apply_actor_edit(content)?;
        if let Some(actor) = self.kill.take() {
            let Content::Gameplay(session) = content else {
                return Err("Start a mission before killing actors".into());
            };
            if session.cinematic() {
                return Err("Wait for the cinematic before killing actors".into());
            }
            session.world.kill_actor(actor, &session.assets)?;
            session.reset_interpolation()?;
            self.debug.actors_edited = true;
            self.debug.notice = Some((
                format!("Killed actor #{actor}. Mission objectives may advance."),
                web_time::Instant::now(),
            ));
        }
        if let Some((position, snap)) = self.teleport.take() {
            let Content::Gameplay(session) = content else {
                return Err("Start a mission before teleporting".into());
            };
            if session.cinematic() {
                return Err("Wait for the cinematic before teleporting".into());
            }
            session
                .world
                .teleport_player(position, snap, &session.assets)?;
            session.reset_interpolation()?;
            self.debug.teleport_armed = false;
        }
        Ok(())
    }

    pub fn hint(&self, ctx: &egui::Context, menu_open: bool) {
        if self.debug.click_kill && !menu_open && !self.debug.photo_mode {
            crate::debug::overlay::paint_labels(
                ctx,
                &[crate::debug::Label(
                    crate::debug::Anchor::BottomLeft([12.0, 96.0]),
                    "KILL · click/tap an NPC · Esc/Back: cancel".into(),
                    [1.0, 0.85, 0.2],
                )],
            );
        }
        if (self.debug.teleport_armed
            || self.debug.click_teleport
            || self.debug.pick_actor_position)
            && !menu_open
            && !self.debug.photo_mode
        {
            crate::debug::overlay::paint_labels(
                ctx,
                &[crate::debug::Label(
                    crate::debug::Anchor::BottomLeft([12.0, 96.0]),
                    if self.debug.pick_actor_position {
                        "ACTOR POSITION · click/tap a floor · Esc/Back: cancel"
                    } else {
                        "TELEPORT · click/tap a floor · Esc/Back: cancel"
                    }
                    .into(),
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
        if (self.debug.camera.is_some() || self.debug.camera_actor.is_some())
            && !self.debug.photo_mode
            && !menu_open
        {
            egui::Area::new("native-camera-help".into())
                .anchor(egui::Align2::LEFT_BOTTOM, [12.0, -12.0])
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(if self.debug.camera_actor.is_some() {
                            "FOLLOW ACTOR · RMB/right stick: orbit · F2/Back: return"
                        } else if cfg!(target_os = "android") {
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
