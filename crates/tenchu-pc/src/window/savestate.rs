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
    pub(super) fn state_tools_available(&self) -> Result<()> {
        if self.demo.is_some() || self.benchmarks.run.is_some() {
            return Err("Finish the demo or benchmark before using savestates".into());
        }
        if self.trial.open || self.randomizer.open {
            return Err("Close the mission/randomizer editor before using savestates".into());
        }
        Ok(())
    }

    fn snapshot(&self) -> Result<State<&Game>> {
        self.state_tools_available()?;
        if !self.campaign.scene_visible() || self.faulted {
            return Err("Start a mission before saving a state".into());
        }
        Ok(State {
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
            audio: self
                .reverse
                .audio
                .clone()
                .or_else(|| self.audio.as_ref().map(crate::audio::Audio::capture_state)),
        })
    }

    pub(super) fn capture_state(&self) -> Result<Vec<u8>> {
        crate::savestate::encode(&self.snapshot()?)
    }

    pub(super) fn capture_rewind(&mut self) -> Result<crate::rewind::Snapshot> {
        use tenchu_assets::checkpoint::Capture;
        let mut recorder = std::mem::take(&mut self.rewind.recorder);
        recorder.begin();
        let result = (|| {
            let State {
                game,
                options,
                progress,
                seed,
                randomized,
                death_hold,
                frontend,
                quick_launcher,
                preferences,
                orbit,
                imports,
                actors_edited,
                visuals,
                audio,
            } = self.snapshot()?;
            game.capture(&mut recorder)?;
            recorder.value(&options)?;
            recorder.value(&progress)?;
            recorder.value(&seed)?;
            recorder.value(&randomized)?;
            recorder.value(&death_hold)?;
            recorder.value(&frontend)?;
            recorder.value(&quick_launcher)?;
            recorder.shared(&serde_json::to_string(&preferences).map_err(|e| e.to_string())?)?;
            recorder.value(&orbit)?;
            recorder.value(&imports)?;
            recorder.value(&actors_edited)?;
            recorder.value(&visuals.is_some())?;
            if let Some(visuals) = visuals {
                visuals.capture(&mut recorder)?;
            }
            recorder.value(&audio)?;
            recorder.finish()
        })();
        self.rewind.recorder = recorder;
        result
    }

    pub(super) fn restore_rewind(&mut self, state: &crate::rewind::Snapshot) -> Result<()> {
        self.restore_snapshot(state.decode()?, true)
    }

    pub(super) fn restore_state(&mut self, bytes: &[u8]) -> Result<()> {
        self.restore_snapshot(crate::savestate::decode(bytes)?, false)
    }

    fn restore_snapshot(&mut self, mut state: State<Game>, rewind: bool) -> Result<()> {
        self.state_tools_available()?;
        state.validate()?;
        #[cfg(target_arch = "wasm32")]
        {
            let language = state.game.mission.scripts.language;
            crate::web::prepare_language(&self.assets, language);
            if !crate::web::mission_ready(language)? {
                return Err("Preparing this savestate's language and audio; load again once the download finishes".into());
            }
        }
        let reuse = rewind
            && self.settings.player_model == state.preferences.player_model
            && same_scene(&state.game, &self.campaign.game);
        let sound = if reuse {
            None
        } else {
            Some((
                crate::audio::game_bank(&self.assets, &state.game)?,
                tenchu_assets::audio::catalog::Catalog::load(
                    &self.assets.executable,
                    state.game.mission.scripts.language,
                )?,
            ))
        };
        let debug = (!reuse)
            .then(|| crate::debug::Debug::new(&state.game))
            .transpose()?;
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
        if !reuse && let Some(setup) = &state.game.mission.trial {
            self.trial
                .restore_playtest(&self.assets, &setup.mission, state.options.character)?;
        }
        self.finish_recording();
        self.rewind.clear();
        self.release_pointer();
        if let Some(frontend) = &mut self.frontend {
            frontend.leave();
        }
        if reuse {
            reuse_scene(&mut state.game, &mut self.campaign.game);
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
        if let Some(debug) = debug {
            self.debug = debug;
        }
        self.debug.pause_view = true;
        self.debug.actor_imports = state.imports;
        self.debug.actors_edited = state.actors_edited;
        self.orbit = state.orbit;
        self.orbit.looking = false;
        self.trial.eligible = false;
        self.recent_inputs = Default::default();
        if let Some((bank, catalog)) = sound
            && let Some(audio) = &mut self.audio
        {
            audio.restore_state(&self.campaign, catalog, bank, None);
            if rewind {
                audio.pause(true, false);
            }
        }
        if rewind {
            self.reverse.audio = state.audio;
        } else if let Some(audio) = &mut self.audio {
            audio.restore_playback(&self.campaign, state.audio);
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
            if !reuse && let Err(error) = view.reload(&self.campaign.game, &self.settings) {
                eprintln!("Savestate restored with original models: {error}");
            }
            if let Some(visuals) = state.visuals {
                view.restore_state(visuals, &self.campaign.game);
            }
        }
        self.controls.clear();
        self.controls.stop_rumble();
        self.clock = GameClock::default();
        if !rewind {
            self.last_frame = Instant::now();
        }
        self.paused = true;
        self.faulted = false;
        self.fault = None;
        self.error = None;
        if !rewind {
            self.reverse = Default::default();
            self.sync_pause();
            self.update_view();
        }
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

fn same_scene(next: &Game, previous: &Game) -> bool {
    let a = &next.mission;
    let b = &previous.mission;
    if a.stage != b.stage
        || a.trial.is_some() != b.trial.is_some()
        || next.player().kind != previous.player().kind
    {
        return false;
    }
    if a.scripts.language != b.scripts.language
        || a.placements != b.placements
        || a.map != b.map
        || a.objects != b.objects
        || a.models != b.models
        || a.common_models != b.common_models
        || a.weapons.models != b.weapons.models
        || a.imported_models != b.imported_models
        || a.imported_voices != b.imported_voices
    {
        return false;
    }
    true
}

fn reuse_scene(next: &mut Game, previous: &mut Game) {
    let a = &mut next.mission;
    let b = &mut previous.mission;
    std::mem::swap(&mut a.map, &mut b.map);
    std::mem::swap(&mut a.objects, &mut b.objects);
    std::mem::swap(&mut a.models, &mut b.models);
    std::mem::swap(&mut a.common_models, &mut b.common_models);
    std::mem::swap(&mut a.weapons.models, &mut b.weapons.models);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; cached capture must retain every field, including edited textures, animations, geometry and scripts"]
    fn cached_rewind_matches_full_serialization_after_asset_changes() {
        for mission in [1, 2, 6, 9] {
            let mut app = crate::window::tests::application(Options {
                mission,
                ..Default::default()
            });
            for step in 0..6 {
                if step == 2 {
                    let assets = &mut app.campaign.game.mission;
                    assets.vram.words[4096] ^= 0x70;
                    assets.clips[0].root[0].value[0] ^= 7;
                    assets.scripts.events[0].next[0] ^= 1;
                    assets.placements[0].position[0] += 1;
                    let node = assets.terrain.rows[0].list;
                    assets.terrain.set_surface(node, -17, 3).unwrap();
                }
                if step == 4 {
                    let assets = &mut app.campaign.game.mission;
                    Arc::make_mut(&mut assets.models)
                        .values_mut()
                        .next()
                        .unwrap()[0]
                        .position[0] += 1;
                    Arc::make_mut(&mut assets.common_models)
                        .values_mut()
                        .next()
                        .unwrap()[0]
                        .vertices[0][0] ^= 1;
                }
                let expected = postcard::to_allocvec(&app.snapshot().unwrap()).unwrap();
                let state = app.capture_rewind().unwrap();
                assert_eq!(state.bytes().unwrap(), expected, "M{mission}, step {step}");
            }
        }
    }
}
