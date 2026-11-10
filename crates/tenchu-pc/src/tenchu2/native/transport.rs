use super::campaign::{Content, Playback};
use crate::audio::output::{Command, Output};
use std::time::{Duration, Instant};
use tenchu_assets::{ReadAt, Result};
use tenchu_core::clock::GameClock;

#[derive(Default)]
pub struct Transport {
    pub recent: crate::diagnostics::Recent<super::diagnostics::Input>,
    clock: GameClock,
    generation: Option<u64>,
    accelerated: bool,
}

#[derive(Clone, Copy)]
pub struct Input {
    pub buttons: u16,
    pub cutscene_speed: u16,
    pub simulation_speed: u16,
    pub fast_forward: bool,
    pub paused: bool,
}

impl Input {
    pub fn pacing(self, content: &Content) -> (f64, bool) {
        if !content.cutscene() && !matches!(content, Content::Gameplay(_)) {
            return (1.0, false);
        }
        let cutscene = if content.cutscene() {
            if self.fast_forward {
                0
            } else {
                self.cutscene_speed
            }
        } else {
            1
        };
        (
            f64::from(self.simulation_speed) * f64::from(cutscene) / 100.0,
            self.simulation_speed == 0 || cutscene != 1,
        )
    }
}

#[derive(Default)]
pub struct Update {
    pub changed: bool,
    pub quit: bool,
}

impl Transport {
    pub fn interpolation(&self, content: &Content, input: Input) -> f32 {
        if input.paused || input.pacing(content).0 == 0.0 {
            1.0
        } else {
            self.clock.interpolation()
        }
    }

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
        super::diagnostics::record(&mut self.recent, playback, buttons);
        let generation = playback.generation;
        let update = playback.step(buttons)?;
        let changed = playback.generation != generation;
        if changed {
            self.recent.reset(playback.generation);
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
        let (speed, muted) = input.pacing(&playback.content);
        self.audio(playback, muted && !input.paused, output);
        if input.paused {
            self.clock.discard_host_backlog();
            return Ok(Update::default());
        }
        if speed == 0.0 {
            self.clock.discard_host_backlog();
        } else {
            self.clock
                .elapse(elapsed.min(Duration::from_millis(250)).mul_f64(speed));
        }
        let deadline = Instant::now() + Duration::from_millis(8);
        for _ in 0..2048 {
            if speed != 0.0 && self.clock.step().is_none() {
                break;
            }
            let update = self.step(playback, input.buttons, output)?;
            if update.changed || update.quit {
                // Discard accumulated time at scene boundaries; the next scene chooses its own rate.
                let (_, accelerated) = input.pacing(&playback.content);
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
