use super::*;
use crate::tenchu2::menu::Event;

impl Application {
    pub(super) fn tenchu2_action(&mut self, event: Event) {
        if let Err(error) = self.apply_tenchu2(event) {
            self.tenchu2.notice = error;
            self.tenchu2.open = true;
        }
        self.sync_pause();
        self.update_view();
    }

    fn apply_tenchu2(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Close => self.tenchu2.open = false,
            Event::Open => {
                if self.campaign.playing() || self.demo.is_some() {
                    return Ok(());
                }
                self.tenchu2.open = true;
                self.randomizer.open = false;
                self.menu.open = false;
                #[cfg(not(target_arch = "wasm32"))]
                if self.tenchu2.path.is_empty() {
                    self.tenchu2.guard = Some(0x10);
                    if let Some(path) = crate::tenchu2::menu::disc_path() {
                        self.tenchu2.path = path.to_string_lossy().into();
                        return self.apply_tenchu2(Event::Connect);
                    }
                }
            }
            Event::Browse => {
                #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select your Tenchu 2 Europe disc")
                    .add_filter("Tenchu 2 disc", &["cue", "bin", "iso"])
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
                    if maps.is_empty() {
                        return Err("No Tenchu 2 maps found".into());
                    }
                    crate::tenchu2::menu::remember(&path)?;
                    self.tenchu2.path = path.to_string_lossy().into();
                    self.tenchu2.connected.clone_from(&self.tenchu2.path);
                    self.tenchu2.maps = maps;
                    self.tenchu2.layouts = layouts;
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
                        "{} areas available. Disc path remembered.",
                        self.tenchu2.maps.len()
                    );
                }
            }
            Event::Play => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if self.tenchu2.path != self.tenchu2.connected {
                        return Err("Load the selected disc first".into());
                    }
                    let disc = crate::source::open_disc(self.tenchu2.path.as_ref())?;
                    let guard = self.tenchu2.sequence.and(self.tenchu2.guard);
                    let sandbox = crate::tenchu2::Sandbox::open(
                        &disc,
                        Some(self.tenchu2.map),
                        self.tenchu2.sequence,
                        guard,
                    )?;
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
