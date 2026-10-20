use super::*;

fn cycle(value: u16, choices: &[u16], direction: i32) -> u16 {
    let index = choices.iter().position(|&v| v == value).unwrap_or(0);
    choices[(index as i32 + direction).rem_euclid(choices.len() as i32) as usize]
}

impl Application {
    pub(super) fn pick_debug_target(&mut self) {
        let Some(view) = &self.view else {
            return;
        };
        let aspect = view.size[0] as f32 / view.size[1].max(1) as f32;
        let ndc = [
            (self.pointer[0] / f64::from(view.size[0].max(1)) * 2.0 - 1.0) as f32,
            (1.0 - self.pointer[1] / f64::from(view.size[1].max(1)) * 2.0) as f32,
        ];
        let (origin, direction) =
            crate::debug::camera::ray(self.debug.matrix(&self.campaign.game, aspect), ndc);
        if self.debug.teleport_armed {
            if let Some(point) =
                crate::debug::terrain::pick(&self.debug.surfaces, origin, direction)
            {
                self.debug.target = point;
                let snap = self.debug.snap_to_floor;
                self.debug.snap_to_floor = false;
                if let Err(error) = self.teleport() {
                    self.menu.notice = error;
                }
                self.debug.snap_to_floor = snap;
            } else {
                self.menu.notice = "No collision floor at that point".into();
            }
        } else {
            self.debug
                .pick_actor(&self.campaign.game, origin, direction);
        }
        self.update_view();
    }

    pub(super) fn release_pointer(&mut self) {
        self.debug.looking = false;
        if let Some(view) = &self.view {
            let _ = view.window.set_cursor_grab(CursorGrabMode::None);
            view.window.set_cursor_visible(true);
        }
    }

    fn stop_edited_recording(&mut self) {
        if self.recording.is_some() {
            self.finish_recording();
            self.menu.notice = "Recording ended before gameplay edit".into();
            eprintln!("Recording ended before gameplay edit; the saved prefix remains replayable.");
        }
    }

    fn set_sight(&mut self, value: u16) {
        if self.debug.sight_percent != value {
            self.stop_edited_recording();
        }
        self.debug.sight_percent = value;
        self.campaign.game.intelligence.sight_percent = value;
    }

    fn teleport(&mut self) -> Result<()> {
        if !self.campaign.playing() {
            return Err("Start a mission before teleporting".into());
        }
        self.stop_edited_recording();
        self.campaign
            .game
            .teleport_player(self.debug.target, self.debug.snap_to_floor)?;
        self.debug.teleport_armed = false;
        self.menu.notice = "Player teleported; hazards still apply".into();
        self.controls.clear();
        Ok(())
    }

    pub(super) fn settings_action(&mut self, action: Action) {
        let rows = settings::menu::rows(self.menu.page, &self.settings, &self.debug, self.paused);
        let item = rows[self.menu.row].item;
        if matches!(action, Action::Confirm)
            && let Item::Target(axis) = item
        {
            self.menu.edit = Some(self.debug.target[axis].to_string());
            self.update_view();
            return;
        }
        if let Some(direction) = self.menu.action(action, rows.len()) {
            if item.button() && !matches!(action, Action::Confirm) {
                return;
            }
            self.menu.notice.clear();
            let previous = self.settings.clone();
            if let Err(error) = self.apply_item(item, direction) {
                self.menu.notice = error;
            }
            if self.settings != previous
                && let Err(error) = self.settings_store.write(&self.settings)
            {
                eprintln!("Settings: {error}");
                self.menu.notice = "Could not save settings; see terminal".into();
            }
            if let Some(view) = &mut self.view {
                view.settings(&self.settings);
            }
            if let Some(audio) = &mut self.audio {
                audio.volume(self.settings.volume);
            }
        }
        self.release_pointer();
        self.sync_pause();
        self.update_view();
    }

