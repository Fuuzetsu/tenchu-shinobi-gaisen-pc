use super::*;
use crate::level_editor::{Action as CueAction, Command, Observation, Runtime};
use tenchu_assets::{executable::Executable, stage::MissionAssets};
use tenchu_core::game::events::Outcome;
mod audio;
#[cfg(test)]
mod tests;

pub(super) struct Session {
    campaign: Campaign,
    settings: Settings,
    debug: crate::debug::Debug,
    frontend: Option<crate::frontend::Frontend>,
    paused: bool,
    fault: Option<String>,
    visual: Option<view::State>,
    audio: Option<crate::audio::State>,
    #[cfg(not(target_arch = "wasm32"))]
    native: Option<tenchu2::Native>,
    slots: Vec<usize>,
    message_until: Option<u64>,
    sounds: audio::Sounds,
    preview_project: Option<crate::level_editor::Project>,
    preview_tick: u64,
    preview_fraction: f64,
    eligible: bool,
    #[cfg(target_arch = "wasm32")]
    pending_audio: Option<Command>,
}

impl Application {
    pub(super) fn studio_action(&mut self, command: Command) {
        #[cfg(target_arch = "wasm32")]
        if let Some(session) = &mut self.studio_previous {
            session.pending_audio = None;
        }
        if let Err(error) = self.apply_studio(command) {
            self.studio.notice = error.clone();
            if self.studio_previous.is_some() {
                self.studio.open = true;
                self.studio.testing = false;
                self.paused = true;
                self.debug.camera = Some(self.studio.camera);
            }
            if self.studio_previous.is_none() {
                if let Some(frontend) = &mut self.frontend {
                    frontend.notice = error;
                } else {
                    self.menu.notice = error;
                }
            }
        }
        self.sync_pause();
        self.update_view();
    }

    fn build_studio(&mut self) -> Result<Game> {
        self.studio.project.validate()?;
        let tables = self
            .studio
            .tables
            .as_ref()
            .ok_or("Open Level Studio first")?;
        let theme = self.studio.theme.as_ref().ok_or("Theme is not loaded")?;
        let project = self.studio.project.clone();
        let mut random = tenchu_core::random::Random::new(1);
        let mission = MissionAssets::load_trial(
            &self.assets,
            tables,
            theme,
            &project.mission,
            project.character,
            || random.next15() as u16,
        )?;
        let mut game = Game::with_random(
            mission,
            &Executable::new(&self.assets.executable)?,
            project.character,
            random,
        )?;
        let materials = self.studio.add_scenery(&mut game.mission, &self.assets)?;
        if !project.actors.is_empty() {
            let library = self
                .view
                .as_ref()
                .and_then(|v| v.model_library.as_ref())
                .ok_or("Character catalogue has not finished loading")?;
            let mut imported = Default::default();
            for (&slot, model) in &project.actors {
                if library.get(model).is_none() {
                    return Err(format!(
                        "Character asset {model} is unavailable; connect its source disc before opening this project"
                    ));
                }
                if slot == 0 {
                    continue;
                }
                let id = game.roster.order[slot];
                if matches!(game.roster.get(id).kind, 5 | 6 | 170) {
                    return Err("Keep original appearances for Lord Gohda, Princess Kiku and the objective herb".into());
                }
                let source = crate::visuals::appearance::library::import_actor(
                    &mut game.mission,
                    library,
                    model,
                    &mut imported,
                )?;
                game.replace_actor(id, source.actor, source.think)?;
                game.roster.get_mut(id).voice_program = source.voice;
            }
            game.collision.compute(&mut game.scene)?;
        }
        self.studio.surfaces = crate::debug::terrain::surfaces(&game.mission.terrain)?;
        if let Some(view) = &mut self.view {
            view.studio_scenery = materials;
        }
        Ok(game)
    }

