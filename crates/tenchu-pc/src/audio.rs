#[cfg_attr(target_arch = "wasm32", path = "web/audio_loader.rs")]
mod loader;
mod output;
use output::Command;

use crate::{
    campaign::{Campaign, Screen},
    source::DiscSource,
};
use std::{
    mem::{Discriminant, discriminant},
    path::Path,
    sync::Arc,
};
use tenchu_assets::{
    Result,
    audio::{bank::Bank, catalog::Catalog, stream::Track},
    game::GameAssets,
};
use tenchu_core::{audio::Sound, game::Game};
use web_time::Instant;

#[cfg(test)]
mod tests;

pub struct Audio {
    output: output::Output,
    loader: loader::Loader,
    catalog: Catalog,
    screen: Option<Discriminant<Screen>>,
    paused: bool,
    music_while_paused: bool,
    trace: bool,
    fast_forward: bool,
    volume: u16,
    mission_track: Option<Track>,
    current_track: Option<Track>,
    ready_track: Option<Track>,
    playhead: f64,
    playhead_updated: Instant,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    current: Option<Track>,
    mission: Option<Track>,
    playhead: f64,
}

impl Audio {
    pub fn capture_state(&self) -> State {
        State {
            current: self.current_track.clone(),
            mission: self.mission_track.clone(),
            playhead: self.music_position(),
        }
    }

    pub fn restore_state(
        &mut self,
        campaign: &Campaign,
        catalog: Catalog,
        bank: Bank,
        state: Option<State>,
    ) {
        self.catalog = catalog;
        self.output.send(Command::Clear);
        self.output.send(Command::Bank(Arc::new(bank)));
        self.restore_playback(campaign, state);
    }

    pub fn restore_playback(&mut self, campaign: &Campaign, state: Option<State>) {
        self.output.send(Command::Clear);
        self.screen = Some(discriminant(&campaign.screen));
        self.fast_forward = false;
        self.volume(self.volume);
        self.mission_track = state.as_ref().and_then(|s| s.mission.clone());
        let playhead = state.as_ref().map_or(0.0, |s| s.playhead);
        self.track(state.and_then(|s| s.current));
        self.playhead = if playhead.is_finite() {
            playhead.max(0.0)
        } else {
            0.0
        };
    }

    fn music_position(&self) -> f64 {
        self.playhead
            + if (!self.paused || self.music_while_paused)
                && !self.fast_forward
                && self.ready_track.is_some()
            {
                self.playhead_updated.elapsed().as_secs_f64()
            } else {
                0.0
            }
    }

    fn update_playhead(&mut self) {
        self.playhead = self.music_position();
        self.playhead_updated = Instant::now();
    }

