use super::*;
use crate::level_editor::{Action as CueAction, AssetGame, Command, Cue, Event};

impl Application {
    pub(super) fn start_studio_demo(&mut self, feature: demo::studio::Feature) -> Result<()> {
        if self.studio_previous.is_none() {
            self.settings = Settings {
                volume: self.settings.volume,
                model_directory: self.settings.model_directory.clone(),
                pause_on_defocus: false,
                ..Default::default()
            };
            self.studio = crate::level_editor::Editor::for_showcase();
            let player = self.demo.take().unwrap();
            let entered = self.apply_studio(Command::Open);
            self.demo = Some(player);
            entered?;
        }
        self.studio.load_showcase(&self.assets)?;
        let mut sounds = vec![];
        for game in [AssetGame::Tenchu, AssetGame::Tenchu2] {
            self.apply_studio(Command::BrowseSounds(game, 0))?;
            let bank = self.studio_bank(game, 0)?;
            let id = bank
                .programs
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.as_ref().map(|p| (i, p)))
                .flat_map(|(program, p)| {
                    p.tones
                        .iter()
                        .enumerate()
                        .map(move |(tone, t)| (program * 16 + tone, t))
                })
                .find(|(_, t)| {
                    t.sample != 0
                        && bank.samples.get(t.sample).is_some_and(|s| {
                            s.loop_start.is_none()
                                && (2205..=44_100).contains(&s.frames.len())
                                && s.frames.iter().any(|v| v.unsigned_abs() > 256)
                        })
                })
                .map(|(id, _)| id as u16)
                .ok_or("Showcase needs a short non-looping sound effect")?;
            sounds.push(Cue {
                delay: 30,
                action: CueAction::Sound {
                    game,
                    bank: 0,
                    id,
                    volume: 100,
                    pan: 64,
                },
            });
        }
        self.studio.project.events.push(Event {
            name: "T1 then T2 sound cues".into(),
            cues: sounds,
            ..Default::default()
        });
        self.studio.prepare_showcase(feature);
        self.apply_studio(Command::Rebuild)?;
        self.settings.pause_on_defocus = false;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(sound) = self.video.as_mut().and_then(|v| v.sound.as_mut()) {
            sound.studio_music(None);
            sound.reload(&self.assets, &self.campaign.game, None)?;
        }
        if feature == demo::studio::Feature::Playtest {
            self.apply_studio(Command::Play)?;
        }
        self.paused = false;
        self.faulted = false;
        self.fault = None;
        self.controls.clear();
        let player = self.demo.as_ref().unwrap();
        println!(
            "Studio {}/{}: {} — {}",
            player.index + 1,
            player.playlist.len(),
            player.clip().id,
            player.clip().title
        );
        self.last_frame = Instant::now();
        self.update_view();
        Ok(())
    }

    pub(super) fn advance_studio_demo(&mut self, ticks: usize) -> Result<()> {
        let player = self.demo.as_ref().unwrap();
        let Feature::Studio(feature) = player.clip().feature else {
            return Ok(());
        };
        let frame = player.frame;
        let total = u64::from(player.clip().seconds) * 60;
        self.studio.showcase_frame(feature, frame, total)?;
        if feature == demo::studio::Feature::Weather
            && (frame == total / 4 || frame == total * 65 / 100)
        {
            self.apply_studio(Command::WeatherPreview(120))?;
        }
        if feature == demo::studio::Feature::Sounds && (frame == total / 4 || frame == total / 2) {
            let cue = self.studio.project.events.last().unwrap().cues
                [usize::from(frame == total / 2)]
            .action
            .clone();
            self.apply_studio(Command::Preview(cue))?;
        }
        if self.studio.testing {
            for _ in 0..ticks {
                if !self.game_tick(0) {
                    break;
                }
            }
            if frame == total * 6 / 7 {
                self.apply_studio(Command::Return)?;
            }
        }
        if self.studio.open {
            self.advance_studio_preview(1.0 / 60.0);
            self.refresh_studio()?;
            self.paused = false;
        }
        Ok(())
    }
}