    pub(super) fn apply_item(&mut self, item: Item, direction: i32) -> Result<()> {
        match item {
            Item::Fps => self.settings.show_fps = !self.settings.show_fps,
            Item::Focus => self.settings.pause_on_defocus = !self.settings.pause_on_defocus,
            Item::Fullscreen => self.settings.fullscreen = !self.settings.fullscreen,
            Item::Vsync => self.settings.vsync = !self.settings.vsync,
            Item::Hud => self.settings.show_hud = !self.settings.show_hud,
            Item::Volume => {
                self.settings.volume =
                    (i32::from(self.settings.volume) + direction * 5).clamp(0, 100) as u16
            }
            Item::Reset => {
                if self.debug.sight_percent != 100 {
                    self.stop_edited_recording();
                }
                self.settings = Settings::default();
                self.debug = crate::debug::Debug::new(&self.campaign.game)?;
                self.campaign.game.intelligence.sight_percent = 100;
            }
            Item::Fog => self.settings.graphics.fog = !self.settings.graphics.fog,
            Item::FogDistance => {
                self.settings.graphics.fog_distance =
                    (i32::from(self.settings.graphics.fog_distance) + direction * 25).clamp(25, 400)
                        as u16
            }
            Item::Textures => self.settings.graphics.textures = !self.settings.graphics.textures,
            Item::Shading => self.settings.graphics.shading = !self.settings.graphics.shading,
            Item::Exposure => {
                self.settings.graphics.exposure = (i32::from(self.settings.graphics.exposure)
                    + direction * 25)
                    .clamp(25, 300) as u16
            }
            Item::Filtering => self.settings.graphics.bilinear = !self.settings.graphics.bilinear,
            Item::Antialiasing => {
                if let Some(view) = &self.view {
                    let counts = &view.renderer.sample_counts;
                    let index = counts
                        .iter()
                        .position(|&n| n == self.settings.graphics.samples)
                        .unwrap_or(0);
                    self.settings.graphics.samples =
                        counts[(index as i32 + direction).rem_euclid(counts.len() as i32) as usize];
                }
            }
            Item::Hitboxes => self.debug.hitboxes = !self.debug.hitboxes,
            Item::Sight => self.debug.sight = !self.debug.sight,
            Item::SightStance => {
                self.debug.sight_stance =
                    (i32::from(self.debug.sight_stance) + direction).rem_euclid(3) as u8
            }
            Item::Collision => self.debug.collision = !self.debug.collision,
            Item::CollisionOnly => self.debug.collision_only = !self.debug.collision_only,
            Item::Patrols => self.debug.patrols = !self.debug.patrols,
            Item::Events => self.debug.events = !self.debug.events,
            Item::Labels => self.debug.labels = !self.debug.labels,
            Item::Xray => self.debug.xray = !self.debug.xray,
            Item::Trail => self.debug.show_trail = !self.debug.show_trail,
            Item::FreeCamera => self.debug.toggle_camera(&self.campaign.game),
            Item::FlightSpeed => {
                self.debug.flight_speed =
                    (i32::from(self.debug.flight_speed) + direction * 2).clamp(2, 100) as u16
            }
            Item::Fov => {
                let camera = self.debug.camera.get_or_insert_with(|| {
                    crate::debug::camera::Camera::from_game(&self.campaign.game)
                });
                camera.fov = (camera.fov + direction as f32 * 5.0).clamp(20.0, 110.0);
            }
            Item::FocusPlayer => {
                self.debug.selected_actor = self.campaign.game.player_id;
                self.debug.focus(&self.campaign.game, false);
            }
            Item::FocusActor => self.debug.focus(&self.campaign.game, false),
            Item::Overhead => self.debug.focus(&self.campaign.game, true),
            Item::SaveCamera => {
                self.debug.bookmark = Some(self.debug.camera.unwrap_or_else(|| {
                    crate::debug::camera::Camera::from_game(&self.campaign.game)
                }));
                self.menu.notice = "Camera bookmark saved".into();
            }
            Item::RestoreCamera => {
                self.debug.camera = Some(self.debug.bookmark.ok_or("No camera bookmark saved")?);
            }
            Item::Actor => self.debug.select(&self.campaign.game, direction),
            Item::InspectActor => self.debug.inspect_actor = !self.debug.inspect_actor,
            Item::Target(axis) => {
                self.debug.target[axis] =
                    (self.debug.target[axis] + direction * 100).clamp(-320_000, 320_000)
            }
            Item::PlayerPosition => self.debug.target = self.campaign.game.player().position,
            Item::CameraPosition => {
                self.debug.target = self
                    .debug
                    .camera
                    .ok_or("Enable free camera first")?
                    .position
                    .to_array()
                    .map(|v| v.round() as i32)
            }
            Item::SnapFloor => self.debug.snap_to_floor = !self.debug.snap_to_floor,
            Item::Teleport => self.teleport()?,
            Item::PickTeleport => {
                if !self.campaign.playing() {
                    return Err("Start a mission before teleporting".into());
                }
                self.debug.camera.get_or_insert_with(|| {
                    crate::debug::camera::Camera::from_game(&self.campaign.game)
                });
                self.debug.teleport_armed = true;
                self.menu.open = false;
            }
            Item::Pause => {
                if !self.campaign.playing() || self.faulted {
                    return Err("Simulation is not running".into());
                }
                self.paused = !self.paused;
                self.debug.pause_view = true;
                self.menu.open = false;
            }
            Item::Step => {
                if !self.campaign.playing() || self.faulted {
                    return Err("Simulation is not running".into());
                }
                self.paused = true;
                self.debug.pause_view = true;
                self.debug.frame_step = true;
                self.menu.open = false;
            }
            Item::Speed => {
                self.debug.speed = cycle(self.debug.speed, &[25, 50, 100, 200, 400], direction)
            }
            Item::CutsceneSpeed => {
                self.settings.cutscene_speed = cycle(
                    self.settings.cutscene_speed,
                    &[1, 2, 4, 8, 16, 0],
                    direction,
                )
            }
            Item::SightDistance => {
                let value =
                    (i32::from(self.debug.sight_percent) + direction * 25).clamp(0, 400) as u16;
                self.set_sight(value);
            }
            Item::Screenshot => {
                self.screenshot = true;
                self.menu.open = false;
            }
            Item::ResetTweaks => {
                self.debug.speed = 100;
                self.set_sight(100);
            }
        }
        Ok(())
    }

