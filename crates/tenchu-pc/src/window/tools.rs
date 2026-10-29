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
        if self.debug.teleport_armed || self.debug.click_teleport {
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

    fn mission_edit(&self) -> Result<()> {
        if !self.campaign.playing()
            || self.campaign.game.cinematic.active
            || self.campaign.game.player().health <= 0
        {
            return Err("Experiment controls require a living player outside a cutscene".into());
        }
        Ok(())
    }

    pub(super) fn set_god_mode(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            self.mission_edit()?;
        }
        self.stop_edited_recording();
        let player = self.campaign.game.player_id;
        self.campaign.game.roster.get_mut(player).invulnerable = enabled;
        self.debug.update(&self.campaign.game);
        Ok(())
    }

    pub(super) fn set_stealth(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            self.mission_edit()?;
        }
        self.stop_edited_recording();
        self.campaign.game.intelligence.ignore_player = enabled;
        if enabled {
            self.campaign.game.intelligence.alert = 0;
        }
        self.debug.update(&self.campaign.game);
        Ok(())
    }

    pub(super) fn set_inventory(&mut self, slot: usize, count: u8) -> Result<()> {
        self.mission_edit()?;
        self.stop_edited_recording();
        self.campaign.game.set_inventory(slot, count)?;
        self.debug.update(&self.campaign.game);
        Ok(())
    }

    pub(super) fn set_sight(&mut self, value: u16) {
        if self.debug.sight_percent != value {
            self.stop_edited_recording();
        }
        self.settings.sight_percent = value;
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
        if (self.settings.model_moveset != previous.model_moveset
            || self.settings.model_moveset_options != previous.model_moveset_options
            || self.settings.player_model != previous.player_model)
            && let Err(error) = self.sync_model_moveset()
        {
            self.settings = previous.clone();
            self.menu.notice = error;
            return;
        }
        if let Some(view) = &mut self.view
            && let Err(error) = view.settings(&self.settings, &self.campaign.game)
        {
            self.menu.notice = error;
            self.settings = previous.clone();
            self.campaign.game.activation.keep_all_active = previous.keep_enemies_active;
            self.set_sight(previous.sight_percent);
            let _ = self.sync_model_moveset();
            return;
        }
        if self.settings.vibration != previous.vibration
            || self.settings.frontend.rumble != previous.frontend.rumble
        {
            self.controls.stop_rumble();
        }
        if let Some(frontend) = &mut self.frontend {
            frontend.set_rumble(self.settings.frontend.rumble);
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

    pub(super) fn sync_model_moveset(&mut self) -> Result<()> {
        let Some(view) = &mut self.view else {
            return Ok(());
        };
        if self.settings.model_moveset
            && !self.settings.player_model.is_empty()
            && view.model_library.is_none()
        {
            view.load_model_library(&self.assets, &self.campaign.game, &self.settings)?;
        }
        let body = self
            .settings
            .model_moveset
            .then(|| {
                view.model_library
                    .as_ref()
                    .and_then(|l| l.get(&self.settings.player_model))
            })
            .flatten();
        let character = body.and_then(|b| b.character.as_ref());
        let options = self.settings.model_moveset_options;
        if self
            .campaign
            .game
            .moveset
            .matches(character.map(|c| c.kind), options)
        {
            return Ok(());
        }
        self.campaign.game.set_player_moveset(
            character,
            body.map_or(&[], |b| &b.clips),
            options,
        )?;
        view.reset_interpolation(&self.campaign.game);
        self.stop_edited_recording();
        Ok(())
    }

    pub(super) fn set_gameplay_preset(&mut self, index: usize) -> Result<()> {
        let preset = settings::gameplay::PRESETS
            .get(index)
            .ok_or("Unknown gameplay preset")?;
        if preset.practice {
            self.mission_edit()?;
        }
        self.keep_enemies_active(preset.active);
        self.set_sight(preset.sight);
        self.settings.cutscene_speed = preset.cutscenes;
        self.debug.speed = 100;
        self.set_god_mode(preset.practice)?;
        self.set_stealth(preset.practice)?;
        self.debug.click_teleport = preset.practice;
        if self.debug.armor {
            self.apply_item(Item::Armor, 1)?;
        }
        Ok(())
    }

    pub(super) fn apply_item(&mut self, item: Item, direction: i32) -> Result<()> {
        if item.change_setting(&mut self.settings, direction) {
            return Ok(());
        }
        match item {
            Item::GodMode => self.set_god_mode(!self.debug.god_mode)?,
            Item::Stealth => self.set_stealth(!self.debug.stealth)?,
            Item::ClickTeleport => {
                self.mission_edit()?;
                self.debug.click_teleport = !self.debug.click_teleport;
            }
            Item::Inventory(slot) => {
                let count = (i32::from(self.debug.inventory[slot]) + direction).clamp(0, 255) as u8;
                self.set_inventory(slot, count)?;
            }
            Item::GiveItems | Item::ClearItems => {
                for slot in 0..19 {
                    self.set_inventory(
                        slot,
                        if item == Item::ClearItems {
                            0
                        } else if slot == 0 {
                            255
                        } else {
                            10
                        },
                    )?;
                }
            }
            Item::Armor => {
                self.mission_edit()?;
                self.stop_edited_recording();
                self.campaign.game.combat.armor = !self.campaign.game.combat.armor;
                self.debug.update(&self.campaign.game);
            }
            Item::CharacterRefinement | Item::FoliageDetail => {
                let options = &mut self.settings.graphics.models;
                let value = if item == Item::CharacterRefinement {
                    &mut options.characters
                } else {
                    &mut options.foliage
                };
                *value = (i32::from(*value) + direction)
                    .rem_euclid(if item == Item::FoliageDetail { 5 } else { 3 })
                    as u16;
            }
            Item::WaterDetail => {
                self.settings.graphics.water.detail =
                    (i32::from(self.settings.graphics.water.detail) + direction).rem_euclid(3)
                        as u16
            }
            Item::Weather => {
                let modes = settings::WeatherMode::ALL;
                let index = modes
                    .iter()
                    .position(|&mode| mode == self.settings.graphics.weather.mode)
                    .unwrap_or(0);
                self.settings.graphics.weather.mode =
                    modes[(index as i32 + direction).rem_euclid(modes.len() as i32) as usize];
            }
            Item::WeatherQuality => {
                self.settings.graphics.weather.quality =
                    (i32::from(self.settings.graphics.weather.quality) + direction).rem_euclid(3)
                        as u16;
            }
            Item::ClearWeather => {
                self.settings.graphics.weather.mode = settings::WeatherMode::Clear;
                if let Some(view) = &mut self.view {
                    view.clear_weather();
                }
            }
            Item::ClearFootprints => {
                if let Some(view) = &mut self.view {
                    view.footprints.clear();
                }
            }
            Item::ClearBlood => {
                if let Some(view) = &mut self.view {
                    view.blood.clear();
                }
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
            Item::GameplayPreset => {
                let presets = &settings::gameplay::PRESETS;
                let current = settings::gameplay::current(&self.settings, &self.debug);
                let index = current.map_or_else(
                    || if direction < 0 { presets.len() - 1 } else { 0 },
                    |i| (i as i32 + direction).rem_euclid(presets.len() as i32) as usize,
                );
                self.set_gameplay_preset(index)?;
            }
            Item::Preset => {
                let choices = self.presets.library.choices();
                let current = self.presets.library.current(&self.settings);
                let index = choices
                    .iter()
                    .position(|(choice, _)| Some(*choice) == current)
                    .map_or_else(
                        || if direction < 0 { choices.len() - 1 } else { 0 },
                        |i| (i as i32 + direction).rem_euclid(choices.len() as i32) as usize,
                    );
                self.presets
                    .library
                    .apply(choices[index].0, &mut self.settings)?;
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
            Item::ModelDetail => {
                self.settings.graphics.model_detail =
                    cycle(self.settings.graphics.model_detail, &[0, 1, 2], direction)
            }
            Item::Upscale => {
                self.settings.graphics.upscale =
                    cycle(self.settings.graphics.upscale, &[1, 2, 4], direction)
            }
            Item::SurfaceDetail => {
                self.settings.graphics.surface_detail =
                    cycle(self.settings.graphics.surface_detail, &[0, 1, 2], direction)
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
            Item::DirectionalColor => {
                self.settings.graphics.lighting.directional.color = cycle(
                    self.settings.graphics.lighting.directional.color,
                    &[0, 1, 2],
                    direction,
                )
            }
            Item::DirectionalResolution => {
                self.settings.graphics.lighting.directional.resolution = u32::from(cycle(
                    self.settings.graphics.lighting.directional.resolution as u16,
                    &[512, 1024, 2048, 4096],
                    direction,
                ))
            }
            Item::ShadowCount => {
                self.settings.graphics.lighting.shadow_count = cycle(
                    self.settings.graphics.lighting.shadow_count,
                    &[0, 1, 2, 3, 4, 6, 8],
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
            Item::Reset => {
                self.set_god_mode(false)?;
                self.set_stealth(false)?;
                self.keep_enemies_active(false);
                if self.debug.sight_percent != 100 {
                    self.stop_edited_recording();
                }
                self.settings = Settings::default();
                self.campaign.game.intelligence.sight_percent = 100;
                self.debug = crate::debug::Debug::new(&self.campaign.game)?;
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
                self.set_god_mode(false)?;
                self.set_stealth(false)?;
                self.debug.click_teleport = false;
                self.debug.speed = 100;
                self.set_sight(100);
                self.keep_enemies_active(false);
            }
            _ => return Err("This control has no editable value".into()),
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
