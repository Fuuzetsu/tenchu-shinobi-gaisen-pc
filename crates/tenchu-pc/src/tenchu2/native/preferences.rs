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
        models: crate::settings::Models {
            characters: g.models.characters,
            grapple: g.models.grapple,
            ..Default::default()
        },
        blood: crate::settings::Blood {
            hit: g.blood.hit,
            ground: g.blood.ground,
            death: g.blood.death,
            player_death: g.blood.player_death,
            death_size: g.blood.death_size,
            retain_original: g.blood.retain_original,
            conform_retained: g.blood.conform_retained,
            amount: g.blood.amount,
            marks: g.blood.marks,
            runoff: g.blood.runoff,
            ..Default::default()
        },
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
            splashes: g.effects.splashes,
            spark_density: g.effects.spark_density,
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
    pub leave: bool,
    pub select_mission: bool,
    pub step: bool,
    pub clear_weather: bool,
    pub clear_blood: bool,
    pub clear_footprints: bool,
    pub models: Option<std::sync::Arc<crate::visuals::pack::Pack>>,
    pub reload_artwork: bool,
    pub save_diagnostics: bool,
}

impl Menu {
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
        if let Err(error) = self.tools.apply(content, size) {
            self.panel.notice = error.clone();
            self.tools.debug.notice = Some((error, web_time::Instant::now()));
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

    pub fn controller(&mut self, pressed: u16) {
        if !self.panel.open {
            return;
        }
        use tenchu_core::input;
        self.pending = if pressed & input::L1 != 0 {
            Some(Navigation::Page(-1))
        } else if pressed & input::R1 != 0 {
            Some(Navigation::Page(1))
        } else {
            match controls::gamepad_command(pressed, false, false, false) {
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
            return action;
        }
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

    fn apply(&mut self, edit: Edit, state: &mut State<'_>, action: &mut Action) {
        self.panel.notice.clear();
        if let Some(reason) = Runtime::Tenchu2.unavailable_edit(&edit) {
            self.panel.notice = reason.into();
            return;
        }
        let result = match edit {
            Edit::Diagnostics => {
                action.save_diagnostics = true;
                Ok(())
            }
            Edit::PlayerModel(id) => {
                state.settings.player_model = id;
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
                        Item::KeepEnemiesActive => {
                            state.settings.keep_enemies_active =
                                !state.settings.keep_enemies_active;
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
        for edit in [
            Edit::GameplayPreset(0),
            Edit::Randomizer,
            Edit::Benchmark,
            Edit::ImportRewind,
            Edit::ImportUnarmed,
        ] {
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
                            | Item::FollowActor
                            | Item::ClickTeleport
                            | Item::GodMode
                            | Item::Armor
                            | Item::SightDistance
                            | Item::Stealth
                            | Item::FlightSpeed
                            | Item::Fov
                            | Item::Actor
                            | Item::PhotoMode
                            | Item::MovesetModel
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
        menu.controller(tenchu_core::input::R1);
        let _ = ctx.run(input(Vec::new()), |ctx| {
            menu.show(ctx, &mut state);
        });
        assert_eq!(menu.panel.page, 0);
        menu.panel.page = 10;
        menu.controller(tenchu_core::input::CROSS);
        let _ = ctx.run(input(Vec::new()), |ctx| {
            menu.show(ctx, &mut state);
        });
        assert!(!menu.panel.model_picker && menu.panel.open);
        assert_eq!(*state.settings, Settings::default());
        assert!(!menu.panel.notice.is_empty());
    }
}
