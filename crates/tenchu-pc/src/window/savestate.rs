use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
struct State<G> {
    game: G,
    options: Options,
    progress: tenchu_core::progress::Progress,
    seed: u32,
    randomized: Option<crate::campaign::Randomized>,
    death_hold: Option<u32>,
    frontend: bool,
    quick_launcher: bool,
    #[serde(with = "crate::savestate::json")]
    preferences: Settings,
    orbit: crate::navigation::Orbit,
    imports: BTreeMap<String, crate::visuals::appearance::library::Imported>,
    actors_edited: bool,
    visuals: Option<view::State>,
    audio: Option<crate::audio::State>,
}

impl Application {
    fn state_tools_available(&self) -> Result<()> {
        if self.demo.is_some() || self.benchmarks.run.is_some() {
            return Err("Finish the demo or benchmark before using savestates".into());
        }
        if self.trial.open || self.randomizer.open {
            return Err("Close the mission/randomizer editor before using savestates".into());
        }
        Ok(())
    }

    pub(super) fn capture_state(&self) -> Result<Vec<u8>> {
        self.state_tools_available()?;
        if !self.campaign.scene_visible() || self.faulted {
            return Err("Start a mission before saving a state".into());
        }
        crate::savestate::encode(&State {
            game: &self.campaign.game,
            options: self.campaign.options,
            progress: self.campaign.progress.clone(),
            seed: self.campaign.seed,
            randomized: self.campaign.randomized.clone(),
            death_hold: match self.campaign.screen {
                Screen::DeathHold { retry_seed } => Some(retry_seed),
                _ => None,
            },
            frontend: self.campaign.frontend,
            quick_launcher: self.campaign.quick_launcher,
            preferences: self.settings.clone(),
            orbit: self.orbit.clone(),
            imports: self.debug.actor_imports.clone(),
            actors_edited: self.debug.actors_edited,
            visuals: self.view.as_ref().map(View::capture_state),
            audio: self.audio.as_ref().map(crate::audio::Audio::capture_state),
        })
    }

