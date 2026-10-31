use super::*;
use crate::randomizer::Event;

impl Application {
    pub(super) fn toggle_randomizer(&mut self) {
        if self.demo.is_some()
            || self.benchmarks.run.is_some()
            || self.trial.open
            || self.campaign.game.trial.is_some()
        {
            return;
        }
        self.randomizer.open = !self.randomizer.open;
        if self.randomizer.open {
            self.menu.open = false;
            self.controls.close_editor();
        }
        self.release_pointer();
        self.flush_recording();
        self.sync_pause();
        self.update_view();
    }

    pub(super) fn randomizer_action(&mut self, event: Event) {
        let result = self.apply_randomizer(event);
        if let Err(error) = result {
            self.randomizer.notice = error;
            self.randomizer.open = true;
            self.sync_pause();
        }
    }

    fn apply_randomizer(&mut self, event: Event) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        if let Event::Start(run) = &event {
            let language = run.document.rules.language.game();
            crate::web::prepare_language(&self.assets, language);
            if !crate::web::mission_ready(language)? {
                return Err("Preparing assets and audio for this language; try Start again once the download finishes".into());
            }
        }
        if self.demo.is_some()
            || self.benchmarks.run.is_some()
            || self.trial.open
            || self.campaign.game.trial.is_some()
        {
            return Err("Finish the demo or benchmark first".into());
        }
        let view = self.view.as_mut().ok_or("Renderer is not ready")?;
        match event {
            Event::Start(run) => {
                if view.model_library.is_none() {
                    view.load_model_library(&self.assets, &self.campaign.game, &self.settings)?;
                }
                self.campaign.start_randomizer(
                    (*run).clone(),
                    &self.assets,
                    view.model_library.as_ref().unwrap(),
                )?;
                self.randomizer.started(&run);
            }
            Event::End => {
                self.campaign.end_randomizer(&self.assets)?;
                self.randomizer.open = false;
            }
        }
        self.finish_recording();
        self.menu.open = false;
        self.reload_game()?;
        if self.campaign.randomized.is_none() {
            self.open_title();
        }
        self.sync_pause();
        self.update_view();
        Ok(())
    }

    pub(super) fn randomizer_command(&mut self, action: Action) -> bool {
        if matches!(self.campaign.screen, Screen::DeathHold) {
            return false;
        }
        let Some(active) = &self.campaign.randomized else {
            return false;
        };
        let run = active.run.clone();
        match (&self.campaign.screen, action) {
            (Screen::Results(_), Action::Confirm) => match run.next() {
                Ok(next) => self.randomizer_action(Event::start(next)),
                Err(error) => {
                    self.randomizer.notice = error;
                    self.toggle_randomizer();
                }
            },
            (Screen::GameOver, Action::Confirm) | (_, Action::Replay) => {
                self.randomizer_action(Event::start(run))
            }
            (_, Action::Back) => self.toggle_randomizer(),
            _ => (),
        }
        true
    }

    pub(super) fn reload_game(&mut self) -> Result<()> {
        self.release_pointer();
        if let Some(frontend) = &mut self.frontend {
            frontend.leave();
        }
        if self.campaign.randomized.is_none() {
            self.campaign.game.intelligence.sight_percent = self.settings.sight_percent;
            self.settings
                .apply_activation(&mut self.campaign.game.roster);
        }
        self.debug.sight_percent = self.campaign.game.intelligence.sight_percent;
        self.debug.reload(&self.campaign.game)?;
        if let Some(audio) = &mut self.audio {
            if let Err(error) = audio.reload(&self.assets, &self.campaign.game) {
                eprintln!("Audio: {error}");
            }
            audio.screen(&self.campaign.screen);
        }
        if let Some(view) = &mut self.view {
            view.randomizer_player = self
                .campaign
                .randomized
                .as_ref()
                .map(|r| r.plan.player_model.clone().unwrap_or_default());
            view.reload(&self.campaign.game, &self.settings)?;
        }
        self.clock = GameClock::default();
        self.last_frame = Instant::now();
        self.paused = false;
        self.faulted = false;
        self.fault = None;
        self.controls.clear();
        Ok(())
    }
}
