use super::*;

impl Application {
    pub(super) fn pick_debug_target(&mut self) {
        if self.debug.photo_mode {
            return;
        }
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
        if self.debug.teleport_armed || self.debug.click_teleport || self.debug.pick_actor_position
        {
            if let Some(point) =
                crate::debug::terrain::pick(&self.debug.surfaces, origin, direction)
            {
                self.debug.target = point;
                if self.debug.pick_actor_position {
                    self.debug.pick_actor_position = false;
                    self.menu.open = true;
                    self.menu.page(4 - self.menu.page as i32);
                    self.menu.reveal_row = false;
                    self.release_pointer();
                    self.update_view();
                    return;
                }
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
            let picked = self
                .debug
                .pick_actor(&self.campaign.game, origin, direction);
            if self.debug.click_kill {
                let result = picked
                    .ok_or_else(|| "No NPC at that point".to_owned())
                    .and_then(|_| self.edit_actor(Item::KillActor));
                if let Err(error) = result {
                    self.menu.notice = error;
                }
                self.debug.notice = Some((self.menu.notice.clone(), web_time::Instant::now()));
            }
        }
        self.update_view();
    }

    pub(super) fn release_pointer(&mut self) {
        self.debug.looking = false;
        self.orbit.looking = false;
        if let Some(view) = &self.view {
            let _ = view.window.set_cursor_grab(CursorGrabMode::None);
            view.window.set_cursor_visible(true);
        }
    }

    pub(super) fn stop_edited_recording(&mut self) {
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
        if self.campaign.game.roster.keep_all_active != enabled {
            self.stop_edited_recording();
        }
        self.settings.keep_enemies_active = enabled;
        self.settings
            .apply_activation(&mut self.campaign.game.roster);
    }

    pub(super) fn set_independent_attacks(&mut self, enabled: bool) {
        if self.campaign.game.intelligence.independent_attacks != enabled {
            self.stop_edited_recording();
        }
        self.settings.independent_attacks = enabled;
        self.campaign.game.intelligence.independent_attacks = enabled;
    }

    pub(super) fn set_multi_target_attacks(&mut self, enabled: bool) {
        if self.campaign.game.combat.multi_target != enabled {
            self.stop_edited_recording();
        }
        self.settings.multi_target_attacks = enabled;
        self.campaign.game.combat.multi_target = enabled;
    }

    pub(super) fn set_attack_assist(&mut self, enabled: bool) {
        if self.campaign.game.combat.attack_assist != enabled {
            self.stop_edited_recording();
        }
        self.settings.attack_assist = enabled;
        self.campaign.game.combat.attack_assist = enabled;
    }

    pub(super) fn set_strict_corpse_sight(&mut self, enabled: bool) {
        if self.campaign.game.intelligence.strict_corpse_sight != enabled {
            self.stop_edited_recording();
        }
        self.settings.strict_corpse_sight = enabled;
        self.campaign.game.intelligence.strict_corpse_sight = enabled;
    }

    pub(super) fn set_circle_action(&mut self, action: tenchu_core::input::CircleAction) {
        if self.campaign.game.intelligence.circle_action != action {
            self.stop_edited_recording();
            self.controls.clear();
        }
        self.settings.circle_action = action;
        self.campaign.game.intelligence.circle_action = action;
    }

    pub(super) fn set_guard_controls(&mut self, enabled: bool) {
        if self.campaign.game.intelligence.guard_controls != enabled {
            self.stop_edited_recording();
            self.controls.clear();
        }
        self.settings.guard_controls = enabled;
        self.campaign.game.intelligence.guard_controls = enabled;
        self.campaign.game.intelligence.guard_draw = self.settings.guard_draw;
    }

    pub(super) fn set_sheathing(
        &mut self,
        mut settings: tenchu_core::game::sheathing::Settings,
    ) -> Result<()> {
        settings.validate();
        let voices = self.campaign.game.mission.imported_voices.len();
        crate::sheathing::configure(&mut self.campaign.game, &self.assets, settings)?;
        if self.campaign.game.mission.imported_voices.len() != voices
            && let Some(audio) = &mut self.audio
        {
            audio.refresh_actor_bank(&self.assets, &self.campaign.game)?;
        }
        if self.settings.sheathing != settings {
            self.stop_edited_recording();
            self.controls.clear();
        }
        self.settings.sheathing = settings;
        Ok(())
    }

    pub(super) fn set_walking(&mut self, enabled: bool) -> Result<()> {
        if self.campaign.game.walking.is_some() != enabled {
            crate::walking::configure(&mut self.campaign.game, &self.assets, enabled)?;
            self.stop_edited_recording();
            self.controls.clear();
        }
        self.settings.walking.enabled = enabled;
        Ok(())
    }

    pub(super) fn set_dismemberment(
        &mut self,
        mut settings: tenchu_core::game::dismemberment::Settings,
    ) {
        settings.validate();
        if self.campaign.game.combat.dismemberment != settings {
            self.stop_edited_recording();
        }
        self.settings.dismemberment = settings;
        self.campaign.game.combat.dismemberment = settings;
    }

    pub(super) fn set_bleeding(&mut self, mut settings: tenchu_core::game::bleeding::Settings) {
        settings.validate();
        if self
            .campaign
            .game
            .player()
            .bleeding
            .as_ref()
            .map(|s| s.settings)
            != settings.enabled.then_some(settings)
        {
            self.stop_edited_recording();
        }
        self.settings.bleeding = settings;
        self.campaign.game.set_bleeding(settings);
    }

    pub(super) fn set_damage_escalation(
        &mut self,
        mut settings: tenchu_core::ai::escalation::Settings,
    ) {
        settings.validate();
        if self.campaign.game.intelligence.damage_escalation != settings {
            self.stop_edited_recording();
        }
        self.settings.damage_escalation = settings;
        self.campaign.game.set_damage_escalation(settings);
    }

    pub(super) fn set_quick_shuriken_dodge(&mut self, enabled: bool) {
        if self.campaign.game.intelligence.quick_shuriken_dodge != enabled {
            self.stop_edited_recording();
        }
        self.settings.quick_shuriken_dodge = enabled;
        self.campaign.game.intelligence.quick_shuriken_dodge = enabled;
    }

    pub(super) fn set_reinforcement_limit(&mut self, limit: u16) {
        if self.settings.reinforcement_limit != limit && self.campaign.game.roster.keep_all_active {
            self.stop_edited_recording();
        }
        self.settings.reinforcement_limit = limit;
        self.campaign.game.roster.reinforcement_limit = (limit != 0).then_some(usize::from(limit));
    }

    pub(super) fn set_reinforcement_call(&mut self, call: tenchu_core::roster::ReinforcementCall) {
        if self.settings.reinforcement_call != call && self.campaign.game.roster.keep_all_active {
            self.stop_edited_recording();
        }
        self.settings.reinforcement_call = call;
        self.campaign.game.roster.reinforcement_call = call;
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
            if !matches!(item, Item::LoadState | Item::Rewind) {
                self.apply_settings(&previous);
            }
        }
        self.release_pointer();
        self.sync_pause();
        self.update_view();
    }

