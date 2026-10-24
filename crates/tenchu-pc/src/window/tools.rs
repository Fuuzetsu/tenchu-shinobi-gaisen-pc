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

    pub(super) fn set_sight(&mut self, value: u16) {
        if self.debug.sight_percent != value {
            self.stop_edited_recording();
        }
        self.debug.sight_percent = value;
        self.campaign.game.intelligence.sight_percent = value;
    }

    pub(super) fn keep_enemies_active(&mut self, enabled: bool) {
        if self.campaign.game.activation.keep_all_active != enabled {
            self.stop_edited_recording();
        }
        self.settings.keep_enemies_active = enabled;
        self.campaign.game.activation.keep_all_active = enabled;
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
            self.menu.edit = Some((item, self.debug.target[axis].to_string()));
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
            self.apply_settings(&previous);
        }
        self.release_pointer();
        self.sync_pause();
        self.update_view();
    }

    pub(super) fn apply_settings(&mut self, previous: &Settings) {
        if let Some(view) = &mut self.view
            && let Err(error) = view.settings(&self.settings, &self.campaign.game)
        {
            self.menu.notice = error;
            self.settings = previous.clone();
            self.campaign.game.activation.keep_all_active = previous.keep_enemies_active;
            return;
        }
        if self.demo.is_none()
            && self.settings != *previous
            && let Err(error) = self.settings_store.write(&self.settings)
        {
            eprintln!("Settings: {error}");
            self.menu.notice = "Could not save settings; see terminal".into();
        }
        if let Some(audio) = &mut self.audio {
            audio.volume(self.settings.volume);
        }
    }

    pub(super) fn apply_item(&mut self, item: Item, direction: i32) -> Result<()> {
        match item {
            Item::Blood => {
                self.settings.graphics.blood.enabled = !self.settings.graphics.blood.enabled
            }
            Item::ClearBlood => {
                if let Some(view) = &mut self.view {
                    view.blood.clear();
                }
            }
            Item::BloodAmount | Item::BloodSeconds | Item::BloodMarks => {
                let blood = &mut self.settings.graphics.blood;
                let (value, step, min, max) = match item {
                    Item::BloodAmount => (&mut blood.amount, 10, 0, 200),
                    Item::BloodSeconds => (&mut blood.seconds, 2, 0, 60),
                    _ => (&mut blood.marks, 32, 32, 1024),
                };
                *value = (i32::from(*value) + direction * step).clamp(min, max) as u16;
            }

            Item::Interpolation => {
                self.settings.graphics.interpolation = !self.settings.graphics.interpolation
            }
            Item::PhotoMode => self.debug.photo_mode = !self.debug.photo_mode,
            Item::PlayerModel => {
                if let Some(view) = &self.view {
                    let choices = &view.player_models;
                    let index = choices
                        .iter()
                        .position(|(id, _)| id == &self.settings.player_model)
                        .unwrap_or(0);
                    self.settings.player_model = choices
                        [(index as i32 + direction).rem_euclid(choices.len() as i32) as usize]
                        .0
                        .clone();
                }
            }
            Item::ReloadModels => {
                if let Some(view) = &mut self.view {
                    view.load_bodies(
                        &self.menu.model_directory,
                        &self.campaign.game,
                        &self.settings,
                    )?;
                    if !view
                        .player_models
                        .iter()
                        .any(|(id, _)| id == &self.settings.player_model)
                    {
                        self.settings.player_model.clear();
                    }
                    self.settings
                        .model_directory
                        .clone_from(&self.menu.model_directory);
                    self.menu.notice =
                        format!("{} player bodies available", view.player_models.len() - 1);
                }
            }
            Item::Preset => {
                self.settings.graphics.cycle_preset(direction);
            }
            Item::Post => {
                self.settings.graphics.post.enabled = !self.settings.graphics.post.enabled
            }
            Item::Filmic => {
                self.settings.graphics.post.filmic = !self.settings.graphics.post.filmic
            }
            Item::FogTint => {
                self.settings.graphics.post.fog_tint = cycle(
                    self.settings.graphics.post.fog_tint,
                    &[0, 1, 2, 3],
                    direction,
                )
            }
            Item::Temperature => {
                self.settings.graphics.post.temperature =
                    (i32::from(self.settings.graphics.post.temperature) + direction * 5)
                        .clamp(-50, 50) as i16
            }
            Item::Bloom
            | Item::BloomThreshold
            | Item::Occlusion
            | Item::OcclusionRadius
            | Item::Contrast
            | Item::Saturation
            | Item::Gamma
            | Item::Vignette
            | Item::Sharpen
            | Item::Haze => {
                let post = &mut self.settings.graphics.post;
                let (value, step, min, max) = match item {
                    Item::Bloom => (&mut post.bloom, 5, 0, 100),
                    Item::BloomThreshold => (&mut post.bloom_threshold, 25, 25, 300),
                    Item::Occlusion => (&mut post.occlusion, 5, 0, 100),
                    Item::OcclusionRadius => (&mut post.occlusion_radius, 1, 2, 20),
                    Item::Contrast => (&mut post.contrast, 5, 50, 150),
                    Item::Saturation => (&mut post.saturation, 5, 0, 200),
                    Item::Gamma => (&mut post.gamma, 5, 50, 150),
                    Item::Vignette => (&mut post.vignette, 5, 0, 100),
                    Item::Sharpen => (&mut post.sharpen, 5, 0, 100),
                    _ => (&mut post.haze, 5, 0, 100),
                };
                *value = (i32::from(*value) + direction * step).clamp(min, max) as u16;
            }
            Item::SmoothModels => {
                self.settings.graphics.smooth_models = !self.settings.graphics.smooth_models
            }
            Item::ModelDetail => {
                self.settings.graphics.model_detail =
                    cycle(self.settings.graphics.model_detail, &[0, 1, 2], direction)
            }
            Item::AssetPack => {
                self.settings.graphics.asset_pack = !self.settings.graphics.asset_pack
            }
            Item::SharpText => self.settings.sharp_text = !self.settings.sharp_text,
            Item::Upscale => {
                self.settings.graphics.upscale =
                    cycle(self.settings.graphics.upscale, &[1, 2, 4], direction)
            }
            Item::SurfaceDetail => {
                self.settings.graphics.surface_detail =
                    cycle(self.settings.graphics.surface_detail, &[0, 1, 2], direction)
            }
            Item::SurfaceStrength => {
                self.settings.graphics.surface_strength =
                    (i32::from(self.settings.graphics.surface_strength) + direction * 5)
                        .clamp(0, 100) as u16
            }
            Item::Magnification => {
                self.settings.graphics.magnification = cycle(
                    self.settings.graphics.magnification,
                    &[0, 1, 2, 3],
                    direction,
                )
            }
            Item::ReloadAssets => {
                let pack = crate::visuals::pack::Pack::from_env(&self.campaign.game.mission)?;
                let textures = pack.textures.len();
                if let Some(view) = &mut self.view {
                    let models = view.load_pack(pack, &self.campaign.game, &self.settings)?;
                    self.menu.notice = format!("Loaded {textures} textures, {models} model parts");
                }
                if let Some(frontend) = &mut self.frontend {
                    let images = frontend.reload_artwork()?;
                    self.menu
                        .notice
                        .push_str(&format!(", {images} frontend images"));
                }
            }
            Item::Lighting => {
                self.settings.graphics.lighting.enabled = !self.settings.graphics.lighting.enabled
            }
            Item::SkyTint => {
                self.settings.graphics.lighting.sky_tint =
                    (i32::from(self.settings.graphics.lighting.sky_tint) + direction * 10)
                        .clamp(0, 100) as u16
            }
            Item::DirectionalIntensity => {
                self.settings.graphics.lighting.directional.intensity =
                    (i32::from(self.settings.graphics.lighting.directional.intensity)
                        + direction * 10)
                        .clamp(0, 200) as u16
            }
            Item::DirectionalColor => {
                self.settings.graphics.lighting.directional.color = cycle(
                    self.settings.graphics.lighting.directional.color,
                    &[0, 1, 2],
                    direction,
                )
            }
            Item::DirectionalAzimuth => {
                self.settings.graphics.lighting.directional.azimuth =
                    (i32::from(self.settings.graphics.lighting.directional.azimuth)
                        + direction * 15)
                        .rem_euclid(360) as u16
            }
            Item::DirectionalElevation => {
                self.settings.graphics.lighting.directional.elevation =
                    (i32::from(self.settings.graphics.lighting.directional.elevation)
                        + direction * 5)
                        .clamp(10, 85) as u16
            }
            Item::DirectionalRadius => {
                self.settings.graphics.lighting.directional.radius =
                    (i32::from(self.settings.graphics.lighting.directional.radius) + direction * 5)
                        .clamp(10, 60) as u16
            }
            Item::DirectionalResolution => {
                self.settings.graphics.lighting.directional.resolution = u32::from(cycle(
                    self.settings.graphics.lighting.directional.resolution as u16,
                    &[512, 1024, 2048, 4096],
                    direction,
                ))
            }
            Item::Shadows => {
                self.settings.graphics.lighting.shadows = !self.settings.graphics.lighting.shadows
            }
            Item::FireLights => {
                self.settings.graphics.lighting.fires = !self.settings.graphics.lighting.fires
            }
            Item::LampLights => {
                self.settings.graphics.lighting.lamps = !self.settings.graphics.lighting.lamps
            }
            Item::LightMarkers => {
                self.settings.graphics.lighting.markers = !self.settings.graphics.lighting.markers
            }
            Item::ShadowSoftness => {
                self.settings.graphics.lighting.softness =
                    (i32::from(self.settings.graphics.lighting.softness) + direction).clamp(0, 8)
                        as u16
            }
            Item::ShadowCount => {
                self.settings.graphics.lighting.shadow_count = cycle(
                    self.settings.graphics.lighting.shadow_count,
                    &[0, 1, 2, 3, 4],
                    direction,
                )
            }
            Item::LightCount => {
                self.settings.graphics.lighting.count = cycle(
                    self.settings.graphics.lighting.count,
                    &[0, 1, 2, 4, 8, 16],
                    direction,
                )
            }
            Item::BlobShadows => {
                self.settings.graphics.blob_shadows = !self.settings.graphics.blob_shadows
            }
            Item::ShadowPreview => {
                if !self.campaign.playing() {
                    return Err("Start a mission to preview model shadows".into());
                }
                self.settings.graphics.lighting.enabled = true;
                self.settings.graphics.lighting.shadows = true;
                self.settings.graphics.lighting.fires = false;
                self.settings.graphics.lighting.lamps = false;
                self.settings.graphics.lighting.shadow_count = 1;
                self.settings.graphics.lighting.ambient = 25;
                self.settings.graphics.lighting.intensity = 250;
                self.settings.graphics.lighting.radius = 12;
                self.settings.graphics.lighting.resolution = 1024;
                self.settings.graphics.blob_shadows = false;
                self.apply_item(Item::PlayerLight, 1)?;
                self.menu.notice = "Preview ready: close F1; move or attack".into();
            }
            Item::LightRadius => {
                self.settings.graphics.lighting.radius =
                    (i32::from(self.settings.graphics.lighting.radius) + direction).clamp(2, 50)
                        as u16
            }
            Item::LightIntensity => {
                self.settings.graphics.lighting.intensity =
                    (i32::from(self.settings.graphics.lighting.intensity) + direction * 10)
                        .clamp(0, 400) as u16
            }
            Item::LightAmbient => {
                self.settings.graphics.lighting.ambient =
                    (i32::from(self.settings.graphics.lighting.ambient) + direction * 5)
                        .clamp(0, 100) as u16
            }
            Item::LightResolution => {
                self.settings.graphics.lighting.resolution = u32::from(cycle(
                    self.settings.graphics.lighting.resolution as u16,
                    &[256, 512, 1024, 2048],
                    direction,
                ))
            }
            Item::DebugLight => {
                self.debug.light = if self.debug.light.is_some() {
                    None
                } else {
                    Some(self.debug.eye(&self.campaign.game))
                };
                self.debug.light_follow = false;
            }
            Item::FocusLight => {
                let game = &self.campaign.game;
                let source = self
                    .view
                    .as_ref()
                    .and_then(|v| v.nearest_lamp(game, self.debug.eye(game)))
                    .ok_or("No lantern emitters found in this mission")?;
                let mut camera = self
                    .debug
                    .camera
                    .unwrap_or_else(|| crate::debug::camera::Camera::from_game(game));
                let direction = (source.light.position - camera.position).normalize_or_zero();
                camera.yaw = direction.x.atan2(direction.z);
                camera.focus(source.light.position, false);
                camera.position = source.light.position - camera.forward() * 4500.0;
                self.debug.camera = Some(camera);
                self.settings.graphics.lighting.markers = true;
                self.menu.notice = format!(
                    "{} at {:.0?}",
                    source.name,
                    source.light.position.to_array()
                );
            }
            Item::PlaceLight => {
                self.debug.light = Some(self.debug.eye(&self.campaign.game));
                self.debug.light_follow = false;
            }
            Item::PlayerLight => {
                self.debug.light = Some(
                    glam::Vec3::from(
                        self.campaign
                            .game
                            .player()
                            .position
                            .map(|v| v as f32)
                            .to_array(),
                    ) + glam::Vec3::new(2000.0, -4000.0, 2000.0),
                );
                self.debug.light_follow = false;
            }
            Item::FollowLight => {
                self.debug.light = self.debug.light_position(&self.campaign.game);
                self.debug.light_follow = !self.debug.light_follow;
                self.debug
                    .light
                    .get_or_insert(self.debug.eye(&self.campaign.game));
            }
            Item::LightAxis(axis) => {
                self.debug.light_follow = false;
                let eye = self.debug.eye(&self.campaign.game);
                let light = self.debug.light.get_or_insert(eye);
                light[axis] = (light[axis] + direction as f32 * 500.0).clamp(-320_000.0, 320_000.0);
            }
            Item::Performance => self.settings.show_performance = !self.settings.show_performance,
            Item::Fps => self.settings.show_fps = !self.settings.show_fps,
            Item::Focus => self.settings.pause_on_defocus = !self.settings.pause_on_defocus,
            Item::MenuPause => self.settings.pause_in_menu = !self.settings.pause_in_menu,
            Item::Fullscreen => self.settings.fullscreen = !self.settings.fullscreen,
            Item::Vsync => self.settings.vsync = !self.settings.vsync,
            Item::Hud => self.settings.show_hud = !self.settings.show_hud,
            Item::Volume => {
                self.settings.volume =
                    (i32::from(self.settings.volume) + direction * 5).clamp(0, 100) as u16
            }
            Item::Reset => {
                self.keep_enemies_active(false);
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
            Item::Filtering => {
                let next = cycle(
                    self.settings.graphics.filtering(),
                    &[0, 1, 2, 3, 4, 5, 6],
                    direction,
                );
                self.settings.graphics.set_filtering(next);
            }
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
                    .into()
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
                self.debug.speed = cycle(self.debug.speed, &[25, 50, 100, 200, 400, 0], direction)
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
            Item::KeepEnemiesActive => self.keep_enemies_active(!self.settings.keep_enemies_active),
            Item::Screenshot => {
                self.screenshot = true;
                self.menu.open = false;
            }
            Item::ResetTweaks => {
                self.debug.speed = 100;
                self.set_sight(100);
                self.keep_enemies_active(false);
            }
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
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
            let name = if let Some(demo) = &self.demo {
                format!("demo-{}-{:05}-{time}.png", demo.clip().id, demo.frame)
            } else {
                format!("tenchu-{}-{time}.png", self.campaign.game.tick)
            };
            let path = directory.join(name);
            self.view.as_mut().ok_or("Window is not ready")?.capture(
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
