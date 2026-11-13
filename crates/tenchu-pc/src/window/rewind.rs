use super::*;
use std::time::Duration;
mod recording;

#[derive(Default)]
pub(super) struct Playback {
    pub active: bool,
    pub preview: bool,
    pub fraction: f64,
    pub audio: Option<crate::audio::State>,
    pub cpu: Duration,
    pub selection: Option<[u64; 2]>,
    pub forward: bool,
    pub recorded_visuals: Option<bool>,
    pub source_report: Option<String>,
}

impl Application {
    pub(super) fn record_rewind(&mut self, input: Option<crate::rewind::recording::Input>) {
        if self.reverse.active
            || self.reverse.preview
            || self.settings.rewind_seconds == 0
            || self.demo.is_some()
            || self.benchmarks.run.is_some()
            || self.studio_previous.is_some()
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
            self.rewind.push(
                self.campaign.game.tick,
                self.settings.rewind_seconds,
                state,
                input,
            )
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
            self.state_tools_available()?;
            if self.settings.rewind_seconds == 0 && self.rewind.entries.is_empty() {
                return Err("Enable rewind history under F1 → Simulation first".into());
            }
            self.rewind
                .previous(self.campaign.game.tick)
                .ok_or("No earlier rewind state available")?;
            self.debug.frame_step = false;
            self.reverse.active = true;
            self.reverse.forward = false;
            self.reverse.fraction = 0.0;
            self.paused = true;
            self.debug.pause_view = true;
            self.menu.open = false;
            self.finish_recording();
            self.controls.stop_rumble();
            self.release_pointer();
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
        let tick = if self.reverse.forward {
            self.campaign.game.tick.saturating_add(steps)
        } else {
            self.campaign.game.tick.saturating_sub(steps)
        };
        let index = self
            .rewind
            .entries
            .iter()
            .rposition(|e| e.tick <= tick)
            .unwrap_or(0);
        if let Err(error) = self.rewind_to(index) {
            self.stop_rewind();
            self.state_notice(format!("Rewind stopped: {error}"));
        } else if !self.reverse.forward && index == 0
            || self.reverse.forward && index + 1 == self.rewind.entries.len()
        {
            self.stop_rewind();
            self.state_notice("End of timeline playback · F3: resume".into());
        }
    }

    pub(super) fn rewind_to(&mut self, index: usize) -> Result<()> {
        let started = Instant::now();
        let mut history = std::mem::take(&mut self.rewind);
        let context = history
            .entries
            .get(index)
            .and_then(|e| e.input.as_ref())
            .map(|i| (i.camera, i.camera_actor, i.light, i.light_follow));
        let result = history
            .snapshot(index)
            .and_then(|state| self.restore_rewind(state));
        self.rewind = history;
        if result.is_ok() {
            self.reverse.preview = true;
            if self.reverse.recorded_visuals == Some(true) {
                let (camera, actor, light, light_follow) = context.unwrap_or_default();
                self.debug.camera = camera;
                self.debug.camera_actor = actor;
                self.debug.follow_camera = None;
                self.debug.light = light;
                self.debug.light_follow = light_follow;
            }
        }
        self.reverse.cpu += started.elapsed();
        result
    }

    pub(super) fn seek_rewind(&mut self, tick: u64) -> Result<()> {
        if self.rewind.entries.is_empty() {
            return Err("No rewind history available".into());
        }
        let index = self
            .rewind
            .entries
            .iter()
            .rposition(|e| e.tick <= tick)
            .unwrap_or(0);
        self.rewind_to(index)?;
        self.reverse.active = false;
        self.reverse.fraction = 0.0;
        self.debug.frame_step = false;
        self.sync_pause();
        self.update_view();
        Ok(())
    }

    pub(super) fn resume_rewind(&mut self) -> Result<()> {
        if self.reverse.preview && !self.rewind.entries.is_empty() {
            let started = Instant::now();
            let index = self
                .rewind
                .entries
                .iter()
                .position(|e| e.tick == self.campaign.game.tick)
                .ok_or("The selected rewind state is no longer available")?;
            self.rewind.branch(index)?;
            self.reverse.cpu += started.elapsed();
        }
        self.reverse.preview = false;
        self.reverse.recorded_visuals = None;
        self.reverse.source_report = None;
        self.stop_rewind();
        Ok(())
    }

    pub(super) fn clear_rewind(&mut self) {
        self.stop_rewind();
        self.reverse.preview = false;
        self.reverse.selection = None;
        self.reverse.recorded_visuals = None;
        self.reverse.source_report = None;
        self.rewind.clear();
    }
}
