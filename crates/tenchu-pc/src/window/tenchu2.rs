use super::*;
use crate::tenchu2::menu::Event;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::Native;

impl Application {
    pub(super) fn tenchu2_action(&mut self, event: Event) {
        if let Err(error) = self.apply_tenchu2(event) {
            self.tenchu2.notice = error;
            self.tenchu2.open = true;
        }
        self.sync_pause();
        self.update_view();
    }

    pub(super) fn apply_tenchu2(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Quick => {
                self.campaign.screen = Screen::Launcher;
                self.campaign.quick_launcher = true;
                self.campaign.cursor = 1;
                self.tenchu2.open = false;
                self.randomizer.open = false;
                self.menu.open = false;
                self.paused = false;
                if let Some(audio) = &mut self.audio {
                    audio.screen(&self.campaign.screen);
                }
            }
            Event::Options(options) => self.campaign.options = options,
            Event::MissionSelection => {
                #[cfg(not(target_arch = "wasm32"))]
                if self.tenchu2.native() && self.tenchu2.path == self.tenchu2.connected {
                    return self.start_native_tenchu2(true);
                }
            }
            Event::Gaisen(continue_save) => {
                self.campaign.cursor = usize::from(!continue_save);
                self.campaign_action(Action::Confirm);
            }
            Event::Select(edition) => {
                if let Some(edition) = edition {
                    let path = self
                        .tenchu2
                        .discs
                        .iter()
                        .find(|(e, _)| *e == edition)
                        .ok_or("Connect this edition's disc first")?
                        .1
                        .clone();
                    self.tenchu2.path = path;
                    self.apply_tenchu2(Event::Connect)?;
                } else {
                    self.tenchu2.selected = None;
                    self.tenchu2.notice.clear();
                }
                self.campaign.cursor = 7;
            }
            Event::Close => self.tenchu2.open = false,
            Event::Open => {
                if self.campaign.playing() || self.demo.is_some() {
                    return Ok(());
                }
                self.tenchu2.open = true;
                self.randomizer.open = false;
                self.menu.open = false;
                #[cfg(not(target_arch = "wasm32"))]
                if self.tenchu2.path.is_empty()
                    && let Some(path) = crate::tenchu2::menu::disc_path()
                {
                    self.tenchu2.path = path.to_string_lossy().into();
                    return self.apply_tenchu2(Event::Connect);
                }
            }
            Event::Browse => {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select your Tenchu 2 disc")
                    .add_filter("Tenchu 2 disc or archive", crate::source::DISC_EXTENSIONS)
                    .pick_file()
                {
                    self.tenchu2.path = path.to_string_lossy().into();
                    return self.apply_tenchu2(Event::Connect);
                }
                #[cfg(target_os = "android")]
                crate::android::import("tenchu2")?;
            }
            Event::Connect => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let path = PathBuf::from(self.tenchu2.path.trim());
                    let path = std::fs::canonicalize(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    let disc = crate::source::open_disc(&path)?;
                    let maps = tenchu_assets::tenchu2::level::Level::available(&disc)?;
                    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc)?;
                    let music = tenchu_assets::tenchu2::music::tracks(&disc)?;
                    if maps.is_empty() {
                        return Err("No Tenchu 2 maps found".into());
                    }
                    crate::tenchu2::menu::remember(&path)?;
                    self.tenchu2.refresh_discs();
                    self.tenchu2.selected = Some(tenchu_assets::tenchu2::Edition::detect(&disc)?);
                    self.tenchu2.path = path.to_string_lossy().into();
                    self.tenchu2.connected.clone_from(&self.tenchu2.path);
                    self.tenchu2.maps = maps;
                    self.tenchu2.layouts = layouts;
                    self.tenchu2.music_tracks = music.len();
                    if !self.tenchu2.maps.contains(&self.tenchu2.map) {
                        self.tenchu2.map = self.tenchu2.maps[0];
                    }
                    self.tenchu2.sequence = self
                        .tenchu2
                        .layouts
                        .iter()
                        .find(|l| l.map == self.tenchu2.map)
                        .map(|l| l.sequence);
                    self.tenchu2.notice = format!(
                        "{}: {} areas available. Disc path remembered.",
                        tenchu_assets::tenchu2::Edition::detect(&disc)?.name(),
                        self.tenchu2.maps.len()
                    );
                    if let Some(view) = &mut self.view {
                        view.model_library = None;
                        view.model_picker.clear();
                        self.menu.moveset_models.clear();
                    }
                }
            }
            Event::Play => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if self.tenchu2.path != self.tenchu2.connected {
                        return Err("Load the selected disc first".into());
                    }
                    if self.tenchu2.native() {
                        return self.start_native_tenchu2(false);
                    }
                    let guard = self.tenchu2.sequence.and(self.tenchu2.guard);
                    let mut sandbox = crate::tenchu2::Sandbox::open(
                        self.tenchu2.path.as_ref(),
                        Some(self.tenchu2.map),
                        self.tenchu2.sequence,
                        guard,
                    )?;
                    sandbox.source_models = self.tenchu2.source_models;
                    sandbox.music.selected = self.tenchu2.music;
                    let previous = self.campaign.options;
                    self.campaign.options.character = self.tenchu2.character;
                    if let Err(error) = self.campaign.start_sandbox(sandbox, &self.assets) {
                        self.campaign.options = previous;
                        return Err(error);
                    }
                    self.finish_recording();
                    self.tenchu2.open = false;
                    self.reload_game()?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[ignore = "requires Gaisen, TENCHU2_DISC, TENCHU2_JP_DISC and isolated TENCHU_T2_CONFIG; both editions persist and remain independently selectable"]
    fn both_editions_remain_connected_after_switching_and_restarting() {
        let config = std::env::var_os("TENCHU_T2_CONFIG").expect("Isolate disc preferences");
        let europe = std::fs::canonicalize(std::env::var_os("TENCHU2_DISC").unwrap()).unwrap();
        let japan = std::fs::canonicalize(std::env::var_os("TENCHU2_JP_DISC").unwrap()).unwrap();
        std::fs::create_dir_all(Path::new(&config).parent().unwrap()).unwrap();
        // Upgrade the previously saved single-disc format.
        std::fs::write(&config, serde_json::to_vec(&europe).unwrap()).unwrap();
        let mut app = super::super::tests::application(Options::default());
        app.campaign.screen = Screen::Launcher;
        app.apply_tenchu2(Event::Open).unwrap();
        assert_eq!(app.tenchu2.discs.len(), 1);
        app.tenchu2.path = japan.to_string_lossy().into();
        app.apply_tenchu2(Event::Connect).unwrap();
        assert_eq!(app.tenchu2.maps.len(), 23);
        assert_eq!(app.tenchu2.discs.len(), 2);
        app.tenchu2 = Default::default();
        app.apply_tenchu2(Event::Open).unwrap();
        assert_eq!(Path::new(&app.tenchu2.connected), japan);
        app.tenchu2.path = europe.to_string_lossy().into();
        app.apply_tenchu2(Event::Connect).unwrap();
        assert_eq!(app.tenchu2.discs.len(), 2);
        let mission = app.campaign.options.mission;
        app.apply_tenchu2(Event::Close).unwrap();
        for edition in [
            tenchu_assets::tenchu2::Edition::Japan,
            tenchu_assets::tenchu2::Edition::Europe,
        ] {
            app.apply_tenchu2(Event::Select(Some(edition))).unwrap();
            assert_eq!(app.tenchu2.selected, Some(edition));
            app.action(Action::Down);
            assert_eq!(app.campaign.cursor, 4);
            app.action(Action::Right);
            assert_eq!(app.campaign.options.mission, mission);
            assert!(
                app.tenchu2
                    .layouts
                    .iter()
                    .any(|l| l.map == app.tenchu2.map && Some(l.sequence) == app.tenchu2.sequence)
            );
        }
        app.apply_tenchu2(Event::Select(None)).unwrap();
        for edition in [
            Some(tenchu_assets::tenchu2::Edition::Europe),
            Some(tenchu_assets::tenchu2::Edition::Japan),
            None,
        ] {
            app.action(Action::Right);
            assert_eq!(app.tenchu2.selected, edition);
        }
        app.action(Action::Down);
        app.action(Action::Right);
        assert_eq!(app.campaign.options.mission, (mission + 1) % 11);
        app.apply_tenchu2(Event::Gaisen(false)).unwrap();
        assert!(matches!(app.campaign.screen, Screen::Inventory(_)));
        let library = crate::visuals::appearance::library::Library::load(&app.assets).unwrap();
        for name in [
            "rikimaru", "ayame", "tatumaru", "ayamea", "tatua_m", "tatu2_m",
        ] {
            let eu = library.get(&format!("tenchu2:{name}")).unwrap();
            let jp = library.get(&format!("tenchu2-jp:{name}")).unwrap();
            assert!(!std::ptr::eq(eu, jp));
            assert!(!eu.has_moveset() && !jp.has_moveset());
        }
        assert!(
            library.get("tenchu2:rikimaru").unwrap().vram
                != library.get("tenchu2-jp:rikimaru").unwrap().vram
        );
        std::fs::remove_file(config).unwrap();
    }

    #[test]
    #[ignore = "requires TENCHU_DISC, TENCHU2_DISC and isolated TENCHU_T2_CONFIG; actual menu selection, retry and campaign isolation"]
    fn disc_menu_connects_and_switches_sandbox_without_campaign_writes() {
        let temporary = std::env::var_os("TENCHU_T2_CONFIG").expect("Isolate Tenchu 2 preferences");
        let disc = std::env::var_os("TENCHU2_DISC").unwrap();
        let mut app = super::super::tests::application(Options::default());
        app.campaign.screen = Screen::Launcher;
        app.tenchu2.path = disc.to_string_lossy().into();
        app.apply_tenchu2(Event::Open).unwrap();
        assert!(app.is_suspended());
        app.apply_tenchu2(Event::Connect).unwrap();
        assert_eq!(app.tenchu2.maps.len(), 23);
        assert_eq!(app.tenchu2.music_tracks, 8);
        assert_eq!(
            crate::tenchu2::menu::disc_path().unwrap(),
            std::fs::canonicalize(&disc).unwrap()
        );
        app.campaign.saved = Some(tenchu_core::progress::save::Save::default());
        let saved = app.campaign.saved.as_ref().unwrap().encode();
        for (map, character, guard) in [(1, 1, Some(0x11)), (20, 0, None)] {
            app.tenchu2.map = map;
            app.tenchu2.character = character;
            app.tenchu2.guard = guard;
            app.tenchu2.music = (map == 1).then_some(7);
            app.tenchu2.sequence = app
                .tenchu2
                .layouts
                .iter()
                .find(|l| l.map == map)
                .map(|l| l.sequence);
            app.apply_tenchu2(Event::Play).unwrap();
            assert!(app.campaign.playing());
            assert!(!app.is_suspended());
            assert_eq!(app.campaign.game.player().kind, character);
            assert!(app.campaign.game.mission.world_vram.is_some());
            assert_eq!(
                app.campaign.sandbox.as_ref().unwrap().music.selected,
                app.tenchu2.music
            );
            app.campaign.checkpoint(app.campaign.options);
            assert!(app.campaign.save_request.is_none());
            for _ in 0..60 {
                assert!(app.game_tick(tenchu_core::input::UP));
            }
            app.action(Action::Replay);
            assert_eq!(app.campaign.game.tick, 0);
            app.action(Action::Back);
            assert!(!app.campaign.playing());
            assert!(app.campaign.sandbox.is_none());
            assert_eq!(app.campaign.saved.as_ref().unwrap().encode(), saved);
            app.apply_tenchu2(Event::Open).unwrap();
        }
        app.tenchu2.path.push_str(".missing");
        assert!(app.apply_tenchu2(Event::Connect).is_err());
        assert_eq!(
            crate::tenchu2::menu::disc_path().unwrap(),
            std::fs::canonicalize(&disc).unwrap()
        );
        app.apply_tenchu2(Event::Close).unwrap();
        assert!(!app.is_suspended());
        std::fs::remove_file(temporary).unwrap();
    }
}
