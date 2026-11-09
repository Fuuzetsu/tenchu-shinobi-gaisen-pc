use super::campaign::Playback;
use crate::audio::output::{Command, Output};
use std::time::{Duration, Instant};
use tenchu_assets::{ReadAt, Result};
use tenchu_core::clock::GameClock;

pub fn speed_control(ui: &mut egui::Ui, speed: &mut u16) {
    egui::ComboBox::from_label("Auto cutscene speed")
        .selected_text(if *speed == 0 {
            "Unlimited".into()
        } else {
            format!("{speed}×")
        })
        .show_ui(ui, |ui| {
            for value in [1, 2, 4, 8, 16, 0] {
                ui.selectable_value(
                    speed,
                    value,
                    if value == 0 {
                        "Unlimited".into()
                    } else {
                        format!("{value}×")
                    },
                );
            }
        });
    ui.small("Hold Backspace to fast-forward cutscenes. Audio is muted while accelerated and resumes at the matching position. Gameplay and menus stay at normal speed.");
}

#[derive(Default)]
pub struct Transport {
    clock: GameClock,
    generation: Option<u64>,
    accelerated: bool,
}

#[derive(Clone, Copy)]
pub struct Input {
    pub buttons: u16,
    pub speed: u16,
    pub fast_forward: bool,
    pub paused: bool,
}

#[derive(Default)]
pub struct Update {
    pub changed: bool,
    pub quit: bool,
}

impl Transport {
    fn audio(
        &mut self,
        playback: &Playback<impl ReadAt>,
        accelerated: bool,
        output: Option<&Output>,
    ) {
        if self.generation == Some(playback.generation) && self.accelerated == accelerated {
            return;
        }
        if let Some(output) = output {
            if accelerated {
                output.send(Command::Clear);
                output.send(Command::Music(None, false));
            } else {
                playback.content.resume_audio(output);
            }
        }
        self.generation = Some(playback.generation);
        self.accelerated = accelerated;
    }

    pub fn step(
        &mut self,
        playback: &mut Playback<impl ReadAt>,
        buttons: u16,
        output: Option<&Output>,
    ) -> Result<Update> {
        let generation = playback.generation;
        let update = playback.step(buttons)?;
        let changed = playback.generation != generation;
        if changed {
            self.clock.discard_host_backlog();
        } else if !self.accelerated
            && let Some(output) = output
        {
            update.play_audio(output);
        }
        Ok(Update {
            changed,
            quit: update.quit,
        })
    }

    pub fn frame_step(
        &mut self,
        playback: &mut Playback<impl ReadAt>,
        buttons: u16,
        output: Option<&Output>,
    ) -> Result<Update> {
        self.audio(playback, false, output);
        let update = self.step(playback, buttons, output)?;
        self.audio(playback, false, output);
        self.clock.discard_host_backlog();
        Ok(update)
    }

    pub fn advance(
        &mut self,
        playback: &mut Playback<impl ReadAt>,
        elapsed: Duration,
        input: Input,
        output: Option<&Output>,
    ) -> Result<Update> {
        let speed = if playback.content.cutscene() {
            if input.fast_forward { 0 } else { input.speed }
        } else {
            1
        };
        self.audio(playback, speed != 1 && !input.paused, output);
        if input.paused {
            self.clock.discard_host_backlog();
            return Ok(Update::default());
        }
        if speed == 0 {
            self.clock.discard_host_backlog();
        } else {
            self.clock
                .elapse(elapsed.min(Duration::from_millis(250)) * u32::from(speed));
        }
        let deadline = Instant::now() + Duration::from_millis(8);
        for _ in 0..2048 {
            if speed != 0 && self.clock.step().is_none() {
                break;
            }
            let update = self.step(playback, input.buttons, output)?;
            if update.changed || update.quit {
                // Stop at each content boundary so accelerated time never enters gameplay or menus.
                let accelerated =
                    playback.content.cutscene() && (input.fast_forward || input.speed != 1);
                self.audio(playback, accelerated, output);
                return Ok(update);
            }
            if Instant::now() >= deadline {
                break;
            }
        }
        Ok(Update::default())
    }
}

#[cfg(test)]
mod tests;