    pub fn preferences(&self, preferences: &crate::frontend::Preferences) {
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::movie_volume(self.volume, preferences.music);
            crate::web::movie_stereo(preferences.stereo);
        }
        self.output.send(Command::Balance(
            preferences.music,
            preferences.effects,
            preferences.stereo,
        ));
    }

    pub fn frontend(&mut self, event: crate::frontend::Event) {
        use crate::frontend::Event;
        self.fast_forward(false);
        match event {
            Event::Bank(bank) => {
                self.output.send(Command::Bank(bank));
                self.screen = None;
            }
            Event::Effect(id) => self.effect(id),
            Event::Voice(id) => self.output.send(Command::Voice(id)),
            Event::Music(index) => {
                let track = index
                    .and_then(|i| self.catalog.music(i as i32).cloned())
                    .map(|mut track| {
                        track.repeat = index != Some(14);
                        track
                    });
                self.track(track);
                self.output.send(Command::Clear);
            }
            Event::MovieAudio(clip) => {
                self.loader.request(None);
                self.output.send(Command::Music(Some(clip), false));
            }
            Event::Narration(track) => self.track(Some(track)),
            _ => (),
        }
    }
    pub fn new(path: &Path, assets: &GameAssets<DiscSource>, campaign: &Campaign) -> Result<Self> {
        let mut audio = Self {
            output: output::Output::new()?,
            loader: loader::Loader::new(path)?,
            catalog: Catalog::load(&assets.executable, campaign.game.mission.scripts.language)?,
            screen: None,
            paused: false,
            music_while_paused: false,
            trace: std::env::var_os("TENCHU_AUDIO_TRACE").is_some(),
            fast_forward: false,
            volume: 100,
            mission_track: None,
            current_track: None,
            ready_track: None,
            playhead: 0.0,
            playhead_updated: Instant::now(),
        };
        audio.reload(assets, &campaign.game)?;
        audio.screen(&campaign.screen);
        Ok(audio)
    }

    pub fn refresh_actor_bank(
        &mut self,
        assets: &GameAssets<DiscSource>,
        game: &Game,
    ) -> Result<()> {
        self.output
            .send(Command::Bank(Arc::new(game_bank(assets, game)?)));
        Ok(())
    }

    pub fn reload(&mut self, assets: &GameAssets<DiscSource>, game: &Game) -> Result<()> {
        let bank = game_bank(assets, game)?;
        self.catalog = Catalog::load(&assets.executable, game.mission.scripts.language)?;
        self.mission_track = None;
        self.track(None);
        self.output.send(output::Command::Bank(Arc::new(bank)));
        self.screen = None;
        Ok(())
    }

    pub fn music(&mut self, cue: u8) {
        self.track(self.catalog.music(i32::from(cue)).cloned());
    }

    fn track(&mut self, track: Option<Track>) {
        self.playhead = 0.0;
        self.playhead_updated = Instant::now();
        self.ready_track = None;
        self.current_track = track.clone();
        if self.fast_forward {
            return;
        }
        if self.trace {
            eprintln!("Audio request: {track:?}");
        }
        self.output.send(output::Command::Music(None, false));
        self.loader.request(track);
    }

    pub fn effect(&self, id: i16) {
        self.output.send(output::Command::Effect(Sound::local(id)));
    }

    pub fn pause_effect(&self, paused: bool) {
        self.output
            .send(Command::UiEffect(if paused { 9 } else { 10 }));
    }

    pub fn weather(&self, cue: Option<tenchu_audio::ThunderCue>, storm: bool) {
        if !storm || self.fast_forward {
            self.output.send(Command::Thunder(None));
        } else if cue.is_some() {
            self.output.send(Command::Thunder(cue));
        }
    }

    pub fn tick(
        &mut self,
        game: &Game,
        frame: Option<&crate::visuals::interpolation::Frame>,
    ) -> Result<()> {
        if !self.fast_forward {
            for sound in game
                .sounds_with_effects(crate::visuals::death::blood::sounds(&game.effects, frame))?
            {
                self.output.send(output::Command::Effect(sound));
            }
        }
        if let Some((index, repeat)) = game.trial.as_ref().and_then(|s| s.music) {
            let track = self.catalog.music(index as i32).cloned().map(|mut t| {
                t.repeat = repeat;
                t
            });
            self.track(track);
        }
        for cue in &game.music {
            if self.trace {
                eprintln!("Audio tick {}: {cue:?}", game.tick);
            }
            let track = cue.and_then(|cue| self.catalog.event(cue)).cloned();
            if track.as_ref().is_some_and(|track| track.repeat) {
                self.mission_track = track.clone();
            }
            self.track(track);
        }
        Ok(())
    }

    pub fn screen(&mut self, screen: &Screen) {
        if !matches!(screen, Screen::Playing | Screen::DeathHold { .. }) {
            self.fast_forward(false);
        }
        if self.screen == Some(discriminant(screen)) {
            return;
        }
        self.screen = Some(discriminant(screen));
        if matches!(screen, Screen::DeathHold { .. }) {
            if self.current_track != self.mission_track {
                self.track(self.mission_track.clone());
            }
            return;
        }
        self.output.send(output::Command::Clear);
        let cue = match screen {
            Screen::Launcher => Some(15),
            Screen::Results(_) | Screen::Complete => Some(12),
            Screen::GameOver => Some(11),
            Screen::Playing => None,
            Screen::Frontend
            | Screen::Ending(_)
            | Screen::Inventory(_)
            | Screen::DeathHold { .. } => None,
        };
        self.track(cue.and_then(|cue| self.catalog.music(cue)).cloned());
    }

    pub fn pause(&mut self, paused: bool, music_while_paused: bool) {
        if self.paused != paused || self.music_while_paused != music_while_paused {
            self.update_playhead();
            self.paused = paused;
            self.music_while_paused = music_while_paused;
            self.output.send(Command::Pause {
                paused,
                music_while_paused,
            });
        }
    }

    pub fn volume(&mut self, percent: u16) {
        self.volume = percent;
        #[cfg(target_arch = "wasm32")]
        crate::web::movie_volume(percent, 255);
        self.output
            .send(output::Command::Volume(if self.fast_forward {
                0
            } else {
                percent
            }));
    }

    pub fn fast_forward(&mut self, enabled: bool) {
        if self.fast_forward == enabled {
            return;
        }
        self.update_playhead();
        self.fast_forward = enabled;
        self.output.send(output::Command::Clear);
        if enabled {
            self.loader.request(None);
            self.output.send(output::Command::Music(None, false));
        } else {
            let music = self.current_track.clone().filter(|track| track.repeat);
            self.track(music);
        }
        self.volume(self.volume);
    }

    pub fn poll(&mut self) {
        self.update_playhead();
        if let Some(result) = self.loader.poll() {
            self.ready_track = self.current_track.clone();
            match result {
                Ok((clip, repeat)) => {
                    if self.trace {
                        eprintln!(
                            "Audio ready: {} frames at {} Hz, repeat {repeat}",
                            clip.frames.len(),
                            clip.rate
                        );
                    }
                    self.output.send(output::Command::Music(Some(clip), repeat));
                    self.output.send(output::Command::SeekMusic(self.playhead));
                }
                Err(error) => eprintln!("Audio: {error}"),
            }
        }
    }

    pub fn track_ready(&self, track: &Track) -> bool {
        self.ready_track.as_ref() == Some(track)
    }
}

pub(super) fn game_bank(assets: &GameAssets<DiscSource>, game: &Game) -> Result<Bank> {
    let mut bank = if let Some(trial) = &game.mission.trial {
        Bank::parse(&assets.asset(&format!(
            "SOUND/TRIAL/TRIAL{}{}.VAB",
            trial.mission.theme + 1,
            if game.player().kind == 0 { 'R' } else { 'A' }
        ))?)?
    } else {
        Bank::load(
            assets,
            game.mission.stage.id,
            game.player().kind,
            game.mission.scripts.language,
        )?
    };
    let mut sources = std::collections::BTreeMap::new();
    for source in &game.mission.imported_voices {
        let donor = match sources.entry((source.stage, source.character)) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => entry.insert(Bank::load(
                assets,
                source.stage,
                source.character,
                game.mission.scripts.language,
            )?),
        };
        bank.import_program(donor, source.program)?;
    }
    Ok(bank)
}
