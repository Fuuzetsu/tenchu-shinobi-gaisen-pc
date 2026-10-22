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

pub struct Audio {
    output: output::Output,
    loader: loader::Loader,
    catalog: Catalog,
    screen: Option<Discriminant<Screen>>,
    paused: bool,
    trace: bool,
    fast_forward: bool,
    volume: u16,
    current_track: Option<Track>,
}

impl Audio {
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
            trace: std::env::var_os("TENCHU_AUDIO_TRACE").is_some(),
            fast_forward: false,
            volume: 100,
            current_track: None,
        };
        audio.reload(assets, &campaign.game)?;
        audio.screen(&campaign.screen);
        Ok(audio)
    }

    pub fn reload(&mut self, assets: &GameAssets<DiscSource>, game: &Game) -> Result<()> {
        let bank = Bank::load(
            assets,
            game.mission.stage.id,
            game.player().kind,
            game.mission.scripts.language,
        )?;
        self.catalog = Catalog::load(&assets.executable, game.mission.scripts.language)?;
        self.track(None);
        self.output.send(output::Command::Bank(Arc::new(bank)));
        self.screen = None;
        Ok(())
    }

    fn track(&mut self, track: Option<Track>) {
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

    pub fn tick(&mut self, game: &Game) -> Result<()> {
        if !self.fast_forward {
            for sound in game.sounds()? {
                self.output.send(output::Command::Effect(sound));
            }
        }
        for cue in &game.music {
            if self.trace {
                eprintln!("Audio tick {}: {cue:?}", game.tick);
            }
            self.track(cue.and_then(|cue| self.catalog.event(cue)).cloned());
        }
        Ok(())
    }

    pub fn screen(&mut self, screen: &Screen) {
        if !matches!(screen, Screen::Playing) {
            self.fast_forward(false);
        }
        if self.screen == Some(discriminant(screen)) {
            return;
        }
        self.screen = Some(discriminant(screen));
        self.output.send(output::Command::Clear);
        let cue = match screen {
            Screen::Launcher | Screen::Inventory(_) => Some(15),
            Screen::Results(_) | Screen::Complete => Some(12),
            Screen::GameOver => Some(11),
            Screen::Playing => None,
            Screen::Frontend | Screen::Ending(_) => None,
        };
        self.track(cue.and_then(|cue| self.catalog.music(cue)).cloned());
    }

    pub fn pause(&mut self, paused: bool) {
        if self.paused != paused {
            self.paused = paused;
            self.output.send(output::Command::Pause(paused));
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
        if let Some(result) = self.loader.poll() {
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
                }
                Err(error) => eprintln!("Audio: {error}"),
            }
        }
    }
}