    pub(super) fn apply_settings(&mut self, previous: &Settings) {
        if self.settings.dismemberment != previous.dismemberment {
            self.set_dismemberment(self.settings.dismemberment);
        }
        if self.settings.bleeding != previous.bleeding {
            self.set_bleeding(self.settings.bleeding);
        }
        if self.settings.rewind_seconds != previous.rewind_seconds {
            self.clear_rewind();
            self.record_rewind(None);
        }
        if self.settings.navigation.camera_relative != previous.navigation.camera_relative {
            self.orbit = Default::default();
            self.campaign.game.navigation = Default::default();
            self.controls.clear();
            self.release_pointer();
        }
        if (self.settings.model_moveset != previous.model_moveset
            || self.settings.moveset_model != previous.moveset_model
            || self.settings.weapon_model != previous.weapon_model
            || self.settings.voice_model != previous.voice_model
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
            previous.apply_activation(&mut self.campaign.game.roster);
            self.set_sight(previous.sight_percent);
            self.set_independent_attacks(previous.independent_attacks);
            self.set_quick_shuriken_dodge(previous.quick_shuriken_dodge);
            self.set_strict_corpse_sight(previous.strict_corpse_sight);
            self.set_guard_controls(previous.guard_controls);
            self.set_circle_action(previous.circle_action);
            let _ = self.set_walking(previous.walking.enabled);
            let _ = self.set_sheathing(previous.sheathing);
            self.set_bleeding(previous.bleeding);
            self.set_dismemberment(previous.dismemberment);
            self.set_damage_escalation(previous.damage_escalation);
            self.set_multi_target_attacks(previous.multi_target_attacks);
            self.set_attack_assist(previous.attack_assist);
            self.set_behavior_profiles(previous.behavior_profiles);
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
        if !self.settings.wait_after_death
            && matches!(self.campaign.screen, Screen::DeathHold { .. })
        {
            if let Err(error) = self.campaign.action(Action::Confirm, &self.assets) {
                self.pause_error(error);
                return;
            }
            self.save();
            if let Some(audio) = &mut self.audio {
                audio.screen(&self.campaign.screen);
            }
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
        if self.campaign.randomized.is_some()
            || self
                .demo
                .as_ref()
                .is_some_and(|demo| demo.randomizer_plan().is_some())
        {
            return Ok(());
        }
        let Some(view) = &mut self.view else {
            return Ok(());
        };
        if (self.settings.moveset_id().is_some_and(|id| !id.is_empty())
            || !self.settings.weapon_model.is_empty()
            || !self.settings.voice_model.is_empty())
            && view.model_library.is_none()
        {
            view.load_model_library(&self.assets, &self.campaign.game, &self.settings)?;
        }
        let weapons = self.campaign.game.mission.weapons.imported_models.clone();
        let changed = if let Some(library) = &view.model_library {
            library.apply_player(&mut self.campaign.game, &self.settings)?
        } else {
            let changed = !self.campaign.game.moveset.matches(None, Default::default());
            self.campaign
                .game
                .set_player_moveset(None, &[], Default::default())?;
            changed
        };
        if !changed {
            return Ok(());
        }
        if !std::sync::Arc::ptr_eq(
            &weapons,
            &self.campaign.game.mission.weapons.imported_models,
        ) {
            view.refresh_actor_assets(&self.campaign.game, &self.settings)?;
        } else {
            view.reset_interpolation(&self.campaign.game);
        }
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
        self.set_independent_attacks(false);
        self.set_quick_shuriken_dodge(false);
        self.set_strict_corpse_sight(false);
        self.set_guard_controls(false);
        self.set_circle_action(Default::default());
        self.set_walking(false)?;
        self.set_sheathing(Default::default())?;
        self.set_bleeding(Default::default());
        self.set_dismemberment(Default::default());
        self.set_damage_escalation(Default::default());
        self.set_multi_target_attacks(false);
        self.set_attack_assist(false);
        self.set_behavior_profiles(Default::default());
        self.set_reinforcement_limit(0);
        self.set_reinforcement_call(Default::default());
        self.set_sight(preset.sight);
        self.settings.cutscene_speed = preset.cutscenes;
        self.debug.speed = 100;
        self.set_god_mode(preset.practice)?;
        self.set_stealth(preset.practice)?;
        self.debug.click_teleport = preset.practice;
        self.debug.click_kill = false;
        if self.debug.armor {
            self.apply_item(Item::Armor, 1)?;
        }
        Ok(())
    }

    pub(super) fn apply_item(&mut self, item: Item, direction: i32) -> Result<()> {
        if let Some(reason) = settings::runtime::Runtime::Gaisen.unavailable(item) {
            return Err(reason.into());
        }
        if item == Item::FollowActor && !self.campaign.scene_visible() {
            return Err("Start a mission before following an actor".into());
        }
        if item.change_setting(&mut self.settings, direction) {
            if item == Item::CircleAction {
                self.set_circle_action(self.settings.circle_action);
            }
            return Ok(());
        }
        if self
            .debug
            .change_tool(&self.campaign.game, item, direction)?
        {
            if item == Item::SaveCamera {
                self.menu.notice = "Camera bookmark saved".into();
            }
            return Ok(());
        }
        match item {
            Item::Tenchu2Missions | Item::Tenchu2Diagnostics | Item::Tenchu2Launcher => {
                unreachable!("checked runtime availability")
            }
            Item::UseBandage => {
                self.state_tools_available()?;
                let state = self
                    .campaign
                    .game
                    .roster
                    .get_mut(self.campaign.game.player_id)
                    .bleeding
                    .as_mut()
                    .ok_or("Enable Player bleeding first")?;
                if !state.active {
                    return Err("No bleeding to treat".into());
                }
                if state.bandages == 0 {
                    return Err("No bandages left".into());
                }
                state.request_bandage();
            }
            Item::Bandages => {
                self.mission_edit()?;
                self.stop_edited_recording();
                let state = self
                    .campaign
                    .game
                    .roster
                    .get_mut(self.campaign.game.player_id)
                    .bleeding
                    .as_mut()
                    .ok_or("Enable Player bleeding first")?;
                state.bandages = (i32::from(state.bandages) + direction).clamp(0, 99) as u16;
            }
            Item::StateSlot => {
                self.debug.state_slot =
                    (i32::from(self.debug.state_slot) + direction).rem_euclid(5) as u8
            }
            Item::SaveState => self.state_action(true)?,
            Item::LoadState => self.state_action(false)?,
            Item::Rewind => self.toggle_rewind()?,
            Item::RecenterOrbit => self.recenter_orbit(),
            Item::Randomizer => self.toggle_randomizer(),
            Item::GodMode => self.set_god_mode(!self.debug.god_mode)?,
            Item::Stealth => self.set_stealth(!self.debug.stealth)?,
            Item::ClickTeleport => {
                self.mission_edit()?;
                self.debug.click_teleport = !self.debug.click_teleport;
                self.debug.click_kill = false;
                self.debug.pick_actor_position = false;
            }
            Item::ClickKill => {
                self.mission_edit()?;
                self.debug.click_kill = !self.debug.click_kill;
                self.debug.click_teleport = false;
                self.debug.teleport_armed = false;
                self.debug.pick_actor_position = false;
            }
            Item::Inventory(slot) => {
                let count = self
                    .debug
                    .inventory
                    .iter()
                    .find(|entry| entry.id == slot)
                    .ok_or("Invalid inventory slot")?
                    .count;
                let count = (i32::from(count) + direction).clamp(0, 255) as u8;
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
            Item::HeadRest => {
                let choices = tenchu_core::game::dismemberment::HeadRest::ALL;
                let current = choices
                    .iter()
                    .position(|s| *s == self.settings.dismemberment.head_rest)
                    .unwrap();
                let mut settings = self.settings.dismemberment;
                settings.head_rest =
                    choices[(current as i32 + direction).rem_euclid(choices.len() as i32) as usize];
                self.set_dismemberment(settings);
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
            Item::WeaponModel | Item::VoiceModel => {
                let (choices, selected) = if item == Item::WeaponModel {
                    (&self.menu.weapon_models, &mut self.settings.weapon_model)
                } else {
                    (&self.menu.voice_models, &mut self.settings.voice_model)
                };
                if !choices.is_empty() {
                    let index = choices
                        .iter()
                        .position(|(id, _)| id == selected)
                        .unwrap_or(0);
                    selected.clone_from(
                        &choices
                            [(index as i32 + direction).rem_euclid(choices.len() as i32) as usize]
                            .0,
                    );
                }
            }
            Item::MovesetModel => {
                let choices = &self.menu.moveset_models;
                if !choices.is_empty() {
                    let index = choices
                        .iter()
                        .position(|(id, _)| id == &self.settings.moveset_model)
                        .unwrap_or(0);
                    self.settings.moveset_model = choices
                        [(index as i32 + direction).rem_euclid(choices.len() as i32) as usize]
                        .0
                        .clone();
                    self.settings.model_moveset = true;
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
            Item::ShadowPreview => {
                if !self.campaign.playing() {
                    return Err("Start a mission to preview model shadows".into());
                }
                self.settings.shadow_preview();
                self.apply_item(Item::PlayerLight, 1)?;
                self.menu.notice = "Preview ready: close F1; move or attack".into();
            }

            Item::FocusLight => {
                let game = &self.campaign.game;
                let source = self
                    .view
                    .as_ref()
                    .and_then(|v| v.nearest_lamp(game, self.debug.eye(game)))
                    .ok_or("No lantern emitters found in this mission")?;
                self.debug.focus_light(game, &source)?;
                self.settings.graphics.lighting.markers = true;
                self.menu.notice = format!(
                    "{} at {:.0?}",
                    source.name,
                    source.light.position.to_array()
                );
            }
            Item::Reset => {
                self.set_god_mode(false)?;
                self.set_stealth(false)?;
                self.keep_enemies_active(false);
                self.set_independent_attacks(false);
                self.set_quick_shuriken_dodge(false);
                self.set_strict_corpse_sight(false);
                self.set_guard_controls(false);
                self.set_circle_action(Default::default());
                self.set_walking(false)?;
                self.set_sheathing(Default::default())?;
                self.set_bleeding(Default::default());
                self.set_dismemberment(Default::default());
                self.set_damage_escalation(Default::default());
                self.set_multi_target_attacks(false);
                self.set_attack_assist(false);
                self.set_behavior_profiles(Default::default());
                if self.debug.sight_percent != 100 {
                    self.stop_edited_recording();
                }
                self.settings = Settings::default();
                self.settings
                    .apply_activation(&mut self.campaign.game.roster);
                self.campaign.game.intelligence.sight_percent = 100;
                self.debug = crate::debug::Debug::new(&self.campaign.game)?;
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
            Item::Teleport => self.teleport()?,
            Item::PickTeleport => {
                if !self.campaign.playing() {
                    return Err("Start a mission before teleporting".into());
                }
                self.debug.free_camera(&self.campaign.game);
                self.debug.teleport_armed = true;
                self.debug.click_kill = false;
                self.debug.pick_actor_position = false;
                self.menu.open = false;
            }
            Item::SpawnActor
            | Item::MoveActor
            | Item::RemoveActor
            | Item::KillActor
            | Item::ActorPosition
            | Item::PickActorPosition => self.edit_actor(item)?,
            Item::Pause => {
                if !self.simulating() || self.faulted {
                    return Err("Simulation is not running".into());
                }
                self.paused = !self.paused;
                self.debug.pause_view = true;
                self.menu.open = false;
            }
            Item::Step => {
                if !self.simulating() || self.faulted {
                    return Err("Simulation is not running".into());
                }
                self.paused = true;
                self.debug.pause_view = true;
                self.debug.frame_step = true;
                self.menu.open = false;
            }
            Item::SightDistance => {
                let value =
                    (i32::from(self.debug.sight_percent) + direction * 25).clamp(0, 400) as u16;
                self.set_sight(value);
            }
            Item::KeepEnemiesActive => self.keep_enemies_active(!self.settings.keep_enemies_active),
            Item::IndependentAttacks => {
                self.set_independent_attacks(!self.settings.independent_attacks);
            }
            Item::BehaviorProfiles
            | Item::BehaviorCombat
            | Item::BehaviorIdle
            | Item::BehaviorQuickTurns
            | Item::BehaviorProgression
            | Item::BehaviorSeed
            | Item::ActorCombat
            | Item::ActorIdle => self.edit_behavior(item, direction, false)?,
            Item::AttackAssist => self.set_attack_assist(!self.settings.attack_assist),
            Item::MultiTargetAttacks => {
                self.set_multi_target_attacks(!self.settings.multi_target_attacks);
            }
            Item::StrictCorpseSight => {
                self.set_strict_corpse_sight(!self.settings.strict_corpse_sight);
            }
            Item::Sheathing
            | Item::SheathedOneHit
            | Item::StaySheathed
            | Item::UnarmedAttacks
            | Item::UnarmedStyle => {
                let mut settings = self.settings.sheathing;
                if item == Item::Sheathing {
                    settings.enabled = !settings.enabled;
                } else if item == Item::StaySheathed {
                    settings.stay_sheathed = !settings.stay_sheathed;
                } else if item == Item::UnarmedAttacks {
                    settings.unarmed = !settings.unarmed;
                } else if item == Item::UnarmedStyle {
                    use tenchu_core::game::sheathing::UnarmedStyle;
                    settings.unarmed_style = match settings.unarmed_style {
                        UnarmedStyle::Onikage => {
                            crate::sheathing::imports::load()?;
                            UnarmedStyle::Tatsumaru
                        }
                        UnarmedStyle::Tatsumaru => UnarmedStyle::Onikage,
                    };
                } else {
                    settings.one_hit = !settings.one_hit;
                }
                self.set_sheathing(settings)?;
            }
            Item::SheathedSpeed | Item::SheathedVisibility | Item::SheathedDamage => {
                let mut settings = self.settings.sheathing;
                let field = match item {
                    Item::SheathedSpeed => &mut settings.speed,
                    Item::SheathedVisibility => &mut settings.visibility,
                    _ => &mut settings.damage,
                };
                let slider = item.slider().unwrap();
                *field = (i32::from(*field) + direction * slider.step).clamp(slider.min, slider.max)
                    as u16;
                self.set_sheathing(settings)?;
            }
            Item::AnalogWalking => self.set_walking(!self.settings.walking.enabled)?,
            Item::GuardDraw => {
                self.stop_edited_recording();
                self.settings.guard_draw = !self.settings.guard_draw;
                self.campaign.game.intelligence.guard_draw = self.settings.guard_draw;
            }
            Item::GuardControls => {
                self.set_guard_controls(!self.settings.guard_controls);
            }
            Item::QuickShurikenDodge => {
                self.set_quick_shuriken_dodge(!self.settings.quick_shuriken_dodge);
            }
            Item::DetectionDamage => {
                let mut settings = self.settings.damage_escalation;
                settings.increment =
                    (i32::from(settings.increment) + direction * 10).clamp(0, 100) as u16;
                self.set_damage_escalation(settings);
            }
            Item::DetectionDamageLimit => {
                let mut settings = self.settings.damage_escalation;
                settings.maximum =
                    (i32::from(settings.maximum) + direction * 25).clamp(0, 400) as u16;
                self.set_damage_escalation(settings);
            }
            Item::ReinforcementsPerCall => {
                let mut call = self.settings.reinforcement_call;
                call.count = (i32::from(call.count) + direction).clamp(
                    0,
                    i32::from(tenchu_core::roster::ReinforcementCall::MAX_COUNT),
                ) as u16;
                self.set_reinforcement_call(call);
            }
            Item::RandomReinforcements => {
                let mut call = self.settings.reinforcement_call;
                call.random = !call.random;
                self.set_reinforcement_call(call);
            }
            Item::WitnessedKillReinforcements => {
                let mut call = self.settings.reinforcement_call;
                call.witnessed_kills = !call.witnessed_kills;
                self.set_reinforcement_call(call);
            }
            Item::ReinforcementLimit => {
                let value = (i32::from(self.settings.reinforcement_limit) + direction * 10)
                    .clamp(0, 1000) as u16;
                self.set_reinforcement_limit(value);
            }
            Item::Screenshot => {
                self.screenshot = true;
                self.menu.open = false;
            }
            Item::ResetTweaks => {
                self.set_god_mode(false)?;
                self.set_stealth(false)?;
                self.debug.click_teleport = false;
                self.debug.click_kill = false;
                self.debug.speed = 100;
                self.set_sight(100);
                self.keep_enemies_active(false);
                self.set_independent_attacks(false);
                self.set_quick_shuriken_dodge(false);
                self.set_strict_corpse_sight(false);
                self.set_guard_controls(false);
                self.set_circle_action(Default::default());
                self.set_walking(false)?;
                self.set_sheathing(Default::default())?;
                self.set_bleeding(Default::default());
                self.set_dismemberment(Default::default());
                self.set_damage_escalation(Default::default());
                self.set_multi_target_attacks(false);
                self.set_attack_assist(false);
                self.set_behavior_profiles(Default::default());
                self.set_reinforcement_limit(0);
                self.set_reinforcement_call(Default::default());
            }
            _ => return Err("This control has no editable value".into()),
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn save_screenshot(&mut self) {
        self.screenshot = false;
        let result = (|| -> Result<_> {
            let prefix = if let Some(demo) = &self.demo {
                format!("demo-{}-{:05}", demo.clip().id, demo.frame)
            } else {
                format!("tenchu-{}", self.campaign.game.tick)
            };
            let path = crate::capture::screenshot_path(&prefix)?;
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