    pub(super) fn apply_studio(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Open => {
                if self.studio_previous.is_some() {
                    return Ok(());
                }
                if self.demo.is_some() || self.benchmarks.run.is_some() || self.trial.open {
                    return Err("Return to the title before opening Level Studio".into());
                }
                #[cfg(target_arch = "wasm32")]
                {
                    crate::web::prepare_language(
                        &self.assets,
                        tenchu_assets::script::Language::Japanese,
                    );
                    if !crate::web::mission_ready(tenchu_assets::script::Language::Japanese)? {
                        return Err("Preparing editor assets in the background; open Level Studio again in a moment".into());
                    }
                }
                self.studio.inherit_graphics(&self.settings.graphics);
                self.studio.enter(&self.assets)?;
                if let Some(view) = &mut self.view {
                    if view.model_library.is_none() {
                        view.load_model_library(&self.assets, &self.campaign.game, &self.settings)?;
                    }
                    self.studio.asset_names = view
                        .model_library
                        .as_ref()
                        .unwrap()
                        .bodies
                        .iter()
                        .map(|b| (b.id.clone(), b.name.clone()))
                        .collect();
                }
                let game = self.build_studio()?;
                let mut campaign = Campaign::new(&self.assets, self.campaign.options, None)?;
                campaign.game = game;
                campaign.options.character = self.studio.project.character;
                campaign.screen = Screen::Playing;
                let debug = crate::debug::Debug::new(&campaign.game)?;
                self.finish_recording();
                self.clear_rewind();
                self.studio_previous = Some(Box::new(Session {
                    campaign: std::mem::replace(&mut self.campaign, campaign),
                    settings: self.settings.clone(),
                    debug: std::mem::replace(&mut self.debug, debug),
                    frontend: self.frontend.take(),
                    paused: self.paused,
                    fault: self.fault.take(),
                    visual: self.view.as_ref().map(View::capture_state),
                    audio: self.audio.as_ref().map(crate::audio::Audio::capture_state),
                    #[cfg(not(target_arch = "wasm32"))]
                    native: self.native_tenchu2.take(),
                    slots: vec![],
                    message_until: None,
                    sounds: Default::default(),
                    preview_project: None,
                    preview_tick: 0,
                    preview_fraction: 0.0,
                    eligible: false,
                    #[cfg(target_arch = "wasm32")]
                    pending_audio: None,
                }));
                self.menu.open = false;
                self.randomizer.open = false;
                self.tenchu2.open = false;
                self.studio_present(false)?;
            }
            Command::Close => {
                self.studio.persist()?;
                let Some(previous) = self.studio_previous.take() else {
                    self.studio.open = false;
                    return Ok(());
                };
                self.campaign = previous.campaign;
                self.settings = previous.settings;
                self.frontend = previous.frontend;
                self.debug = previous.debug;
                self.paused = previous.paused;
                self.fault = previous.fault;
                self.faulted = self.fault.is_some();
                self.studio.open = false;
                self.studio.testing = false;
                if let Some(view) = &mut self.view {
                    view.studio_scenery.clear();
                    view.studio_time = None;
                    view.reload(&self.campaign.game, &self.settings)?;
                    view.settings(&self.settings, &self.campaign.game)?;
                    if let Some(state) = previous.visual {
                        view.restore_state(state, &self.campaign.game);
                    }
                }
                if let Some(audio) = &mut self.audio {
                    audio.reload(&self.assets, &self.campaign)?;
                    audio.restore_playback(&self.campaign, previous.audio);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.native_tenchu2 = previous.native;
                    if let Some(native) = &mut self.native_tenchu2 {
                        native.restore_after_studio(self.audio.as_ref().map(|a| a.output()));
                    }
                }
                self.clock = GameClock::default();
            }
            Command::Rebuild | Command::Play | Command::Return => {
                let playing = matches!(command, Command::Play);
                if self.studio_previous.is_none() {
                    return Err("Open Level Studio first".into());
                }
                self.studio.project.validate()?;
                if playing && !self.prepare_studio_audio()? {
                    #[cfg(target_arch = "wasm32")]
                    {
                        self.studio_previous.as_mut().unwrap().pending_audio = Some(command);
                    }
                    self.studio.notice = "Preparing music for the playtest…".into();
                    return Ok(());
                }
                self.studio.persist()?;
                let game = self.build_studio()?;
                self.campaign.game = game;
                self.campaign.options.character = self.studio.project.character;
                self.campaign.screen = Screen::Playing;
                self.studio_present(playing)?;
                if playing {
                    self.studio.notice.clear();
                    self.studio_tick()?;
                }
            }
            Command::WeatherPreview(seconds) => {
                self.refresh_studio()?;
                if let (Some(view), Some(session)) = (&mut self.view, &self.studio_previous) {
                    view.seek_weather(
                        seconds,
                        session.preview_tick,
                        &self.campaign.game,
                        &self.settings,
                        &self.debug,
                    );
                }
            }
            Command::Preview(action) => self.studio_cue(action, true)?,
            Command::BrowseSounds(game, bank) => {
                let source = self.studio_bank(game, bank)?;
                let mut sounds = vec![];
                for (program, tones) in source
                    .programs
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| p.as_ref().map(|p| (i, &p.tones)))
                {
                    for (tone, data) in tones.iter().enumerate().filter(|(_, t)| t.sample != 0) {
                        let id = (program * 16 + tone) as u16;
                        sounds.push((
                            id,
                            format!(
                                "{id:03} · program {program}, tone {tone} · sample {}",
                                data.sample
                            ),
                        ));
                    }
                }
                self.studio.sound_catalog.insert((game, bank), sounds);
            }
        }
        Ok(())
    }

    fn studio_present(&mut self, playing: bool) -> Result<()> {
        self.settings.graphics = self.studio.project.graphics.clone();
        self.settings.graphics.validate();
        self.settings.retro = false;
        self.settings.rewind_seconds = 0;
        self.settings.show_hud = playing;
        self.settings.player_model = self
            .studio
            .project
            .actors
            .get(&0)
            .cloned()
            .unwrap_or_default();
        self.settings.model_moveset = false;
        self.settings.weapon_model.clear();
        self.settings.voice_model.clear();
        self.reload_game()?;
        self.studio.open = !playing;
        self.studio.testing = playing;
        self.studio.preview_revision = self.studio.revision;
        self.studio.runtime = Runtime::default();
        self.studio_previous.as_mut().unwrap().slots = self.campaign.game.roster.order.clone();
        self.studio_previous.as_mut().unwrap().message_until = None;
        self.studio_previous.as_mut().unwrap().preview_project = Some(self.studio.project.clone());
        self.studio_previous.as_mut().unwrap().preview_tick = 0;
        self.studio_previous.as_mut().unwrap().preview_fraction = 0.0;
        self.paused = !playing;
        self.debug.photo_mode = !playing;
        if playing {
            self.debug.return_camera();
        } else {
            self.debug.camera = Some(self.studio.camera);
        }
        if let Some(view) = &mut self.view {
            view.settings(&self.settings, &self.campaign.game)?;
            view.studio_time = (!playing).then_some(0.0);
            view.prewarm_weather(&self.campaign.game, &self.settings, &self.debug, 2);
        }
        self.studio_previous.as_mut().unwrap().eligible = playing
            && self.studio.project.actors.is_empty()
            && !self.studio.project.events.iter().any(|event| event.enabled)
            && self.trial_records_eligible();
        Ok(())
    }

    pub(super) fn refresh_studio(&mut self) -> Result<()> {
        if !self.studio.open {
            return Ok(());
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(command) = self
            .studio_previous
            .as_mut()
            .and_then(|s| s.pending_audio.take())
        {
            self.apply_studio(command)?;
            if !self.studio.open {
                return Ok(());
            }
        }
        self.debug.camera = Some(self.studio.camera);
        self.settings.graphics = self.studio.project.graphics.clone();
        self.settings.graphics.validate();
        if self.studio.preview_revision != self.studio.revision {
            let current = &self.studio.project;
            let scene_changed = self
                .studio_previous
                .as_ref()
                .and_then(|s| s.preview_project.as_ref())
                .is_none_or(|previous| {
                    previous.mission != current.mission
                        || previous.character != current.character
                        || previous.actors != current.actors
                        || previous.scenery != current.scenery
                });
            self.studio.preview_revision = self.studio.revision;
            if scene_changed {
                self.apply_studio(Command::Rebuild)?;
            }
        }
        if let Some(view) = &mut self.view {
            view.settings(&self.settings, &self.campaign.game)?;
        }
        self.update_view();
        Ok(())
    }

    pub(super) fn advance_studio_preview(&mut self, seconds: f64) {
        if !self.studio.open || !self.focused {
            return;
        }
        let Some(session) = &mut self.studio_previous else {
            return;
        };
        session.preview_fraction += seconds.clamp(0.0, 0.25) * 30.0;
        while session.preview_fraction >= 1.0 {
            session.preview_fraction -= 1.0;
            session.preview_tick += 1;
            if let Some(view) = &mut self.view {
                view.studio_preview_tick(
                    session.preview_tick,
                    &self.campaign.game,
                    &self.settings,
                    &self.debug,
                );
            }
        }
        if let Some(view) = &mut self.view {
            view.studio_time =
                Some((session.preview_tick as f64 + session.preview_fraction) as f32 / 30.0);
        }
    }

    pub(super) fn studio_tick(&mut self) -> Result<()> {
        if !self.studio.testing {
            return Ok(());
        }
        if !self.trial_records_eligible() {
            self.studio_previous.as_mut().unwrap().eligible = false;
        }
        if self.finish_studio_test()? {
            return Ok(());
        }
        let game = &self.campaign.game;
        let session = self.studio_previous.as_mut().unwrap();
        if session.message_until.is_some_and(|tick| tick <= game.tick) {
            self.studio.notice.clear();
            session.message_until = None;
        }
        let defeated: Vec<_> = session
            .slots
            .iter()
            .map(|&id| {
                game.roster
                    .actors
                    .get(id)
                    .and_then(Option::as_ref)
                    .is_none_or(|a| a.health <= 0 && a.max_health >= 0)
            })
            .collect();
        let actions = self.studio.runtime.step(
            &self.studio.project.events,
            Observation {
                tick: game.tick,
                player: game.player().position.to_array(),
                defeated: &defeated,
            },
        );
        for action in actions {
            self.studio_cue(action, false)?;
        }
        self.finish_studio_test()?;
        Ok(())
    }

    fn finish_studio_test(&mut self) -> Result<bool> {
        let game = &self.campaign.game;
        if game.events.outcome == Outcome::Running {
            return Ok(false);
        }
        let mut message = if game.events.outcome == Outcome::Complete {
            "Playtest complete".to_owned()
        } else {
            "Playtest ended".to_owned()
        };
        if let Some(trial) = &game.trial {
            message.push_str(&format!(
                " · {}",
                crate::trial::library::time(trial.elapsed)
            ));
            if game.events.outcome == Outcome::Complete
                && matches!(trial.phase, tenchu_core::game::trial::Phase::Success(_))
            {
                if self.studio_previous.as_ref().unwrap().eligible {
                    if let Some(rank) = self.studio.record(trial.elapsed) {
                        message.push_str(&format!(" · Record #{}", rank + 1));
                    }
                } else {
                    message.push_str(" · Practice: no time record");
                }
            }
        }
        self.apply_studio(Command::Return)?;
        self.studio.notice = message;
        Ok(true)
    }

    fn studio_cue(&mut self, action: CueAction, preview: bool) -> Result<()> {
        match action {
            CueAction::Message { text, ticks } => {
                self.studio.notice = text;
                if let Some(session) = &mut self.studio_previous {
                    session.message_until = Some(self.campaign.game.tick + u64::from(ticks));
                }
            }
            CueAction::Weather(mode) => self.settings.graphics.weather.mode = mode,
            CueAction::TimeOfDay(time) => {
                self.settings.graphics.sky.enabled = true;
                self.settings.graphics.sky.time = time;
            }
            CueAction::Complete => self.campaign.game.events.outcome = Outcome::Complete,
            CueAction::Fail => self.campaign.game.events.outcome = Outcome::GameOver,
            CueAction::Flag { .. } => (),
            action => self.studio_audio(action, preview)?,
        }
        Ok(())
    }
}
