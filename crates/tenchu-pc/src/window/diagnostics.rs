use super::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::VecDeque;

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
    inputs: VecDeque<Input>,
}

impl RecentInputs {
    pub fn record(&mut self, game: &Game, buttons: u16) {
        if self.stage_id != game.mission.stage.id
            || self
                .inputs
                .back()
                .is_some_and(|i| i.tick_before >= game.tick)
        {
            self.inputs.clear();
        }
        self.stage_id = game.mission.stage.id;
        if self.inputs.len() == 600 {
            self.inputs.pop_front();
        }
        self.inputs.push_back(Input {
            tick_before: game.tick,
            clock_before: game.clock,
            rng_before: game.random.seed,
            buttons,
            player_position: game.player().position.to_array(),
            player_health: game.player().health,
            alert: game.intelligence.alert,
            strain: game.intelligence.strain,
        });
    }
}

fn platform() -> Value {
    let mut info = json!({
        "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "pointer_bits": usize::BITS, "byte_order": if cfg!(target_endian = "little") { "little" } else { "big" },
        "available_threads": std::thread::available_parallelism().ok().map(|n| n.get()),
    });
    #[cfg(target_os = "linux")]
    {
        info["kernel"] = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .ok()
            .map(|s| s.trim().to_owned())
            .into();
        if let Ok(release) = std::fs::read_to_string("/etc/os-release") {
            info["distribution"] = release
                .lines()
                .filter_map(|line| {
                    let (key, value) = line.split_once('=')?;
                    ["ID", "VERSION_ID", "PRETTY_NAME"]
                        .contains(&key)
                        .then(|| (key.into(), value.trim_matches('"').into()))
                })
                .collect::<serde_json::Map<String, Value>>()
                .into();
        }
        info["session_type"] = std::env::var("XDG_SESSION_TYPE").ok().into();
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        info["version"] = std::process::Command::new("cmd")
            .args(["/D", "/C", "ver"])
            .creation_flags(0x08000000)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .into();
    }
    #[cfg(target_os = "macos")]
    {
        info["version"] = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .into();
    }
    #[cfg(target_arch = "wasm32")]
    {
        info["browser"] =
            serde_json::from_str(&crate::web::diagnostic_platform()).unwrap_or(Value::Null);
    }
    info
}

impl Application {
    fn diagnostic_report(&self) -> Result<Vec<u8>> {
        let game = &self.campaign.game;
        let debug = &self.debug;
        let mut settings = serde_json::to_value(&self.settings).map_err(|e| e.to_string())?;
        settings["model_directory"] = if self.settings.model_directory.is_empty() {
            ""
        } else {
            "<redacted>"
        }
        .into();
        let screen = match self.campaign.screen {
            Screen::Frontend => "frontend",
            Screen::Launcher => "launcher",
            Screen::Inventory(_) => "inventory",
            Screen::Playing => "playing",
            Screen::DeathHold => "death-hold",
            Screen::Results(_) => "results",
            Screen::GameOver => "game_over",
            Screen::Ending(_) => "ending",
            Screen::Complete => "complete",
        };
        let debug_settings = json!({"simulation_speed_percent": debug.speed, "god_mode": debug.god_mode,
                "stealth": debug.stealth, "click_teleport": debug.click_teleport, "armor": debug.armor,
                "sight_percent": debug.sight_percent, "photo_mode": debug.photo_mode,
                "hitboxes": debug.hitboxes, "sight": debug.sight, "sight_stance": debug.sight_stance,
                "collision": debug.collision, "collision_only": debug.collision_only,
                "patrols": debug.patrols, "events": debug.events, "labels": debug.labels,
                "xray": debug.xray, "show_trail": debug.show_trail, "inspect_actor": debug.inspect_actor,
                "selected_actor": debug.selected_actor, "target": debug.target.to_array(),
                "snap_to_floor": debug.snap_to_floor, "teleport_armed": debug.teleport_armed,
                "flight_speed": debug.flight_speed, "pause_view": debug.pause_view,
                "light": debug.light.map(|p| p.to_array()), "light_follow": debug.light_follow,
                "free_camera": debug.camera.map(|c| json!({"position": c.position.to_array(),
                    "yaw_radians": c.yaw, "pitch_radians": c.pitch, "fov_degrees": c.fov}))});
        let report = json!({
            "format": crate::diagnostics::FORMAT, "schema_version": crate::diagnostics::VERSION,
            "created_unix_ms": web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64),
            "build": crate::diagnostics::build(), "platform": platform(),
            "notes": ["No disc assets, raw process memory, saved campaign or filesystem paths are required to read this report.",
                "Snapshot and recent inputs are diagnostic context, not a restorable savestate or complete replay.",
                "Game positions use original integer coordinates; inputs are PS1 pad bitmasks. Timings are milliseconds.",
                "Personal path roots are redacted. Review custom names and errors before sharing. Nothing is uploaded automatically."],
            "disc": {"edition": "SLPS-019.01", "main_exe_sha256": tenchu_assets::game::EXE_SHA256},
            "settings": settings,
            "effective_graphics": self.view.as_ref().map(|v| v.renderer.graphics()),
            "controls": self.controls.diagnostic_settings(),
            "runtime": {"screen": screen, "paused": self.paused, "suspended": self.is_suspended(),
                "focused": self.focused, "menu_open": self.menu.open, "faulted": self.faulted,
                "fault": self.fault, "error": self.error, "audio_active": self.audio.is_some(),
                "recording_inputs": self.recording.is_some(), "demo": self.demo.as_ref().map(|d| &d.clip().id),
                "benchmark_active": self.benchmarks.run.is_some()},
            "debug": debug_settings,
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
        let mut paths = vec![self.settings.model_directory.clone()];
        #[cfg(not(target_arch = "wasm32"))]
        {
            for key in [
                "HOME",
                "USERPROFILE",
                "APPDATA",
                "LOCALAPPDATA",
                "TEMP",
                "TMP",
                "XDG_CONFIG_HOME",
                "XDG_DATA_HOME",
            ] {
                if let Some(path) = std::env::var_os(key) {
                    paths.push(path.to_string_lossy().into_owned());
                }
            }
            for path in [
                std::env::current_dir().ok(),
                crate::visuals::pack::configured_root(),
                self.settings_store.path.parent().map(PathBuf::from),
            ]
            .into_iter()
            .flatten()
            {
                paths.push(path.to_string_lossy().into_owned());
            }
        }
        paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
        crate::diagnostics::encode(report, &paths)
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
        {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Save diagnostic report")
                .add_filter("Diagnostic report (JSON)", &["json"])
                .set_file_name(&name)
                .save_file()
            else {
                return Ok(());
            };
            crate::storage::atomic_write(&path, &bytes)?;
            self.menu.notice = format!("Diagnostic report saved to {}", path.display());
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::download_report(
                &name,
                &String::from_utf8(bytes).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("Diagnostic download failed: {e:?}"))?;
            self.menu.notice = "Diagnostic report sent to browser downloads".into();
        }
        self.sync_pause();
        Ok(())
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
        assert_eq!(
            report["recent_inputs"]["inputs"].as_array().unwrap().len(),
            600
        );
        assert_eq!(crate::session::tests::state(&app.campaign.game), before);
        app.campaign.game.tick = 0;
        app.recent_inputs
            .record(&app.campaign.game, tenchu_core::input::SQUARE);
        assert_eq!(app.recent_inputs.inputs.len(), 1);
    }
}