    pub(super) fn edit_coordinate(&mut self, key: KeyCode, text: Option<&str>) {
        if key == KeyCode::Escape {
            self.menu.edit = None;
        } else if key == KeyCode::Enter {
            let input = self.menu.edit.take().unwrap();
            if let Ok(value) = input.parse::<i32>()
                && (-320_000..=320_000).contains(&value)
            {
                if let Item::Target(axis) =
                    settings::menu::rows(self.menu.page, &self.settings, &self.debug, self.paused)
                        [self.menu.row]
                        .item
                {
                    self.debug.target[axis] = value;
                }
            } else {
                self.menu.notice = "Enter a coordinate within +/-320000".into();
            }
        } else if let Some(input) = &mut self.menu.edit {
            if key == KeyCode::Backspace {
                input.pop();
            } else if key == KeyCode::Delete {
                input.clear();
            } else if let Some(text) = text {
                for ch in text.chars() {
                    if input.len() < 8 && (ch.is_ascii_digit() || ch == '-' && input.is_empty()) {
                        input.push(ch);
                    }
                }
            }
        }
        self.update_view();
    }

    pub(super) fn save_screenshot(&mut self) {
        self.screenshot = false;
        let result = (|| -> Result<_> {
            let directory = std::env::var_os("TENCHU_SCREENSHOTS")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("screenshots"));
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let path = directory.join(format!("tenchu-{}-{time}.png", self.campaign.game.tick));
            self.view.as_ref().ok_or("Window is not ready")?.capture(
                &self.campaign.game,
                &self.debug,
                &path,
            )?;
            Ok(path)
        })();
        self.menu.notice = match result {
            Ok(_) => "Screenshot saved; path in terminal".into(),
            Err(error) => {
                eprintln!("Screenshot: {error}");
                format!("Screenshot: {error}")
            }
        };
        self.update_view();
    }
}
