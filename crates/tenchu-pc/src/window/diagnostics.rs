use super::*;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct Input {
    tick_before: u64,
    clock_before: i32,
    rng_before: u32,
    buttons: u16,
    player_position: [i32; 3],
    player_health: i16,
    alert: i32,
    strain: i32,
}

#[derive(Default, Serialize)]
pub(super) struct RecentInputs {
    stage_id: usize,
    #[serde(flatten)]
    recent: crate::diagnostics::Recent<Input>,
}

impl RecentInputs {
    pub fn record(&mut self, game: &Game, buttons: u16) {
        self.stage_id = game.mission.stage.id;
        self.recent.push(
            self.stage_id as u64,
            game.tick,
            Input {
                tick_before: game.tick,
                clock_before: game.clock,
                rng_before: game.random.seed,
                buttons,
                player_position: game.player().position.to_array(),
                player_health: game.player().health,
                alert: game.intelligence.alert,
                strain: game.intelligence.strain,
            },
        );
    }
}

impl Application {
    pub(super) fn diagnostic_report(&self) -> Result<Vec<u8>> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.native_tenchu2.is_some() {
            return self.native_diagnostic_report();
        }
        let game = &self.campaign.game;
        let screen = match self.campaign.screen {
            Screen::Frontend => "frontend",
            Screen::Launcher => "launcher",
            Screen::Inventory(_) => "inventory",
            Screen::Playing => "playing",
            Screen::DeathHold { .. } => "death-hold",
            Screen::Results(_) => "results",
            Screen::GameOver => "game_over",
            Screen::Ending(_) => "ending",
            Screen::Complete => "complete",
        };
        let mut report = crate::diagnostics::report(&self.settings, &self.debug, &self.controls);
        let details = json!({
            "recording_source": self.reverse.source_report.as_ref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
            "input_encoding": "PS1 pad mask (Gaisen; directional buttons in high byte)",
            "engine": "gaisen",
            "disc": {"edition": "SLPS-019.01", "main_exe_sha256": tenchu_assets::game::EXE_SHA256},
            "effective_graphics": self.view.as_ref().map(|v| v.renderer.graphics()),
            "runtime": {"screen": screen, "paused": self.paused, "suspended": self.is_suspended(),
                "simulating": self.simulating(),
                "death_retry_seed": match self.campaign.screen {
                    Screen::DeathHold { retry_seed } => Some(retry_seed),
                    _ => None,
                },
                "focused": self.focused, "menu_open": self.menu.open, "faulted": self.faulted,
                "fault": self.fault, "error": self.error, "audio_active": self.audio.is_some(),
                "recording_inputs": self.recording.is_some(), "demo": self.demo.as_ref().map(|d| &d.clip().id),
                "benchmark_active": self.benchmarks.run.is_some()},
            "session": {"mission": self.campaign.options.mission, "stage_id": game.mission.stage.id,
                "character": self.campaign.options.character, "layout": self.campaign.options.layout,
                "difficulty": self.campaign.options.difficulty.name(),
                "language": format!("{:?}", self.campaign.options.language.unwrap_or_default()),
                "starting_seed": self.campaign.seed,
                "randomizer": self.campaign.randomized.as_ref().map(|r| &r.run)},
            "game": crate::diagnostics::game(game), "recent_inputs": self.recent_inputs,
            "performance": self.performance.diagnostic_snapshot(),
            "renderer": self.view.as_ref().map(View::diagnostic_snapshot),
        });
        report
            .as_object_mut()
            .unwrap()
            .extend(details.as_object().unwrap().clone());
        report["debug"]["rewind_ticks"] = json!(self.rewind.ticks(game.tick));
        report["debug"]["rewind_bytes"] = json!(self.rewind.bytes);
        report["debug"]["rewind_range"] = json!(
            self.rewind
                .entries
                .front()
                .zip(self.rewind.entries.back())
                .map(|(a, b)| [a.tick, b.tick])
        );
        report["debug"]["rewind_playing"] = json!(self.reverse.active);
        report["debug"]["rewind_preview"] = json!(self.reverse.preview);
        crate::diagnostics::finish(report, &self.settings, self.settings_store.path.parent())
    }

    pub(super) fn save_diagnostics(&mut self) -> Result<()> {
        let bytes = self.diagnostic_report()?;
        let name = format!(
            "tenchu-report-{}-m{}-tick{}.json",
            env!("CARGO_PKG_VERSION"),
            self.campaign.options.mission,
            self.campaign.game.tick
        );
        #[cfg(not(target_arch = "wasm32"))]
        let name = self.native_tenchu2.as_ref().map_or(name, |native| {
            crate::tenchu2::native::diagnostics::filename(&native.playback)
        });
        self.report_save.start(name, bytes);
        self.sync_pause();
        Ok(())
    }

    pub(super) fn poll_diagnostics(&mut self) {
        let Some(result) = self.report_save.poll() else {
            return;
        };
        let notice = match result {
            Ok(notice) => notice,
            Err(error) => Some(format!("Could not save diagnostic report: {error}")),
        };
        if let Some(notice) = notice {
            self.menu.notice = notice.clone();
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(native) = &mut self.native_tenchu2 {
                native.menu.panel.notice = notice;
            }
        }
        self.sync_pause();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC"]
    fn report_preserves_live_settings_and_resets_input_history_on_retry() {
        let mut app = super::super::tests::application(Options {
            mission: 3,
            ..Default::default()
        });
        app.settings.retro = true;
        app.settings.model_directory = "/home/private/models".into();
        app.controls.vibration_error = Some("Test driver error /home/private/models".into());
        app.keep_enemies_active(true);
        app.set_sight(150);
        for _ in 0..605 {
            app.game_tick(0);
        }
        let before = crate::session::tests::state(&app.campaign.game);
        let bytes = app.diagnostic_report().unwrap();
        let report = crate::diagnostics::decode(&bytes).unwrap();
        assert!(!String::from_utf8(bytes).unwrap().contains("/home/private"));
        assert_eq!(report["settings"]["retro"], true);
        assert_eq!(report["game"]["activation"]["keep_all_active"], true);
        assert_eq!(report["game"]["ai"]["sight_percent"], 150);
        assert_eq!(report["controls"]["controller_backend_available"], false);
        assert!(
            report["controls"]["controller_details"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            report["controls"]["vibration_error"]
                .as_str()
                .unwrap()
                .starts_with("Test driver error")
        );
        assert_eq!(
            report["recent_inputs"]["inputs"].as_array().unwrap().len(),
            600
        );
        assert_eq!(crate::session::tests::state(&app.campaign.game), before);
        app.campaign.game.tick = 0;
        app.recent_inputs
            .record(&app.campaign.game, tenchu_core::input::SQUARE);
        assert_eq!(app.recent_inputs.recent.inputs.len(), 1);
    }
}
