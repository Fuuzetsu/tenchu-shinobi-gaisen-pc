use super::*;
use std::time::Duration;

impl Application {
    pub(super) fn game_tick(&mut self, buttons: u16) -> bool {
        self.record_input(buttons);
        if let Err(error) = self.campaign.tick(buttons) {
            self.pause_error(error);
            self.clock.discard_host_backlog();
            return false;
        }
        if self.settings.graphics.interpolation
            && let Some(view) = &mut self.view
        {
            view.interpolation.tick(&self.campaign.game);
        }
        self.debug.update(&self.campaign.game);
        self.controls.feedback(
            &self.campaign.game,
            self.settings.frontend.rumble
                && self.focused
                && self.campaign.playing()
                && self.demo.is_none(),
        );
        if let Some(audio) = &mut self.audio {
            if let Err(error) = audio.tick(&self.campaign.game) {
                eprintln!("Audio: {error}");
            }
            audio.screen(&self.campaign.screen);
        }
        if !self.campaign.playing() {
            self.finish_recording();
            self.controls.clear();
            self.clock.discard_host_backlog();
            return false;
        }
        true
    }

    pub(super) fn advance_simulation(&mut self, elapsed: Duration) {
        if std::mem::take(&mut self.debug.frame_step) {
            if self.focused && self.campaign.playing() && !self.faulted {
                self.game_tick(0);
                self.save();
            }
            return;
        }
        let cinematic = self.campaign.game.cinematic.active;
        let cutscene_speed = if cinematic {
            if self.controls.held(KeyCode::Backspace) {
                0
            } else {
                self.settings.cutscene_speed
            }
        } else {
            1
        };
        let unlimited_simulation = self.debug.speed == 0;
        let accelerated = cinematic && cutscene_speed != 1;
        let suspended = self.is_suspended();
        if let Some(audio) = &mut self.audio {
            audio.fast_forward((accelerated || unlimited_simulation) && !suspended);
        }
        if suspended || !self.campaign.playing() {
            return;
        }
        let unlimited = unlimited_simulation || cutscene_speed == 0;
        if unlimited {
            self.clock.discard_host_backlog();
        } else {
            self.clock.elapse(
                elapsed.mul_f64(f64::from(self.debug.speed) * f64::from(cutscene_speed) / 100.0),
            );
        }
        let deadline = Instant::now() + Duration::from_millis(8);
        for _ in 0..2048 {
            if !unlimited && self.clock.step().is_none() {
                break;
            }
            let buttons = if self.focused && !self.menu.open && self.debug.camera.is_none() {
                self.controls.buttons()
            } else {
                0
            };
            let buttons = self
                .frontend
                .as_ref()
                .map_or(buttons, |f| f.buttons(buttons));
            if !self.game_tick(buttons) {
                break;
            }
            if self.campaign.game.cinematic.active != cinematic
                && !unlimited_simulation
                && (accelerated
                    || self.settings.cutscene_speed != 1
                    || self.controls.held(KeyCode::Backspace))
            {
                // A cutscene's accumulated fast-forward time never spills into gameplay.
                if accelerated || unlimited {
                    self.clock.discard_host_backlog();
                }
                break;
            }
            if Instant::now() >= deadline {
                break;
            }
        }
        if accelerated
            && !unlimited_simulation
            && !self.campaign.game.cinematic.active
            && let Some(audio) = &mut self.audio
        {
            audio.fast_forward(false);
        }
        self.save();
    }
}
