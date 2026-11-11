use super::*;
use std::time::Duration;

impl Application {
    pub(super) fn simulating(&self) -> bool {
        self.campaign.playing()
            || (matches!(self.campaign.screen, Screen::DeathHold { .. })
                && self.settings.wait_after_death
                && self.settings.live_death_scene
                && self.campaign.game.death_wait_ticks() == Some(1))
    }

    pub(super) fn game_tick(&mut self, buttons: u16) -> bool {
        if (self.reverse.active || self.reverse.preview)
            && let Err(error) = self.resume_rewind()
        {
            self.pause_error(error);
            return false;
        }
        let observing = matches!(self.campaign.screen, Screen::DeathHold { .. });
        let recorded_input =
            (self.settings.rewind_seconds != 0).then(|| crate::rewind::recording::Input {
                buttons,
                bandage: self
                    .campaign
                    .game
                    .player()
                    .bleeding
                    .as_ref()
                    .is_some_and(|s| s.bandage_requested()),
                observing_death: observing,
                navigation: self.campaign.game.navigation,
                walking: self.campaign.game.walking,
                sheathing: self.campaign.game.player().sheathing.clone(),
                camera: self.debug.camera,
                camera_actor: self.debug.camera_actor,
                light: self.debug.light,
                light_follow: self.debug.light_follow,
                native: None,
            });
        let result = if observing {
            if !self.simulating() {
                return false;
            }
            self.campaign.game.observe_death()
        } else {
            self.recent_inputs.record(&self.campaign.game, buttons);
            self.record_input(buttons);
            self.campaign.tick(buttons)
        };
        if let Err(error) = result {
            self.pause_error(error);
            self.clock.discard_host_backlog();
            return false;
        }
        if self.settings.wait_after_death
            && self.demo.is_none()
            && (matches!(self.campaign.screen, Screen::GameOver)
                || self.campaign.playing() && self.campaign.game.death_wait_ticks() == Some(1))
        {
            self.campaign.screen = Screen::DeathHold {
                retry_seed: self.campaign.game.random.seed,
            };
        }
        if self.settings.graphics.interpolation
            && let Some(view) = &mut self.view
        {
            view.interpolation.tick(&self.campaign.game);
        }
        if let Some(view) = &mut self.view {
            view.cosmetic_tick(&self.campaign.game, &self.settings, &self.debug);
        }
        self.debug.update(&self.campaign.game);
        if self.orbit_enabled() {
            self.observe_orbit();
        }
        self.controls.feedback(
            &self.campaign.game,
            self.settings.frontend.rumble
                && self.focused
                && self.campaign.playing()
                && self.demo.is_none(),
            &self.settings.vibration,
        );
        let weather_settings = &self.settings.rendering().graphics.weather;
        let storm = weather_settings.mode.storm() && self.simulating();
        let ambience = if self.simulating() {
            self.view
                .as_ref()
                .map_or_else(Default::default, |view| view.ambience(weather_settings))
        } else {
            Default::default()
        };
        let death = self
            .view
            .as_ref()
            .and_then(View::death_frame)
            .filter(|_| !self.settings.retro);
        let blood_sounds = || {
            self.view
                .as_ref()
                .into_iter()
                .flat_map(|view| view.blood.sounds(self.campaign.game.tick))
        };
        if let Some(audio) = &mut self.audio {
            if let Err(error) = audio.tick(
                &self.campaign.game,
                death,
                self.settings.burning_sound,
                blood_sounds(),
            ) {
                eprintln!("Audio: {error}");
            }
            audio.screen(&self.campaign.screen);
            audio.weather(self.view.as_ref().and_then(View::thunder), storm, ambience);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(sound) = self.video.as_mut().and_then(|v| v.sound.as_mut()) {
            if let Err(error) = sound.tick(
                &self.assets,
                &self.campaign.game,
                death,
                self.settings.burning_sound,
                blood_sounds(),
            ) {
                self.pause_error(error);
                return false;
            }
            sound.weather(self.view.as_ref().and_then(View::thunder), storm, ambience);
        }
        self.trial_record_check();
        self.record_rewind(recorded_input);
        if !self.campaign.playing() && !observing {
            self.finish_recording();
            self.controls.clear();
            self.clock.discard_host_backlog();
            return false;
        }
        true
    }

    pub(super) fn advance_simulation(&mut self, elapsed: Duration) {
        if self.reverse.active {
            self.advance_rewind(elapsed);
            return;
        }
        if std::mem::take(&mut self.debug.frame_step) {
            if self.focused && self.simulating() && !self.faulted {
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
        if suspended || !self.simulating() {
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
            let buttons = if self.campaign.playing()
                && self.focused
                && !self.menu.open
                && self.debug.camera.is_none()
            {
                self.player_input()
            } else {
                if let Some(walk) = &mut self.campaign.game.walking {
                    walk.held = false;
                }
                let player = self
                    .campaign
                    .game
                    .roster
                    .get_mut(self.campaign.game.player_id);
                if let Some(state) = &mut player.bleeding {
                    state.input(false);
                }
                if let Some(sheathing) = &mut player.sheathing {
                    sheathing.input(false);
                }
                0
            };
            let buttons = self
                .frontend
                .as_ref()
                .map_or(buttons, |f| f.buttons(buttons));
            let buttons = self.navigation_input(buttons);
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