    pub(super) fn restore_state(&mut self, bytes: &[u8]) -> Result<()> {
        self.state_tools_available()?;
        let state: State<Game> = crate::savestate::decode(bytes)?;
        state.validate()?;
        #[cfg(target_arch = "wasm32")]
        {
            let language = state.game.mission.scripts.language;
            crate::web::prepare_language(&self.assets, language);
            if !crate::web::mission_ready(language)? {
                return Err("Preparing this savestate's language and audio; load again once the download finishes".into());
            }
        }
        // Do all fallible disc work before replacing the running mission.
        let bank = crate::audio::game_bank(&self.assets, &state.game)?;
        let catalog = tenchu_assets::audio::catalog::Catalog::load(
            &self.assets.executable,
            state.game.mission.scripts.language,
        )?;
        let debug = crate::debug::Debug::new(&state.game)?;
        let selection = state
            .randomized
            .as_ref()
            .and_then(|r| r.plan.player_model.as_ref())
            .unwrap_or(&state.preferences.player_model);
        let library = if self
            .view
            .as_ref()
            .is_some_and(|v| v.model_library.is_none())
            && (selection.starts_with("disc:") || selection.starts_with("resource:"))
        {
            Some(crate::visuals::appearance::library::Library::load(
                &self.assets,
            )?)
        } else {
            None
        };
        if let Some(setup) = &state.game.mission.trial {
            self.trial
                .restore_playtest(&self.assets, &setup.mission, state.options.character)?;
        }
        self.finish_recording();
        self.release_pointer();
        if let Some(frontend) = &mut self.frontend {
            frontend.leave();
        }
        if state.game.trial.is_some() {
            if self.trial_previous.is_none() {
                // Preserve the normal campaign scene when entering a saved TRIAL attempt.
                let previous = std::mem::replace(&mut self.campaign.game, state.game);
                self.trial_previous = Some((Box::new(previous), self.campaign.options));
            } else {
                self.campaign.game = state.game;
            }
        } else {
            self.campaign.game = state.game;
            self.trial_previous = None;
            self.trial.open = false;
            self.trial.preview = false;
        }
        self.campaign.options = state.options;
        self.campaign.progress = state.progress;
        self.campaign.seed = state.seed;
        self.campaign.randomized = state.randomized;
        self.campaign.frontend = state.frontend;
        self.campaign.quick_launcher = state.quick_launcher;
        self.campaign.screen =
            state
                .death_hold
                .map_or(Screen::Playing, |retry_seed| Screen::DeathHold {
                    retry_seed,
                });
        self.campaign.save_request = None;
        self.campaign.cursor = 0;
        self.settings
            .restore_gameplay(&state.preferences, &self.campaign.game);
        self.debug = debug;
        self.debug.pause_view = true;
        self.debug.actor_imports = state.imports;
        self.debug.actors_edited = state.actors_edited;
        self.orbit = state.orbit;
        self.orbit.looking = false;
        self.trial.eligible = false;
        self.recent_inputs = Default::default();
        if let Some(audio) = &mut self.audio {
            audio.restore_state(&self.campaign, catalog, bank, state.audio);
        }
        if let Some(view) = &mut self.view {
            if library.is_some() {
                view.model_library = library;
            }
            view.randomizer_player = self
                .campaign
                .randomized
                .as_ref()
                .map(|r| r.plan.player_model.clone().unwrap_or_default());
            if let Err(error) = view.reload(&self.campaign.game, &self.settings) {
                eprintln!("Savestate restored with original models: {error}");
            }
            if let Some(visuals) = state.visuals {
                view.restore_state(visuals, &self.campaign.game);
            }
        }
        self.controls.clear();
        self.controls.stop_rumble();
        self.clock = GameClock::default();
        self.last_frame = Instant::now();
        self.paused = true;
        self.faulted = false;
        self.fault = None;
        self.error = None;
        self.sync_pause();
        self.update_view();
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn state_action(&mut self, save: bool) -> Result<()> {
        let slot = self.debug.state_slot;
        let path = crate::storage::UserFile::States
            .path()?
            .join(format!("slot-{}.tstate", slot + 1));
        let message = if save {
            let bytes = self.capture_state()?;
            crate::storage::atomic_write(&path, &bytes)?;
            self.clock.discard_host_backlog();
            self.last_frame = Instant::now();
            format!(
                "Saved state {} · M{} · tick {}",
                slot + 1,
                self.campaign.options.mission,
                self.campaign.game.tick
            )
        } else {
            let bytes = crate::savestate::read(&path)?;
            self.restore_state(&bytes)?;
            self.debug.state_slot = slot;
            format!("Loaded state {} · F3 to resume", slot + 1)
        };
        self.state_notice(message);
        Ok(())
    }

    pub(super) fn state_notice(&mut self, message: String) {
        self.menu.notice.clone_from(&message);
        self.debug.notice = Some((message, Instant::now()));
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn state_action(&mut self, save: bool) -> Result<()> {
        self.state_tools_available()?;
        let bytes = save.then(|| self.capture_state()).transpose()?;
        crate::web::savestate::start(self.debug.state_slot, bytes)?;
        self.debug.pause_view = true;
        self.set_paused(true);
        self.state_notice(
            if save {
                "Saving state…"
            } else {
                "Loading state…"
            }
            .into(),
        );
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn poll_state(&mut self) {
        use crate::web::savestate::Completed;
        let Some(completed) = crate::web::savestate::poll() else {
            return;
        };
        let message = match completed {
            Completed::Saved(slot) => format!("Saved state {} · F3 to resume", slot + 1),
            Completed::Loaded(slot, bytes) => match self.restore_state(&bytes) {
                Ok(()) => {
                    self.debug.state_slot = slot;
                    format!("Loaded state {} · F3 to resume", slot + 1)
                }
                Err(error) => error,
            },
            Completed::Failed(error) => error,
        };
        self.state_notice(message);
        self.update_view();
    }
}

impl State<Game> {
    fn validate(&self) -> Result<()> {
        let game = &self.game;
        if self.options.mission > 10
            || !(0..=1).contains(&self.options.character)
            || self.options.layout > 3
            || game.mission.stage.id >= 11
            || game
                .roster
                .actors
                .get(game.player_id)
                .is_none_or(Option::is_none)
            || game.collision.len > game.collision.slots.len()
            || game.mission.vram.words.len() != 1024 * 512
        {
            return Err("Invalid mission or player in savestate".into());
        }
        for &id in &game.roster.order {
            let actor = game
                .roster
                .actors
                .get(id)
                .and_then(Option::as_ref)
                .ok_or("Missing savestate actor")?;
            if actor.id != id
                || actor.parts.is_empty()
                || actor.definition >= game.mission.characters.len()
                || actor.root_model >= game.scene.models.len()
                || actor.parts.iter().any(|&p| p >= game.scene.models.len())
                || actor
                    .motion
                    .clip
                    .is_some_and(|c| c >= game.mission.clips.len())
            {
                return Err("Invalid actor reference in savestate".into());
            }
        }
        Ok(())
    }
}
