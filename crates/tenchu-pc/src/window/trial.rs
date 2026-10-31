use super::*;
use crate::trial::Event;
use tenchu_assets::{executable::Executable, stage::MissionAssets};
use tenchu_core::game::events::Outcome;

impl Application {
    pub(super) fn open_trial(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::prepare_language(&self.assets, tenchu_assets::script::Language::Japanese);
            if !crate::web::mission_ready(tenchu_assets::script::Language::Japanese)
                .unwrap_or(false)
            {
                if let Some(frontend) = &mut self.frontend {
                    frontend.notice = "Preparing trial assets and Japanese audio in the background. Press Enter to try again.".into();
                }
                return;
            }
        }
        if self.demo.is_some()
            || self.benchmarks.run.is_some()
            || self.campaign.randomized.is_some()
        {
            return;
        }
        match self.trial.enter(&self.assets) {
            Ok(()) => {
                self.menu.open = false;
                self.randomizer.open = false;
                self.finish_recording();
                self.sync_pause();
                self.update_view();
            }
            Err(e) => {
                if let Some(frontend) = &mut self.frontend {
                    frontend.notice = e;
                } else {
                    self.pause_error(e);
                }
            }
        }
    }
    pub(super) fn trial_action(&mut self, event: Event) {
        if let Err(e) = self.apply_trial(event) {
            self.trial.notice = e;
            self.trial.open = true;
            self.sync_pause();
        }
        self.update_view();
    }
    fn apply_trial(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Play { preview } => {
                self.trial.persist()?;
                let mission = self
                    .trial
                    .mission
                    .as_ref()
                    .ok_or("Choose a trial mission first")?;
                let theme = self
                    .trial
                    .theme
                    .as_ref()
                    .ok_or("Trial theme is not loaded")?;
                let tables = self.trial.tables.as_ref().unwrap();
                let mut random = tenchu_core::random::Random::new(1);
                let prepared = MissionAssets::load_trial(
                    &self.assets,
                    tables,
                    theme,
                    mission,
                    self.trial.character,
                    || random.next15() as u16,
                )?;
                let game = Game::with_random(
                    prepared,
                    &Executable::new(&self.assets.executable)?,
                    self.trial.character,
                    random,
                )?;
                let previous = std::mem::replace(&mut self.campaign.game, game);
                if self.trial_previous.is_none() {
                    self.trial_previous = Some((Box::new(previous), self.campaign.options));
                }
                self.campaign.options.character = self.trial.character;
                self.campaign.screen = Screen::Playing;
                self.campaign.save_request = None;
                self.trial.open = false;
                self.trial.preview = preview;
                self.trial.eligible = !preview;
                self.menu.open = false;
                self.reload_game()?;
                if let Some(audio) = &mut self.audio {
                    audio.music(
                        [8, 0, 1, 2, 9, 10, 3, 4, 5, 6, 7, 18]
                            [self.trial.mission.as_ref().unwrap().theme],
                    );
                }
                if preview {
                    self.debug.toggle_camera(&self.campaign.game);
                    self.debug.focus(&self.campaign.game, true);
                    self.paused = true;
                }
            }
            Event::Return => {
                self.trial.finished(&self.campaign.game);
                self.campaign.screen = Screen::Frontend;
                self.paused = false;
                self.menu.open = false;
                if let Some(audio) = &mut self.audio {
                    audio.screen(&self.campaign.screen);
                }
            }
            Event::Close => {
                self.trial.persist()?;
                self.trial.open = false;
                self.trial.preview = false;
                if let Some((game, options)) = self.trial_previous.take() {
                    self.campaign.game = *game;
                    self.campaign.options = options;
                    self.campaign.screen = Screen::Frontend;
                    self.reload_game()?;
                }
                self.open_title();
            }
        }
        self.sync_pause();
        Ok(())
    }
    pub(super) fn trial_record_check(&mut self) {
        if self.campaign.game.trial.is_none() {
            return;
        }
        let game = &self.campaign.game;
        if self.settings.keep_enemies_active
            || self.settings.sight_percent != 100
            || self.settings.model_moveset
            || game.player().invulnerable
            || game.intelligence.ignore_player
            || self.debug.speed != 100
            || self.debug.camera.is_some()
        {
            self.trial.eligible = false;
        }
        if game.events.outcome != Outcome::Running {
            self.trial_action(Event::Return);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires TENCHU_DISC; trial completion/return must not mutate campaign saves or state"]
    fn trial_playtest_returns_without_changing_campaign() {
        let mut app = super::super::tests::application(Options::default());
        let path =
            std::env::temp_dir().join(format!("tenchu-trial-flow-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        app.trial = crate::trial::Editor::new(path.clone());
        let progress = app.campaign.progress.clone();
        let position = app.campaign.game.player().position;
        let rng = app.campaign.game.random.seed;
        let seed = app.campaign.seed;
        app.trial.enter(&app.assets).unwrap();
        app.trial.theme = Some(tenchu_assets::trial::Theme::load(&app.assets, 0).unwrap());
        app.trial.mission = Some(
            tenchu_assets::trial::Mission::new(0, tenchu_assets::trial::Objective::ReachLord, 140)
                .unwrap(),
        );
        app.trial.character = 1;
        app.apply_trial(Event::Play { preview: false }).unwrap();
        assert!(app.campaign.playing());
        assert_eq!(app.campaign.game.player().kind, 1);
        assert!(app.game_tick(0));
        let target = app
            .campaign
            .game
            .roster
            .get(app.campaign.game.roster.order[1])
            .position;
        app.campaign.game.teleport_player(target, true).unwrap();
        for _ in 0..450 {
            if !app.game_tick(0) {
                break;
            }
        }
        assert_eq!(app.campaign.game.events.outcome, Outcome::Complete);
        assert!(app.trial.open);
        assert_eq!(app.trial.mission.as_ref().unwrap().records.len(), 1);
        assert!(app.campaign.save_request.is_none());
        assert_eq!(app.campaign.progress, progress);
        app.apply_trial(Event::Close).unwrap();
        assert!(app.campaign.game.trial.is_none());
        assert_eq!(app.campaign.options.character, 0);
        assert_eq!(app.campaign.game.player().position, position);
        assert_eq!(app.campaign.game.random.seed, rng);
        assert_eq!(app.campaign.seed, seed);
        assert_eq!(app.campaign.progress, progress);
        std::fs::remove_file(path).unwrap();
    }
}
