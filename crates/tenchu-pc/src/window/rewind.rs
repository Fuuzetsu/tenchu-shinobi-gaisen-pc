use super::*;
use std::time::Duration;

#[derive(Default)]
pub(super) struct Playback {
    pub active: bool,
    pub fraction: f64,
    pub audio: Option<crate::audio::State>,
    pub cpu: Duration,
}

impl Application {
    pub(super) fn record_rewind(&mut self) {
        if self.reverse.active
            || self.settings.rewind_seconds == 0
            || self.demo.is_some()
            || self.benchmarks.run.is_some()
            || self.trial.open
            || self.randomizer.open
            || !self.campaign.scene_visible()
            || self.faulted
            || !self.rewind.due(self.campaign.game.tick)
        {
            return;
        }
        let started = Instant::now();
        let result = self.capture_rewind().and_then(|state| {
            self.rewind
                .push(self.campaign.game.tick, self.settings.rewind_seconds, state)
        });
        self.reverse.cpu += started.elapsed();
        if let Err(error) = result {
            self.rewind.clear();
            self.settings.rewind_seconds = 0;
            self.state_notice(format!("Rewind disabled: {error}"));
        }
    }

    pub(super) fn toggle_rewind(&mut self) -> Result<()> {
        if self.reverse.active {
            self.stop_rewind();
            self.state_notice("Rewind paused · F3: resume · Ctrl+F12: continue back".into());
        } else {
            if self.settings.rewind_seconds == 0 {
                return Err("Enable rewind history under F1 → Simulation first".into());
            }
            self.rewind
                .previous(self.campaign.game.tick)
                .ok_or("No earlier rewind state available")?;
            self.debug.frame_step = false;
            self.reverse.active = true;
            self.reverse.fraction = 0.0;
            self.paused = true;
            self.debug.pause_view = true;
            self.menu.open = false;
            self.finish_recording();
            self.controls.stop_rumble();
            self.sync_pause();
        }
        self.update_view();
        Ok(())
    }

    pub(super) fn stop_rewind(&mut self) {
        self.reverse.active = false;
        self.reverse.fraction = 0.0;
        if let Some(state) = self.reverse.audio.take()
            && let Some(audio) = &mut self.audio
        {
            audio.restore_playback(&self.campaign, Some(state));
        }
        self.sync_audio_pause();
    }

    pub(super) fn advance_rewind(&mut self, elapsed: Duration) {
        if !self.focused && self.settings.pause_on_defocus
            || self.menu.open && self.settings.pause_in_menu
        {
            return;
        }
        self.reverse.fraction +=
            elapsed.as_secs_f64().min(0.1) * 30.0 * f64::from(self.settings.rewind_speed) / 100.0;
        let steps = self.reverse.fraction as u64;
        if steps == 0 {
            return;
        }
        self.reverse.fraction -= steps as f64;
        let tick = self.campaign.game.tick.saturating_sub(steps);
        let index = self
            .rewind
            .entries
            .iter()
            .rposition(|e| e.tick <= tick)
            .unwrap_or(0);
        if let Err(error) = self.rewind_to(index) {
            self.stop_rewind();
            self.state_notice(format!("Rewind stopped: {error}"));
        } else if index == 0 {
            self.stop_rewind();
            self.state_notice("Start of rewind history · F3: resume".into());
        }
    }

    pub(super) fn rewind_to(&mut self, index: usize) -> Result<()> {
        let started = Instant::now();
        let mut history = std::mem::take(&mut self.rewind);
        let result = history
            .snapshot(index)
            .and_then(|state| self.restore_rewind(state))
            .and_then(|()| history.branch(index));
        self.rewind = history;
        self.reverse.cpu += started.elapsed();
        result
    }
}
