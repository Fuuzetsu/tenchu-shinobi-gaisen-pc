use crate::{
    campaign::Action as MenuAction,
    controls::{self, Controls},
    settings::{
        self, Settings,
        library::Manager,
        menu::{Item, Menu as SharedMenu},
        overlay::{self, Edit},
        runtime::Runtime,
    },
};

#[cfg(test)]
mod gameplay_tests;

pub fn graphics(settings: &Settings) -> crate::settings::Graphics {
    let settings = settings.rendering();
    let g = &settings.graphics;
    crate::settings::Graphics {
        fog: g.fog,
        fog_distance: g.fog_distance,
        distant_doors: g.distant_doors,
        textures: g.textures,
        shading: g.shading,
        exposure: g.exposure,
        smooth_models: g.smooth_models,
        model_detail: g.model_detail,
        models: g.models.clone(),
        blood: g.blood,
        blob_shadows: g.blob_shadows,
        weapon_reflections: g.weapon_reflections,
        motion: g.motion.clone(),
        interpolation: g.interpolation,
        bilinear: g.bilinear,
        mipmaps: g.mipmaps,
        anisotropy: g.anisotropy,
        magnification: g.magnification,
        surface_detail: g.surface_detail,
        surface_strength: g.surface_strength,
        asset_pack: g.asset_pack,
        upscale: g.upscale,
        samples: g.samples,
        post: g.post.clone(),
        weather: g.weather.clone(),
        water: g.water.clone(),
        effects: crate::settings::Effects {
            sparks: g.effects.sparks,
            gunfire: g.effects.gunfire,
            splashes: g.effects.splashes,
            spark_density: g.effects.spark_density,
            leaves: g.effects.leaves,
            leaf_density: g.effects.leaf_density,
            scatter: g.effects.scatter,
            ..Default::default()
        },
        sky: g.sky.clone(),
        lighting: g.lighting.clone(),
        ..Default::default()
    }
}

#[derive(Default)]
pub struct Menu {
    pub panel: SharedMenu,
    pub diagnostics: bool,
    pub fast_forward: bool,
    pub screenshot: bool,
    pub actor_assets_changed: bool,
    pub tools: super::tools::Tools,
    pending: Option<Navigation>,
}

enum Navigation {
    Action(MenuAction),
    Page(i32),
    Reset,
}

pub struct State<'a> {
    pub settings: &'a mut Settings,
    pub paused: &'a mut bool,
    pub controls: &'a mut Controls,
    pub presets: &'a mut Manager,
    pub lights: usize,
    pub samples: &'a [u32],
    pub models: Option<crate::visuals::appearance::picker::State<'a>>,
}

#[derive(Default)]
pub struct Action {
    pub spawn: Option<super::spawning::Request>,
    pub leave: bool,
    pub select_mission: bool,
    pub step: bool,
    pub clear_weather: bool,
    pub clear_blood: bool,
    pub clear_footprints: bool,
    pub models: Option<std::sync::Arc<crate::visuals::pack::Pack>>,
    pub reload_artwork: bool,
    pub save_diagnostics: bool,
    pub state: Option<bool>,
    pub rewind: Option<super::rewind::Action>,
}

impl Menu {
    pub fn notice(&mut self, message: String) {
        self.panel.notice = message;
        self.tools.debug.notice = Some((self.panel.notice.clone(), web_time::Instant::now()));
    }

    pub fn state_result(&mut self, save: bool, result: Result<(), String>) {
        self.panel.notice = match result {
            Ok(()) => format!(
                "{} T2 state {}{}",
                if save { "Saved" } else { "Loaded" },
                self.tools.debug.state_slot + 1,
                if save { "" } else { " · F3 to resume" }
            ),
            Err(error) => error,
        };
        self.tools.debug.notice = Some((self.panel.notice.clone(), web_time::Instant::now()));
    }

    pub fn loaded_state(&mut self, controls: &mut Controls, presets: &mut Manager) {
        self.panel.open = false;
        self.panel.model_picker = false;
        self.panel.touch_controls = false;
        self.panel.edit = None;
        self.diagnostics = false;
        self.fast_forward = false;
        self.actor_assets_changed = false;
        self.screenshot = false;
        self.pending = None;
        controls.clear();
        controls.stop_rumble();
        controls.close_editor();
        presets.open = false;
    }

    pub fn apply_spawn(
        &mut self,
        playback: &mut super::campaign::Playback<impl tenchu_assets::ReadAt>,
        request: super::spawning::Request,
        library: Option<&crate::visuals::appearance::library::Library>,
    ) {
        self.panel.notice = match playback.spawn_actor(request, library) {
            Ok(id) => {
                self.actor_assets_changed = true;
                self.tools.sync(&playback.content, playback.generation);
                self.tools.debug.actors_edited = true;
                self.tools.debug.selected_actor = id;
                format!("Placed actor #{id}. Retry restores the original layout.")
            }
            Err(error) => error,
        };
        self.tools.debug.notice = Some((self.panel.notice.clone(), web_time::Instant::now()));
    }

    pub fn request_screenshot(&mut self) {
        self.screenshot = true;
        self.panel.open = false;
    }

    pub fn screenshot_result(&mut self, result: Result<(), String>) {
        self.panel.notice = match result {
            Ok(()) => "Screenshot saved; path in terminal".into(),
            Err(error) => {
                eprintln!("Screenshot: {error}");
                format!("Screenshot: {error}")
            }
        };
        self.tools.debug.notice = Some((self.panel.notice.clone(), web_time::Instant::now()));
    }

    pub fn new(settings: &Settings) -> Self {
        let mut menu = Self::default();
        menu.panel
            .model_directory
            .clone_from(&settings.model_directory);
        menu
    }

    fn reload_models(
        &mut self,
        settings: &mut Settings,
        action: &mut Action,
    ) -> Result<(), String> {
        let pack = crate::visuals::pack::Pack::bodies(&self.panel.model_directory)?;
        if let Some(id) = settings.player_model.strip_prefix("pack:")
            && !pack.player_models.contains_key(id)
        {
            settings.player_model.clear();
        }
        settings
            .model_directory
            .clone_from(&self.panel.model_directory);
        self.panel.notice = format!("{} folder models loaded", pack.player_models.len());
        action.models = Some(pack);
        Ok(())
    }

    pub fn apply_tools(&mut self, content: &mut super::campaign::Content, size: [u32; 2]) {
        let previous_notice = self.tools.debug.notice.as_ref().map(|(_, at)| *at);
        if let Err(error) = self.tools.apply(content, size) {
            self.panel.notice = error.clone();
            self.tools.debug.notice = Some((error, web_time::Instant::now()));
        }
        if let Some((message, at)) = &self.tools.debug.notice
            && Some(*at) != previous_notice
        {
            self.panel.notice.clone_from(message);
        }
        if std::mem::take(&mut self.tools.open_actor_editor) {
            self.panel.open = true;
            self.panel.page(4 - self.panel.page as i32);
            self.panel.reveal_row = false;
        }
    }
    pub fn cutscene_controls(&mut self, ctx: &egui::Context, cinematic: bool) {
        if !cinematic {
            self.fast_forward = false;
            return;
        }
        if !self.panel.open {
            egui::Area::new("jp-fast-forward".into())
                .anchor(egui::Align2::RIGHT_TOP, [-12.0, 42.0])
                .show(ctx, |ui| {
                    ui.spacing_mut().interact_size.y = 36.0;
                    ui.toggle_value(&mut self.fast_forward, "Fast-forward · Backspace");
                });
        }
    }

