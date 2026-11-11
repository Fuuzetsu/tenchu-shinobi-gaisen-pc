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
    menu_pending: u16,
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

#[derive(Clone, Copy)]
pub struct Observation {
    pub replaced: bool,
    pub buttons: u16,
    pub navigation: Option<super::NavigationInput>,
    pub bandage: bool,
    pub observing_death: bool,
}

impl Transport {
    pub fn discard_backlog(&mut self) {
        self.clock.discard_host_backlog();
    }

    pub fn restored(&mut self, playback: &mut Playback<impl ReadAt>, output: Option<&Output>) {
        *self = Self::default();
        self.audio(playback, false, output);
    }

    pub fn preview(&mut self, playback: &mut Playback<impl ReadAt>, output: Option<&Output>) {
        self.clock.discard_host_backlog();
        self.menu_pending = 0;
        self.audio(playback, true, output);
    }

    pub fn interpolation(&self, playback: &Playback<impl ReadAt>, input: Input) -> f32 {
        if input.paused || input.pacing(&playback.content).0 == 0.0 || playback.death.frozen() {
            1.0
        } else {
            self.clock.interpolation()
        }
    }

    fn audio(
        &mut self,
        playback: &mut Playback<impl ReadAt>,
        accelerated: bool,
        output: Option<&Output>,
    ) {
        let restart =
            self.generation != Some(playback.generation) || self.accelerated != accelerated;
        if restart && let Some(output) = output {
            if accelerated {
                output.send(Command::Clear);
                output.send(Command::Music(None, false));
            } else {
                playback.content.resume_audio(output);
            }
        }
        if let Content::Gameplay(session) = &mut playback.content
            && let Some(scene) = &mut session.gaisen
        {
            scene.audio.synchronize(output, accelerated, restart);
        }
        self.generation = Some(playback.generation);
        self.accelerated = accelerated;
    }

    fn step<S: ReadAt>(
        &mut self,
        playback: &mut Playback<S>,
        buttons: u16,
        output: Option<&Output>,
        observed: &mut impl FnMut(&Playback<S>, Observation) -> Result<()>,
    ) -> Result<Update> {
        super::diagnostics::record(&mut self.recent, playback, buttons);
        let generation = playback.generation;
        let cutscene = playback.content.cutscene();
        let mut observation = Observation {
            replaced: false,
            buttons,
            navigation: None,
            bandage: false,
            observing_death: playback.death.waiting(),
        };
        if let Content::Gameplay(session) = &playback.content {
            observation.navigation = session.navigation;
            observation.bandage = session.world.actors[session.world.player]
                .actor
                .bleeding
                .as_ref()
                .is_some_and(|state| state.bandage_requested());
        }
        let update = playback.step(buttons)?;
        let replaced = playback.generation != generation;
        let changed = replaced || playback.content.cutscene() != cutscene;
        observation.replaced = replaced;
        if replaced {
            self.recent.reset(playback.generation);
        }
        observed(playback, observation)?;
        if !replaced
            && !self.accelerated
            && let Some(output) = output
        {
            update.play_audio(output);
        }
        if changed {
            self.clock.discard_host_backlog();
            self.menu_pending = 0;
        }
        Ok(Update {
            changed,
            quit: update.quit,
        })
    }

    pub fn frame_step<S: ReadAt>(
        &mut self,
        playback: &mut Playback<S>,
        buttons: u16,
        output: Option<&Output>,
        mut observed: impl FnMut(&Playback<S>, Observation) -> Result<()>,
    ) -> Result<Update> {
        self.audio(playback, false, output);
        self.menu_pending = 0;
        let update = self.step(playback, buttons, output, &mut observed)?;
        playback.content.clear_feedback();
        self.audio(playback, false, output);
        self.clock.discard_host_backlog();
        Ok(update)
    }

    pub fn advance<S: ReadAt>(
        &mut self,
        playback: &mut Playback<S>,
        elapsed: Duration,
        input: Input,
        output: Option<&Output>,
        mut observed: impl FnMut(&Playback<S>, Observation) -> Result<()>,
    ) -> Result<Update> {
        let (speed, muted) = input.pacing(&playback.content);
        self.audio(playback, muted && !input.paused, output);
        if input.paused
            || playback.death.frozen() && playback.death.enabled() && playback.action.is_none()
        {
            self.clock.discard_host_backlog();
            self.menu_pending = 0;
            playback.content.clear_feedback();
            return Ok(Update::default());
        }
        if playback.content.menu_controls() {
            self.menu_pending |= input.buttons;
        } else {
            self.menu_pending = 0;
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
            let buttons = input.buttons | std::mem::take(&mut self.menu_pending);
            let update = self.step(playback, buttons, output, &mut observed)?;
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