    pub fn back(&mut self, controls: &mut Controls, presets: &mut Manager) {
        if controls.editor.open {
            controls.close_editor();
        } else if presets.open {
            presets.open = false;
        } else if self.panel.touch_controls {
            self.panel.touch_controls = false;
        } else if self.panel.model_picker {
            self.panel.model_picker = false;
        } else {
            self.panel.open = false;
            self.diagnostics = false;
        }
        controls.clear();
    }

    pub fn controller(&mut self, pressed: u16, circle_confirm: bool) {
        if !self.panel.open {
            return;
        }
        use tenchu_core::input;
        self.pending = if pressed & input::L1 != 0 {
            Some(Navigation::Page(-1))
        } else if pressed & input::R1 != 0 {
            Some(Navigation::Page(1))
        } else {
            match controls::gamepad_command(pressed, false, false, circle_confirm) {
                Some(controls::Command::Menu(MenuAction::Replay)) => Some(Navigation::Reset),
                Some(controls::Command::Menu(action)) => Some(Navigation::Action(action)),
                _ => None,
            }
        };
    }

    fn navigate(&mut self, navigation: Navigation, state: &mut State<'_>) -> Option<Edit> {
        if matches!(navigation, Navigation::Action(MenuAction::Back)) {
            self.back(state.controls, state.presets);
            return None;
        }
        if state.controls.editor.open || state.presets.open || self.panel.touch_controls {
            return None;
        }
        let rows = settings::menu::rows(
            self.panel.page,
            state.settings,
            &self.tools.debug,
            *state.paused,
        );
        self.panel.row = self.panel.row.min(rows.len() - 1);
        match navigation {
            Navigation::Page(delta) => self.panel.page(delta),
            Navigation::Reset => {
                return Some(Edit::Reset(
                    self.panel.hovered.unwrap_or(rows[self.panel.row].item),
                ));
            }
            Navigation::Action(action) => {
                let item = rows[self.panel.row].item;
                if let Some(delta) = self.panel.action(action, rows.len())
                    && (!item.button() || matches!(action, MenuAction::Confirm | MenuAction::Start))
                {
                    return Some(Edit::Change(item, delta));
                }
            }
        }
        None
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut State<'_>) -> Action {
        let mut action = Action::default();
        self.tools.hint(ctx, self.panel.open);
        if !self.panel.open {
            for edit in overlay::show(
                ctx,
                &mut self.panel,
                overlay::State {
                    runtime: Runtime::Tenchu2,
                    settings: state.settings,
                    debug: &self.tools.debug,
                    paused: *state.paused,
                    lights: state.lights,
                    samples: state.samples,
                },
                None,
                None,
            ) {
                self.apply(edit, state, &mut action);
            }
            return action;
        }
        self.panel.actor_models.clone_from(&self.tools.actor_models);
        if self.panel.artwork_status.is_empty() {
            self.panel.artwork_status = crate::visuals::pack::configured_tenchu2_root().map_or_else(
                || "Install tenchu2-hd.tar.zst beside the game, or import it in the Android launcher. The Gaisen pack does not contain Tenchu 2 artwork.".into(),
                |path| format!("Tenchu 2 artwork: {}", path.display()),
            );
        }
        if let Some(models) = &state.models {
            self.panel.player_models =
                crate::visuals::appearance::choices(models.game, models.bodies, models.library);
            self.panel.moveset_models = vec![(String::new(), "Selected player body".into())];
            if let Some(library) = models.library {
                self.panel.weapon_models = library.equipment_choices();
                self.panel.voice_models = library.voice_choices();
                self.panel.moveset_models.extend(
                    library
                        .bodies
                        .iter()
                        .filter(|b| super::models::compatible_moveset(b))
                        .map(|b| (b.id.clone(), b.name.clone())),
                );
            }
        }
        let mut navigation = self.pending.take();
        if !ctx.wants_keyboard_input()
            && !state.controls.editor.open
            && !state.presets.open
            && !self.panel.touch_controls
        {
            ctx.input_mut(|input| {
                if input.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab) {
                    navigation = Some(Navigation::Page(-1));
                } else if input.consume_key(egui::Modifiers::NONE, egui::Key::Tab) {
                    navigation = Some(Navigation::Page(1));
                }
                for (key, command) in [
                    (egui::Key::PageUp, Navigation::Page(-1)),
                    (egui::Key::PageDown, Navigation::Page(1)),
                    (egui::Key::R, Navigation::Reset),
                    (egui::Key::ArrowUp, Navigation::Action(MenuAction::Up)),
                    (egui::Key::ArrowDown, Navigation::Action(MenuAction::Down)),
                    (egui::Key::ArrowLeft, Navigation::Action(MenuAction::Left)),
                    (egui::Key::ArrowRight, Navigation::Action(MenuAction::Right)),
                    (egui::Key::Enter, Navigation::Action(MenuAction::Confirm)),
                    (egui::Key::Space, Navigation::Action(MenuAction::Confirm)),
                ] {
                    if input.consume_key(egui::Modifiers::NONE, key) {
                        navigation = Some(command);
                    }
                }
            });
        }
        if let Some(navigation) = navigation
            && let Some(edit) = self.navigate(navigation, state)
        {
            self.apply(edit, state, &mut action);
        }
        let edits = overlay::show(
            ctx,
            &mut self.panel,
            overlay::State {
                runtime: Runtime::Tenchu2,
                settings: state.settings,
                debug: &self.tools.debug,
                paused: *state.paused,
                lights: state.lights,
                samples: state.samples,
            },
            Some(&state.presets.library),
            state.models.as_mut(),
        );
        for edit in edits {
            self.apply(edit, state, &mut action);
        }
        if let Some(choice) = state.presets.show(ctx, state.settings) {
            self.apply(Edit::Preset(choice), state, &mut action);
        }
        state.controls.show_tenchu2_bindings(ctx);
        action
    }

    fn gameplay_preset(
        &mut self,
        index: usize,
        state: &mut State<'_>,
    ) -> tenchu_assets::Result<()> {
        let preset = settings::gameplay::PRESETS
            .get(index)
            .ok_or("Unknown gameplay preset")?;
        self.tools.gameplay_preset(preset)?;
        let defaults = Settings::default();
        let settings = &mut state.settings;
        settings.keep_enemies_active = preset.active;
        settings.independent_attacks = defaults.independent_attacks;
        settings.quick_shuriken_dodge = defaults.quick_shuriken_dodge;
        settings.strict_corpse_sight = defaults.strict_corpse_sight;
        settings.guard_controls = defaults.guard_controls;
        settings.circle_action = defaults.circle_action;
        settings.walking = defaults.walking;
        settings.sheathing = defaults.sheathing;
        settings.bleeding = defaults.bleeding;
        settings.dismemberment = defaults.dismemberment;
        settings.damage_escalation = defaults.damage_escalation;
        settings.multi_target_attacks = defaults.multi_target_attacks;
        settings.attack_assist = defaults.attack_assist;
        settings.behavior_profiles = defaults.behavior_profiles;
        settings.reinforcement_limit = defaults.reinforcement_limit;
        settings.reinforcement_call = defaults.reinforcement_call;
        settings.sight_percent = preset.sight;
        settings.cutscene_speed = preset.cutscenes;
        self.fast_forward = false;
        state.controls.clear();
        Ok(())
    }

    fn apply(&mut self, edit: Edit, state: &mut State<'_>, action: &mut Action) {
        self.panel.notice.clear();
        if let Some(reason) = Runtime::Tenchu2.unavailable_edit(&edit) {
            self.panel.notice = reason.into();
            return;
        }
        let result = match edit {
            Edit::SeekRewind(tick) => {
                action.rewind = Some(super::rewind::Action::Seek(tick));
                Ok(())
            }
            Edit::ResumeRewind => {
                action.rewind = Some(super::rewind::Action::Resume);
                Ok(())
            }
            Edit::PlayRewindForward => {
                action.rewind = Some(super::rewind::Action::Forward);
                Ok(())
            }
            Edit::SelectRewind(range) => {
                action.rewind = Some(super::rewind::Action::Select(range));
                Ok(())
            }
            Edit::ExportRewind => {
                action.rewind = Some(super::rewind::Action::Export);
                Ok(())
            }
            Edit::ImportRewind => {
                action.rewind = Some(super::rewind::Action::Import);
                Ok(())
            }
            Edit::RecordedVisuals(value) => {
                action.rewind = Some(super::rewind::Action::RecordedVisuals(value));
                Ok(())
            }
            Edit::Diagnostics => {
                action.save_diagnostics = true;
                Ok(())
            }
            Edit::PlayerModel(id) => {
                state.settings.player_model = id;
                Ok(())
            }
            Edit::WeaponModel(id) => {
                state.settings.weapon_model = id;
                Ok(())
            }
            Edit::VoiceModel(id) => {
                state.settings.voice_model = id;
                Ok(())
            }
            Edit::MovesetModel(id) => {
                state.settings.moveset_model = id;
                state.settings.model_moveset = true;
                Ok(())
            }
            Edit::Touch(value) => {
                state.settings.touch = value;
                Ok(())
            }
            Edit::Coordinate(axis, value) => {
                if axis < 3 {
                    self.tools.debug.target[axis] = value.clamp(-320_000, 320_000);
                    Ok(())
                } else {
                    Err("Invalid coordinate".into())
                }
            }
            Edit::Preset(choice) => state.presets.library.apply(choice, state.settings),
            Edit::GameplayPreset(index) => self.gameplay_preset(index, state),
            Edit::PresetLibrary => {
                state.presets.open = true;
                Ok(())
            }
            Edit::Controls => {
                state.controls.editor.open = true;
                state.controls.clear();
                Ok(())
            }
            Edit::Reset(item) => {
                if item.reset_setting(state.settings) {
                    Ok(())
                } else {
                    match item {
                        Item::GameplayPreset => self.gameplay_preset(0, state),
                        Item::DetectionDamage | Item::DetectionDamageLimit => {
                            let defaults = Settings::default().damage_escalation;
                            if item == Item::DetectionDamage {
                                state.settings.damage_escalation.increment = defaults.increment;
                            } else {
                                state.settings.damage_escalation.maximum = defaults.maximum;
                            }
                            Ok(())
                        }
                        Item::MultiTargetAttacks | Item::AttackAssist => {
                            if item == Item::MultiTargetAttacks {
                                state.settings.multi_target_attacks =
                                    Settings::default().multi_target_attacks;
                            } else {
                                state.settings.attack_assist = Settings::default().attack_assist;
                            }
                            Ok(())
                        }
                        Item::QuickShurikenDodge | Item::StrictCorpseSight => {
                            if item == Item::QuickShurikenDodge {
                                state.settings.quick_shuriken_dodge =
                                    Settings::default().quick_shuriken_dodge;
                            } else {
                                state.settings.strict_corpse_sight =
                                    Settings::default().strict_corpse_sight;
                            }
                            Ok(())
                        }
                        Item::HeadRest => {
                            state.settings.dismemberment.head_rest =
                                Settings::default().dismemberment.head_rest;
                            Ok(())
                        }
                        Item::Sheathing => {
                            state.settings.sheathing.enabled =
                                Settings::default().sheathing.enabled;
                            Ok(())
                        }
                        Item::UnarmedAttacks => {
                            state.settings.sheathing.unarmed =
                                Settings::default().sheathing.unarmed;
                            Ok(())
                        }
                        Item::UnarmedStyle => {
                            state.settings.sheathing.unarmed_style =
                                Settings::default().sheathing.unarmed_style;
                            Ok(())
                        }
                        Item::SheathedOneHit => {
                            state.settings.sheathing.one_hit =
                                Settings::default().sheathing.one_hit;
                            Ok(())
                        }
                        Item::SheathedSpeed | Item::SheathedVisibility | Item::SheathedDamage => {
                            let defaults = Settings::default().sheathing;
                            match item {
                                Item::SheathedSpeed => {
                                    state.settings.sheathing.speed = defaults.speed
                                }
                                Item::SheathedVisibility => {
                                    state.settings.sheathing.visibility = defaults.visibility
                                }
                                _ => state.settings.sheathing.damage = defaults.damage,
                            }
                            Ok(())
                        }
                        Item::AnalogWalking => {
                            state.settings.walking.enabled = Settings::default().walking.enabled;
                            Ok(())
                        }
                        Item::StateSlot => {
                            self.tools.debug.state_slot = 0;
                            Ok(())
                        }
                        Item::KeepEnemiesActive => {
                            state.settings.keep_enemies_active =
                                Settings::default().keep_enemies_active;
                            Ok(())
                        }
                        Item::Preset => {
                            state.settings.set_visual_preset(0);
                            Ok(())
                        }
                        Item::Antialiasing => {
                            state.settings.graphics.samples = Settings::default().graphics.samples;
                            Ok(())
                        }
                        _ => self.tools.reset(item),
                    }
                }
            }
            Edit::Change(item, delta) => {
                if item.change_setting(state.settings, delta) {
                    Ok(())
                } else {
                    match item {
                        Item::GameplayPreset => {
                            let count = settings::gameplay::PRESETS.len();
                            let current =
                                settings::gameplay::current(state.settings, &self.tools.debug);
                            let index = current.map_or_else(
                                || if delta < 0 { count - 1 } else { 0 },
                                |i| (i as i32 + delta).rem_euclid(count as i32) as usize,
                            );
                            self.gameplay_preset(index, state)
                        }
                        Item::DetectionDamage | Item::DetectionDamageLimit => {
                            let (field, step, maximum) = if item == Item::DetectionDamage {
                                (&mut state.settings.damage_escalation.increment, 10, 100)
                            } else {
                                (&mut state.settings.damage_escalation.maximum, 25, 400)
                            };
                            *field = (i32::from(*field) + delta * step).clamp(0, maximum) as u16;
                            Ok(())
                        }
                        Item::MultiTargetAttacks | Item::AttackAssist => {
                            let field = if item == Item::MultiTargetAttacks {
                                &mut state.settings.multi_target_attacks
                            } else {
                                &mut state.settings.attack_assist
                            };
                            *field = !*field;
                            Ok(())
                        }
                        Item::QuickShurikenDodge | Item::StrictCorpseSight => {
                            let field = if item == Item::QuickShurikenDodge {
                                &mut state.settings.quick_shuriken_dodge
                            } else {
                                &mut state.settings.strict_corpse_sight
                            };
                            *field = !*field;
                            Ok(())
                        }
                        Item::HeadRest => {
                            let choices = tenchu_core::game::dismemberment::HeadRest::ALL;
                            let at = choices
                                .iter()
                                .position(|&p| p == state.settings.dismemberment.head_rest)
                                .unwrap();
                            state.settings.dismemberment.head_rest = choices
                                [(at as i32 + delta).rem_euclid(choices.len() as i32) as usize];
                            Ok(())
                        }
                        Item::Sheathing => {
                            state.settings.sheathing.enabled = !state.settings.sheathing.enabled;
                            Ok(())
                        }
                        Item::UnarmedAttacks => {
                            state.settings.sheathing.unarmed = !state.settings.sheathing.unarmed;
                            Ok(())
                        }
                        Item::UnarmedStyle => {
                            use tenchu_core::game::sheathing::UnarmedStyle;
                            state.settings.sheathing.unarmed_style =
                                match state.settings.sheathing.unarmed_style {
                                    UnarmedStyle::Onikage => UnarmedStyle::Tatsumaru,
                                    UnarmedStyle::Tatsumaru => UnarmedStyle::Onikage,
                                };
                            Ok(())
                        }
                        Item::SheathedOneHit => {
                            state.settings.sheathing.one_hit = !state.settings.sheathing.one_hit;
                            Ok(())
                        }
                        Item::SheathedSpeed | Item::SheathedVisibility | Item::SheathedDamage => {
                            let field = match item {
                                Item::SheathedSpeed => &mut state.settings.sheathing.speed,
                                Item::SheathedVisibility => {
                                    &mut state.settings.sheathing.visibility
                                }
                                _ => &mut state.settings.sheathing.damage,
                            };
                            let slider = item.slider().unwrap();
                            *field = (i32::from(*field) + delta * slider.step)
                                .clamp(slider.min, slider.max)
                                as u16;
                            Ok(())
                        }
                        Item::AnalogWalking => {
                            state.settings.walking.enabled = !state.settings.walking.enabled;
                            Ok(())
                        }
                        Item::KeepEnemiesActive => {
                            state.settings.keep_enemies_active =
                                !state.settings.keep_enemies_active;
                            Ok(())
                        }
                        Item::Reset | Item::ResetTweaks => {
                            let defaults = Settings::default();
                            let all = item == Item::Reset;
                            if all {
                                *state.settings = defaults;
                                self.panel.model_directory.clear();
                                self.fast_forward = false;
                            } else {
                                state.settings.keep_enemies_active = defaults.keep_enemies_active;
                                state.settings.circle_action = defaults.circle_action;
                                state.settings.walking = defaults.walking;
                                state.settings.sheathing = defaults.sheathing;
                                state.settings.dismemberment = defaults.dismemberment;
                                state.settings.bleeding = defaults.bleeding;
                                state.settings.sight_percent = defaults.sight_percent;
                                state.settings.quick_shuriken_dodge = defaults.quick_shuriken_dodge;
                                state.settings.strict_corpse_sight = defaults.strict_corpse_sight;
                                state.settings.multi_target_attacks = defaults.multi_target_attacks;
                                state.settings.attack_assist = defaults.attack_assist;
                                state.settings.damage_escalation = defaults.damage_escalation;
                            }
                            self.tools.restore_defaults(all);
                            state.controls.clear();
                            Ok(())
                        }
                        Item::Preset => {
                            let choices = state.presets.library.choices();
                            let current = state.presets.library.current(state.settings);
                            let index = choices
                                .iter()
                                .position(|(choice, _)| Some(*choice) == current)
                                .map_or_else(
                                    || if delta < 0 { choices.len() - 1 } else { 0 },
                                    |index| {
                                        (index as i32 + delta).rem_euclid(choices.len() as i32)
                                            as usize
                                    },
                                );
                            state
                                .presets
                                .library
                                .apply(choices[index].0, state.settings)
                        }
                        Item::Antialiasing => {
                            if !state.samples.is_empty() {
                                let index = state
                                    .samples
                                    .iter()
                                    .position(|&n| n == state.settings.graphics.samples)
                                    .unwrap_or(0);
                                state.settings.graphics.samples = state.samples[(index as i32
                                    + delta)
                                    .rem_euclid(state.samples.len() as i32)
                                    as usize];
                            }
                            Ok(())
                        }
                        Item::PlayerModel => {
                            if state.models.is_some() {
                                self.panel.model_picker = true;
                                Ok(())
                            } else {
                                Err("Open through play with the Gaisen disc to load the cross-game model gallery.".into())
                            }
                        }
                        Item::WeaponModel | Item::VoiceModel => {
                            let (choices, selected) = if item == Item::WeaponModel {
                                (&self.panel.weapon_models, &mut state.settings.weapon_model)
                            } else {
                                (&self.panel.voice_models, &mut state.settings.voice_model)
                            };
                            if choices.is_empty() {
                                Err("Load the model gallery first.".into())
                            } else {
                                let index = choices
                                    .iter()
                                    .position(|(id, _)| id == selected)
                                    .unwrap_or(0);
                                selected.clone_from(&choices[(index as i32 + delta).rem_euclid(choices.len() as i32) as usize].0);
                                Ok(())
                            }
                        }
                        Item::MovesetModel => {
                            let choices = &self.panel.moveset_models;
                            if choices.is_empty() {
                                Err("Load the model gallery first.".into())
                            } else {
                                let index = choices
                                    .iter()
                                    .position(|(id, _)| *id == state.settings.moveset_model)
                                    .unwrap_or(0);
                                state.settings.moveset_model.clone_from(&choices[(index as i32 + delta).rem_euclid(choices.len() as i32) as usize].0);
                                state.settings.model_moveset = true;
                                Ok(())
                            }
                        }
                        Item::ReloadModels => self.reload_models(state.settings, action),
                        Item::ReloadAssets => {
                            action.reload_artwork = true;
                            Ok(())
                        }
                        Item::ClearBlood => {
                            action.clear_blood = true;
                            Ok(())
                        }
                        Item::ClearWeather => {
                            state.settings.graphics.weather.mode = settings::WeatherMode::Clear;
                            action.clear_weather = true;
                            Ok(())
                        }
                        Item::ClearFootprints => {
                            action.clear_footprints = true;
                            Ok(())
                        }
                        Item::FocusLight => self
                            .tools
                            .change(item, delta)
                            .map(|()| state.settings.graphics.lighting.markers = true),
                        Item::ShadowPreview => self.tools.change(Item::PlayerLight, 1).map(|()| {
                            state.settings.shadow_preview();
                            self.panel.notice = "Preview ready: close F1; move or attack".into();
                        }),
                        Item::Pause => {
                            *state.paused = !*state.paused;
                            self.panel.open = false;
                            Ok(())
                        }
                        Item::Step => {
                            *state.paused = true;
                            action.step = true;
                            self.panel.open = false;
                            Ok(())
                        }
                        Item::Screenshot => {
                            self.request_screenshot();
                            Ok(())
                        }
                        Item::StateSlot => {
                            self.tools.debug.state_slot =
                                (i32::from(self.tools.debug.state_slot) + delta.signum())
                                    .rem_euclid(5) as u8;
                            Ok(())
                        }
                        Item::Rewind => {
                            action.rewind = Some(super::rewind::Action::Toggle);
                            Ok(())
                        }
                        Item::SaveState | Item::LoadState => {
                            action.state = Some(item == Item::SaveState);
                            Ok(())
                        }
                        Item::SpawnActor => super::spawning::Source::selected(
                            &self.panel.spawn_model,
                        )
                        .map(|source| {
                            action.spawn = Some(super::spawning::Request {
                                source,
                                position: self.tools.debug.target,
                                yaw: (i32::from(self.panel.spawn_yaw) * 4096 / 360) as i16,
                                snap: self.tools.debug.snap_to_floor,
                            })
                        }),
                        Item::ActorPosition
                        | Item::PickActorPosition
                        | Item::MoveActor
                        | Item::RemoveActor => self
                            .tools
                            .edit_actor(item, &mut self.panel.spawn_yaw)
                            .map(|()| {
                                if item == Item::PickActorPosition {
                                    self.panel.open = false;
                                }
                            }),
                        Item::Tenchu2Missions => {
                            action.select_mission = true;
                            Ok(())
                        }
                        Item::Tenchu2Launcher => {
                            action.leave = true;
                            Ok(())
                        }
                        Item::Tenchu2Diagnostics => {
                            self.diagnostics = true;
                            self.panel.open = false;
                            Ok(())
                        }
                        _ => self.tools.change(item, delta).map(|()| {
                            if item == Item::PickTeleport {
                                self.panel.open = false;
                            }
                        }),
                    }
                }
            }
            _ => Err("This control is not connected to native Tenchu 2 JP yet.".into()),
        };
        if let Err(error) = result {
            self.panel.notice = error;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::Debug;

    fn presets() -> Manager {
        Manager::new(std::env::temp_dir().join(format!(
            "tenchu-native-menu-presets-{}.json",
            std::process::id()
        )))
    }

    #[test]
    #[ignore = "requires TENCHU2_DISC; native F1 sheathing edits, numeric clamps and resets reach the live player without altering weapon state"]
    fn native_sheathing_menu_edits_and_resets() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut playback =
            super::super::campaign::Playback::load(disc, 9, 2, 17, Some([0; 21]), None).unwrap();
        let mut menu = Menu::default();
        let mut settings = Settings::default();
        let mut paused = true;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1],
            models: None,
        };
        let mut action = Action::default();
        for (item, delta) in [
            (Item::Sheathing, 1),
            (Item::UnarmedAttacks, 1),
            (Item::UnarmedStyle, 1),
            (Item::SheathedSpeed, 100),
            (Item::SheathedVisibility, -100),
            (Item::SheathedDamage, 100),
            (Item::SheathedOneHit, 1),
        ] {
            assert!(Runtime::Tenchu2.unavailable(item).is_none());
            menu.apply(Edit::Change(item, delta), &mut state, &mut action);
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
        }
        playback.configure(state.settings).unwrap();
        let super::super::campaign::Content::Gameplay(session) = &playback.content else {
            unreachable!()
        };
        let actual = session.world.actors[session.world.player]
            .actor
            .sheathing
            .unwrap()
            .settings;
        assert!(actual.enabled && actual.one_hit && actual.unarmed);
        assert_eq!(
            actual.unarmed_style,
            tenchu_core::game::sheathing::UnarmedStyle::Tatsumaru
        );
        assert!(
            session.world.actors[session.world.player]
                .actor
                .unarmed
                .is_some()
        );
        assert_eq!(
            (actual.speed, actual.visibility, actual.damage),
            (200, 25, 400)
        );
        let original = session.world.clone();
        for item in [
            Item::Sheathing,
            Item::UnarmedAttacks,
            Item::UnarmedStyle,
            Item::SheathedSpeed,
            Item::SheathedVisibility,
            Item::SheathedDamage,
            Item::SheathedOneHit,
        ] {
            menu.apply(Edit::Reset(item), &mut state, &mut action);
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
        }
        assert_eq!(state.settings.sheathing, Default::default());
        playback.configure(state.settings).unwrap();
        let super::super::campaign::Content::Gameplay(session) = &playback.content else {
            unreachable!()
        };
        let mut expected = original;
        expected.actors[expected.player].actor.sheathing = None;
        assert!(
            postcard::to_allocvec(&session.world).unwrap()
                == postcard::to_allocvec(&expected).unwrap()
        );
    }

    #[test]
    #[ignore = "requires TENCHU2_DISC; native F1 edits, individual resets and reset-tweaks reach gameplay AI"]
    fn native_ai_options_menu_configures_and_resets_source_preferences() {
        use super::super::campaign::{Content, Playback};
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut playback = Playback::load(disc, 9, 2, 17, Some([0; 21]), None).unwrap();
        let mut menu = Menu::default();
        let mut settings = Settings::default();
        let mut paused = true;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1],
            models: None,
        };
        let mut action = Action::default();
        for reset in [None, Some(Item::ResetTweaks), Some(Item::Reset)] {
            for item in [
                Item::QuickShurikenDodge,
                Item::StrictCorpseSight,
                Item::MultiTargetAttacks,
                Item::AttackAssist,
                Item::DetectionDamage,
                Item::DetectionDamageLimit,
            ] {
                assert!(Runtime::Tenchu2.unavailable(item).is_none());
                menu.apply(Edit::Change(item, 1), &mut state, &mut action);
                assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
            }
            playback.configure(state.settings).unwrap();
            let Content::Gameplay(session) = &playback.content else {
                unreachable!()
            };
            assert!(session.world.ai.quick_shuriken_dodge && session.world.ai.strict_corpse_sight);
            assert!(session.world.controls.multi_target_attacks);
            assert!(session.world.controls.attack_assist);
            assert_eq!(session.world.ai.damage_escalation.increment, 10);
            assert_eq!(session.world.ai.damage_escalation.maximum, 125);
            let mut expected = session.world.clone();
            expected.ai.quick_shuriken_dodge = false;
            expected.ai.strict_corpse_sight = false;
            expected.controls.multi_target_attacks = false;
            expected.controls.attack_assist = false;
            expected.set_damage_escalation(Default::default());
            if let Some(item) = reset {
                menu.apply(Edit::Change(item, 1), &mut state, &mut action);
            } else {
                for item in [
                    Item::QuickShurikenDodge,
                    Item::StrictCorpseSight,
                    Item::MultiTargetAttacks,
                    Item::AttackAssist,
                    Item::DetectionDamage,
                    Item::DetectionDamageLimit,
                ] {
                    menu.apply(Edit::Reset(item), &mut state, &mut action);
                }
            }
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
            playback.configure(state.settings).unwrap();
            let Content::Gameplay(session) = &playback.content else {
                unreachable!()
            };
            assert!(
                postcard::to_allocvec(&session.world).unwrap()
                    == postcard::to_allocvec(&expected).unwrap()
            );
        }
        assert!(
            Runtime::Tenchu2
                .unavailable(Item::IndependentAttacks)
                .unwrap()
                .contains("each fighter")
        );
    }

    #[test]
    fn native_dismemberment_menu_preserves_renderer_preferences() {
        let mut menu = Menu::default();
        let mut settings = Settings::default();
        let mut paused = true;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1],
            models: None,
        };
        let items = [
            Item::Dismemberment,
            Item::PlayerDismemberment,
            Item::HeadRest,
            Item::SaExecutionScene,
            Item::SeverStealth,
            Item::SeverDive,
            Item::SeverArms,
            Item::SeverSide,
            Item::PlayerSeverHead,
            Item::PlayerSeverArms,
            Item::PlayerSeverSide,
            Item::SeveredBlood,
            Item::SeveredStyle,
            Item::SeveredSeconds,
        ];
        let mut action = Action::default();
        for item in items {
            assert!(Runtime::Tenchu2.unavailable(item).is_none());
            let before = serde_json::to_value(&*state.settings).unwrap();
            menu.apply(Edit::Change(item, 1), &mut state, &mut action);
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
            assert_ne!(
                before,
                serde_json::to_value(&*state.settings).unwrap(),
                "menu edit had no effect"
            );
        }
        assert_eq!(
            graphics(state.settings).blood,
            state.settings.rendering().graphics.blood
        );
        state.settings.graphics.blood.model_stains = true;
        state.settings.graphics.blood.drips = crate::settings::BloodStyle::OriginalHd;
        state.settings.graphics.blood.weapons = true;
        state.settings.graphics.blood.death_gush = 175;
        assert_eq!(
            graphics(state.settings).blood,
            state.settings.rendering().graphics.blood
        );
        for item in items {
            menu.apply(Edit::Reset(item), &mut state, &mut action);
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
        }
        assert_eq!(state.settings.dismemberment, Default::default());
        assert_eq!(
            state.settings.graphics.blood.severed,
            Settings::default().graphics.blood.severed
        );
        assert_eq!(
            state.settings.graphics.blood.severed_style,
            Settings::default().graphics.blood.severed_style
        );
        assert_eq!(
            state.settings.graphics.blood.severed_seconds,
            Settings::default().graphics.blood.severed_seconds
        );
    }

    #[test]
    fn native_rewind_controls_and_closed_menu_timeline_are_connected() {
        let mut menu = Menu::default();
        let mut settings = Settings::default();
        let mut paused = true;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1],
            models: None,
        };
        let mut action = Action::default();
        menu.apply(
            Edit::Change(Item::RewindHistory, 1),
            &mut state,
            &mut action,
        );
        assert!(state.settings.rewind_seconds > 0);
        menu.apply(Edit::Change(Item::RewindSpeed, 1), &mut state, &mut action);
        assert_ne!(
            state.settings.rewind_speed,
            Settings::default().rewind_speed
        );
        for edit in [
            Edit::Change(Item::Rewind, 1),
            Edit::SeekRewind(3),
            Edit::ResumeRewind,
            Edit::PlayRewindForward,
            Edit::SelectRewind([2, 5]),
            Edit::ExportRewind,
            Edit::ImportRewind,
            Edit::RecordedVisuals(false),
        ] {
            action.rewind = None;
            menu.apply(edit, &mut state, &mut action);
            assert!(action.rewind.is_some());
            assert!(menu.panel.notice.is_empty());
        }
        menu.tools.debug.rewind = Some(crate::rewind::Timeline {
            start: 0,
            end: 5,
            tick: 2,
            playing: false,
            preview: true,
            selection: [0, 5],
            forward: false,
            recorded_visuals: Some(true),
        });
        let ctx = egui::Context::default();
        let input = |time| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            time: Some(time),
            ..Default::default()
        };
        let _ = ctx.run(input(0.0), |ctx| {
            menu.show(ctx, &mut state);
        });
        let output = ctx.run(input(0.1), |ctx| {
            menu.show(ctx, &mut state);
        });
        assert!(!menu.panel.open);
        fn has_timeline(shape: &egui::Shape) -> bool {
            match shape {
                egui::Shape::Text(text) => text.galley.job.text.contains("Rewind timeline"),
                egui::Shape::Vec(shapes) => shapes.iter().any(has_timeline),
                _ => false,
            }
        }
        assert!(output.shapes.iter().any(|shape| has_timeline(&shape.shape)));
    }

    #[test]
    fn native_savestate_menu_routes_controller_actions_and_clears_load_inputs() {
        use tenchu_core::input;
        for circle in [false, true] {
            let mut settings = Settings {
                circle_confirm: circle,
                ..Default::default()
            };
            let mut menu = Menu::new(&settings);
            menu.panel.open = true;
            let mut controls = Controls::default();
            let mut presets = presets();
            let mut paused = false;
            for item in [Item::StateSlot, Item::SaveState, Item::LoadState] {
                let (page, row) = (0..settings::menu::PAGES.len())
                    .find_map(|page| {
                        settings::menu::rows(page, &settings, &menu.tools.debug, paused)
                            .iter()
                            .position(|row| row.item == item)
                            .map(|row| (page, row))
                    })
                    .unwrap();
                menu.panel.page = page;
                menu.panel.row = row;
                menu.controller(if circle { input::CIRCLE } else { input::CROSS }, circle);
                let mut action = Action::default();
                let ctx = egui::Context::default();
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    action = menu.show(
                        ctx,
                        &mut State {
                            settings: &mut settings,
                            paused: &mut paused,
                            controls: &mut controls,
                            presets: &mut presets,
                            lights: 0,
                            samples: &[1],
                            models: None,
                        },
                    );
                });
                assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
                if item == Item::StateSlot {
                    assert_eq!(menu.tools.debug.state_slot, 1);
                } else {
                    assert_eq!(action.state, Some(item == Item::SaveState));
                }
            }
            controls.touch(crate::touch::Sample {
                buttons: u32::from(input::SQUARE),
                ..Default::default()
            });
            controls.editor.open = true;
            presets.open = true;
            menu.fast_forward = true;
            menu.panel.model_picker = true;
            menu.panel.touch_controls = true;
            menu.panel.edit = Some((Item::Target(0), "123".into()));
            menu.loaded_state(&mut controls, &mut presets);
            assert!(
                !menu.panel.open && !controls.editor.open && !presets.open && !menu.fast_forward
            );
            assert_eq!(controls.buttons(), 0);
            assert!(
                !menu.panel.model_picker && !menu.panel.touch_controls && menu.panel.edit.is_none()
            );
        }
    }

    #[test]
    #[ignore = "requires TENCHU2_DISC; F1 reset actions preserve live/dead mission state, inventory and source RNG"]
    fn native_reset_actions_restore_preferences_without_restarting_the_mission() {
        use super::super::{Session, campaign::Content};
        use tenchu_core::{input, tenchu2::actor::State as ActorState};
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        for all in [false, true] {
            for dead in [false, true] {
                let mut session = Session::load(&disc, 9, 1).unwrap();
                for _ in 0..60 {
                    session.step(0).unwrap();
                }
                session.world.controls.armor = true;
                let player = &mut session.world.actors[session.world.player].actor;
                player.health = if dead { 0 } else { 37 };
                if dead {
                    player.state = ActorState::Dead;
                    session.world.script = 1;
                }
                let expected_world = postcard::to_allocvec(&session.world).unwrap();
                session.world.actors[session.world.player]
                    .actor
                    .invulnerable = true;
                session.world.ai.ignore_player = true;
                session.world.ai.sight_percent = 175;
                session.world.keep_all_active = true;
                let mut settings = Settings {
                    player_model: "body".into(),
                    weapon_model: "weapon".into(),
                    voice_model: "voice".into(),
                    model_directory: "models".into(),
                    circle_confirm: true,
                    keep_enemies_active: true,
                    sight_percent: 175,
                    independent_attacks: true,
                    ..Default::default()
                };
                settings.bleeding.enabled = true;
                settings.walking.enabled = true;
                settings.walking.threshold = 45;
                settings.sheathing.enabled = true;
                settings.sheathing.speed = 125;
                settings.sheathing.one_hit = true;
                session.world.set_bleeding(settings.bleeding);
                settings.set_visual_preset(settings::presets::MODERN_ULTRA);
                let mut expected_settings = settings.clone();
                if all {
                    expected_settings = Settings::default();
                } else {
                    expected_settings.keep_enemies_active = false;
                    expected_settings.sight_percent = 100;
                    expected_settings.bleeding = Default::default();
                    expected_settings.walking = Default::default();
                    expected_settings.sheathing = Default::default();
                }
                let mut content = Content::Gameplay(Box::new(session));
                let mut menu = Menu::new(&settings);
                menu.panel.open = true;
                menu.tools.sync(&content, 7);
                menu.tools.debug.speed = 400;
                menu.tools.debug.hitboxes = true;
                menu.tools.debug.actors_edited = true;
                menu.tools.debug.click_kill = true;
                menu.tools.debug.click_teleport = true;
                menu.tools.debug.teleport_armed = true;
                if !dead {
                    menu.tools.change(Item::GiveItems, 1).unwrap();
                }
                let item = if all { Item::Reset } else { Item::ResetTweaks };
                let (page, row) = (0..settings::menu::PAGES.len())
                    .find_map(|page| {
                        settings::menu::rows(page, &settings, &menu.tools.debug, true)
                            .iter()
                            .position(|row| row.item == item)
                            .map(|row| (page, row))
                    })
                    .unwrap();
                menu.panel.page = page;
                menu.panel.row = row;
                let mut paused = true;
                let mut controls = Controls::default();
                controls.touch(crate::touch::Sample {
                    buttons: u32::from(input::SQUARE),
                    ..Default::default()
                });
                let mut presets = presets();
                let mut state = State {
                    settings: &mut settings,
                    paused: &mut paused,
                    controls: &mut controls,
                    presets: &mut presets,
                    lights: 0,
                    samples: &[1, 4],
                    models: None,
                };
                menu.controller(input::CIRCLE, true);
                let ctx = egui::Context::default();
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    menu.show(ctx, &mut state);
                });
                assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
                menu.apply_tools(&mut content, [800, 600]);
                assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
                assert_eq!(*state.settings, expected_settings);
                assert!(*state.paused && menu.panel.open);
                assert_eq!(menu.tools.debug.speed, 100);
                assert!(!menu.tools.debug.click_kill && !menu.tools.debug.click_teleport);
                assert!(!menu.tools.debug.teleport_armed && !menu.tools.debug.god_mode);
                assert_eq!(menu.tools.debug.hitboxes, !all);
                menu.tools.sync(&content, 7);
                assert!(menu.tools.debug.actors_edited);
                assert_eq!(
                    menu.tools
                        .input(&mut content, state.controls, state.settings, true),
                    0,
                );
                let Content::Gameplay(session) = &content else {
                    unreachable!()
                };
                assert!(postcard::to_allocvec(&session.world).unwrap() == expected_world);
                assert!(session.world.controls.armor);
            }
        }
    }

    #[test]
    fn model_folder_reload_is_transactional_and_keeps_shared_settings() {
        let root =
            std::env::temp_dir().join(format!("tenchu-native-folder-menu-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut settings = Settings {
            model_directory: "previous".into(),
            player_model: "pack:removed".into(),
            ..Default::default()
        };
        let mut menu = Menu::new(&settings);
        assert_eq!(menu.panel.model_directory, "previous");
        let mut action = Action::default();
        menu.panel.model_directory = root.to_string_lossy().into_owned();
        let original = settings.clone();
        assert!(menu.reload_models(&mut settings, &mut action).is_err());
        assert_eq!(settings, original);
        assert!(action.models.is_none());
        let parts: Vec<_> = (0..15).map(|i| serde_json::json!({"file": null, "parent": (i != 0).then_some(0), "position": [0, 0, 0]})).collect();
        std::fs::write(
            root.join("pack.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": 1, "player_models": {"custom": {"name": "Custom", "parts": parts}}
            }))
            .unwrap(),
        )
        .unwrap();
        menu.reload_models(&mut settings, &mut action).unwrap();
        assert_eq!(settings.model_directory, root.to_string_lossy());
        assert!(settings.player_model.is_empty());
        assert!(
            action
                .models
                .as_ref()
                .unwrap()
                .player_models
                .contains_key("custom")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unavailable_controls_cannot_change_saved_preferences_or_open_tools() {
        let mut settings = Settings {
            player_model: "disc:133".into(),
            model_moveset: true,
            rewind_seconds: 20,
            keep_enemies_active: true,
            ..Default::default()
        };
        settings.set_visual_preset(settings::presets::MODERN_ULTRA);
        let original = settings.clone();
        let mut paused = false;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut menu = Menu::default();
        let mut action = Action::default();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1, 4],
            models: None,
        };
        for page in 0..settings::menu::PAGES.len() {
            for row in settings::menu::rows(page, &original, &Debug::default(), false) {
                if Runtime::Tenchu2.unavailable(row.item).is_some() {
                    for edit in [Edit::Change(row.item, 1), Edit::Reset(row.item)] {
                        menu.apply(edit, &mut state, &mut action);
                        assert!(
                            !menu.panel.notice.is_empty(),
                            "{} needs an explanation",
                            row.label
                        );
                        assert_eq!(
                            *state.settings, original,
                            "{} changed an unavailable preference",
                            row.label
                        );
                    }
                }
            }
        }
        for edit in [Edit::Randomizer, Edit::Benchmark, Edit::ImportUnarmed] {
            menu.apply(edit, &mut state, &mut action);
            assert!(!menu.panel.notice.is_empty());
            assert_eq!(*state.settings, original);
        }
        menu.apply(Edit::Diagnostics, &mut state, &mut action);
        assert!(action.save_diagnostics && menu.panel.notice.is_empty());
        assert_eq!(*state.settings, original);
        assert!(!action.leave && !action.select_mission && !action.step);
        assert!(!paused && !controls.editor.open && !presets.open);
    }

    #[test]
    fn weapon_and_voice_menu_choices_persist_and_reset_independently() {
        let mut settings = Settings {
            player_model: "body".into(),
            moveset_model: "moves".into(),
            model_moveset: true,
            ..Default::default()
        };
        let mut paused = false;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut menu = Menu::default();
        menu.panel.weapon_models = vec![
            (String::new(), "Follow moveset".into()),
            ("sword".into(), "Sword".into()),
        ];
        menu.panel.voice_models = vec![
            (String::new(), "Follow moveset".into()),
            ("voice".into(), "Voice".into()),
        ];
        let mut action = Action::default();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[],
            models: None,
        };
        for item in [Item::WeaponModel, Item::VoiceModel] {
            menu.apply(Edit::Change(item, 1), &mut state, &mut action);
            assert!(menu.panel.notice.is_empty());
        }
        assert_eq!(state.settings.weapon_model, "sword");
        assert_eq!(state.settings.voice_model, "voice");
        let saved = serde_json::to_vec(&state.settings).unwrap();
        let restored: Settings = serde_json::from_slice(&saved).unwrap();
        assert_eq!(*state.settings, restored);
        for preset in [0, 1, 2] {
            state.settings.set_visual_preset(preset);
        }
        assert_eq!(state.settings.weapon_model, "sword");
        assert_eq!(state.settings.voice_model, "voice");
        menu.apply(Edit::Reset(Item::WeaponModel), &mut state, &mut action);
        assert!(state.settings.weapon_model.is_empty());
        assert_eq!(state.settings.voice_model, "voice");
        menu.apply(Edit::VoiceModel("second".into()), &mut state, &mut action);
        menu.apply(Edit::WeaponModel("other".into()), &mut state, &mut action);
        assert_eq!(state.settings.voice_model, "second");
        assert_eq!(state.settings.weapon_model, "other");
        menu.apply(Edit::Reset(Item::VoiceModel), &mut state, &mut action);
        assert!(state.settings.voice_model.is_empty());
        assert_eq!(state.settings.player_model, "body");
        assert_eq!(state.settings.moveset_model, "moves");
        assert!(state.settings.model_moveset);
    }

    #[test]
    fn native_settings_share_edits_resets_and_nested_navigation() {
        let mut settings = Settings::default();
        let mut paused = false;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut menu = Menu::default();
        menu.panel.open = true;
        let mut action = Action::default();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1, 4],
            models: None,
        };
        for page in 0..settings::menu::PAGES.len() {
            for row in settings::menu::rows(page, &Settings::default(), &Debug::default(), false) {
                if Runtime::Tenchu2.unavailable(row.item).is_none()
                    && !row.item.button()
                    && !matches!(
                        row.item,
                        Item::FreeCamera
                            | Item::GameplayPreset
                            | Item::FollowActor
                            | Item::ClickTeleport
                            | Item::ClickKill
                            | Item::GodMode
                            | Item::Bandages
                            | Item::Armor
                            | Item::SightDistance
                            | Item::Stealth
                            | Item::FlightSpeed
                            | Item::Fov
                            | Item::Actor
                            | Item::PhotoMode
                            | Item::MovesetModel
                            | Item::WeaponModel
                            | Item::VoiceModel
                            | Item::PlayerModel
                            | Item::DebugLight
                            | Item::FollowLight
                            | Item::LightAxis(_)
                    )
                {
                    let original = state.settings.clone();
                    menu.apply(Edit::Change(row.item, 1), &mut state, &mut action);
                    if row.item == Item::KeepEnemiesActive {
                        assert!(state.settings.keep_enemies_active);
                    }
                    if row.item == Item::SharpText {
                        assert!(state.settings.sharp_text);
                    }
                    assert!(
                        menu.panel.notice.is_empty(),
                        "{}: {}",
                        row.label,
                        menu.panel.notice
                    );
                    menu.apply(Edit::Reset(row.item), &mut state, &mut action);
                    if row.item == Item::LiveDeathScene {
                        assert!(
                            state.settings.wait_after_death && !state.settings.live_death_scene
                        );
                        menu.apply(Edit::Reset(Item::WaitAfterDeath), &mut state, &mut action);
                    }
                    assert!(
                        menu.panel.notice.is_empty(),
                        "{}: {}",
                        row.label,
                        menu.panel.notice
                    );
                    assert_eq!(*state.settings, original, "{} failed to reset", row.label);
                }
            }
        }
        menu.navigate(Navigation::Page(-1), &mut state);
        assert_eq!(menu.panel.page, settings::menu::PAGES.len() - 1);
        menu.apply(Edit::Controls, &mut state, &mut action);
        menu.navigate(Navigation::Action(MenuAction::Back), &mut state);
        assert!(!state.controls.editor.open && menu.panel.open);
        menu.panel.touch_controls = true;
        menu.navigate(Navigation::Action(MenuAction::Back), &mut state);
        assert!(!menu.panel.touch_controls && menu.panel.open);
        menu.apply(
            Edit::Change(Item::Tenchu2Missions, 1),
            &mut state,
            &mut action,
        );
        assert!(action.select_mission && !action.leave);
        menu.apply(Edit::Change(Item::Step, 1), &mut state, &mut action);
        assert!(action.step && *state.paused && !menu.panel.open);
        menu.panel.open = true;
        let before = state.settings.clone();
        menu.apply(Edit::Change(Item::Screenshot, 1), &mut state, &mut action);
        assert!(menu.screenshot && !menu.panel.open && *state.paused);
        assert_eq!(*state.settings, before);
    }

    #[test]
    fn keyboard_and_controller_navigate_without_activating_unsupported_controls() {
        let ctx = egui::Context::default();
        let mut menu = Menu::default();
        menu.panel.open = true;
        let mut settings = Settings::default();
        let mut paused = false;
        let mut controls = Controls::default();
        let mut presets = presets();
        let mut state = State {
            settings: &mut settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1, 4],
            models: None,
        };
        let input = |events| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 540.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input(Vec::new()), |ctx| {
            menu.show(ctx, &mut state);
        });
        // Modifiers may be released before the next video frame processes the key.
        let _ = ctx.run(
            input(vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }]),
            |ctx| {
                menu.show(ctx, &mut state);
            },
        );
        assert_eq!(menu.panel.page, settings::menu::PAGES.len() - 1);
        menu.controller(tenchu_core::input::R1, false);
        let _ = ctx.run(input(Vec::new()), |ctx| {
            menu.show(ctx, &mut state);
        });
        assert_eq!(menu.panel.page, 0);
        menu.panel.page = 10;
        menu.controller(tenchu_core::input::CROSS, false);
        let _ = ctx.run(input(Vec::new()), |ctx| {
            menu.show(ctx, &mut state);
        });
        assert!(!menu.panel.model_picker && menu.panel.open);
        assert_eq!(*state.settings, Settings::default());
        assert!(!menu.panel.notice.is_empty());
        for circle_confirm in [false, true] {
            state.settings.circle_confirm = circle_confirm;
            menu.panel.page = 0;
            menu.panel.touch_controls = true;
            let back = if circle_confirm {
                tenchu_core::input::CROSS
            } else {
                tenchu_core::input::CIRCLE
            };
            menu.controller(back, circle_confirm);
            let _ = ctx.run(input(Vec::new()), |ctx| {
                menu.show(ctx, &mut state);
            });
            assert!(!menu.panel.touch_controls && menu.panel.open);
            menu.controller(back, circle_confirm);
            let _ = ctx.run(input(Vec::new()), |ctx| {
                menu.show(ctx, &mut state);
            });
            assert!(!menu.panel.open);
            menu.panel.open = true;
            menu.panel.row = settings::menu::rows(0, state.settings, &menu.tools.debug, false)
                .iter()
                .position(|row| row.item == Item::CircleConfirm)
                .unwrap();
            menu.controller(
                if circle_confirm {
                    tenchu_core::input::CIRCLE
                } else {
                    tenchu_core::input::CROSS
                },
                circle_confirm,
            );
            let _ = ctx.run(input(Vec::new()), |ctx| {
                menu.show(ctx, &mut state);
            });
            assert_eq!(state.settings.circle_confirm, !circle_confirm);
            let encoded = serde_json::to_vec(state.settings).unwrap();
            assert_eq!(
                serde_json::from_slice::<Settings>(&encoded)
                    .unwrap()
                    .circle_confirm,
                !circle_confirm
            );
        }
    }
}
