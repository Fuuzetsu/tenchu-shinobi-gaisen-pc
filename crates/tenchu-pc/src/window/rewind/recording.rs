use super::*;

impl Application {
    fn rewind_selection(&self) -> Result<[u64; 2]> {
        let first = self
            .rewind
            .entries
            .front()
            .ok_or("No rewind history available")?
            .tick;
        let last = self.rewind.entries.back().unwrap().tick;
        Ok(self.reverse.selection.map_or([first, last], |[a, b]| {
            [a.clamp(first, last), b.clamp(first, last)]
        }))
    }

    pub(in crate::window) fn export_rewind(&mut self) -> Result<()> {
        self.state_tools_available()?;
        self.stop_rewind();
        let range = self.rewind_selection()?;
        let name = format!(
            "tenchu-rewind-{}-m{}-{}-{}.treplay",
            env!("CARGO_PKG_VERSION"),
            self.campaign.options.mission,
            range[0],
            range[1]
        );
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        let path = {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Export rewind recording (contains game data; share privately)")
                .add_filter("Tenchu rewind recording", &["treplay"])
                .set_file_name(&name)
                .save_file()
            else {
                return Ok(());
            };
            path
        };
        let report = if let Some(report) = &self.reverse.source_report {
            report.clone()
        } else {
            String::from_utf8(self.diagnostic_report()?).map_err(|e| e.to_string())?
        };
        let bytes = self.rewind.export(range, report)?;
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        crate::storage::atomic_write(&path, &bytes)?;
        #[cfg(target_os = "android")]
        crate::android::export(&name, &bytes)?;
        #[cfg(target_arch = "wasm32")]
        crate::web::recording::download(&name, &bytes)?;
        self.clock.discard_host_backlog();
        self.last_frame = Instant::now();
        self.state_notice(format!(
            "Exported ticks {}–{} · {:.1} MiB · contains game data; share privately",
            range[0],
            range[1],
            bytes.len() as f64 / 1048576.0
        ));
        Ok(())
    }

    pub(in crate::window) fn load_recording(&mut self, bytes: &[u8]) -> Result<()> {
        self.state_tools_available()?;
        let (mut history, report) = crate::rewind::History::import(bytes)?;
        let metadata: serde_json::Value =
            serde_json::from_str(&report).map_err(|e| format!("Invalid recording report: {e}"))?;
        if metadata["format"] != crate::diagnostics::FORMAT {
            return Err("Recording has no supported diagnostic report".into());
        }
        let source_version = metadata["build"]["revision"].as_str().unwrap_or("unknown");
        let previous = self.settings.clone();
        let visual_mode = self.reverse.recorded_visuals.replace(true);
        let result = history
            .snapshot(0)
            .and_then(|snapshot| self.restore_rewind(snapshot));
        if let Err(error) = result {
            self.reverse.recorded_visuals = visual_mode;
            self.settings = previous;
            return Err(error);
        }
        self.rewind = history;
        self.reverse.active = false;
        self.reverse.preview = true;
        self.reverse.fraction = 0.0;
        self.reverse.selection = None;
        self.reverse.source_report = Some(report);
        self.menu.open = false;
        self.seek_rewind(self.rewind.entries.front().unwrap().tick)?;
        self.state_notice(format!(
            "Loaded {} recorded frames · build {} · inspect timeline or F3 to resume",
            self.rewind.entries.len(),
            source_version
        ));
        Ok(())
    }

    pub(in crate::window) fn import_rewind(&mut self) -> Result<()> {
        self.state_tools_available()?;
        self.stop_rewind();
        self.debug.pause_view = true;
        self.set_paused(true);
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Import rewind recording")
                .add_filter("Tenchu rewind recording", &["treplay"])
                .pick_file()
            else {
                return Ok(());
            };
            let bytes = crate::savestate::read(&path)?;
            self.load_recording(&bytes)
        }
        #[cfg(target_os = "android")]
        return crate::android::import("recording");
        #[cfg(target_arch = "wasm32")]
        crate::web::recording::start(crate::web::recording::Kind::Recording)
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::window) fn poll_recording(&mut self) {
        let Some((kind, result)) = crate::web::recording::poll() else {
            return;
        };
        if let Err(error) = result.and_then(|bytes| {
            if bytes.is_empty() {
                Ok(())
            } else {
                match kind {
                    crate::web::recording::Kind::Recording => self.load_recording(&bytes),
                    crate::web::recording::Kind::Unarmed => {
                        self.install_unarmed(crate::sheathing::imports::decode(&bytes)?)
                    }
                    crate::web::recording::Kind::StudioAsset(kind) => {
                        self.studio
                            .import_custom(format!("Imported {}", kind.label()), kind, bytes)
                    }
                    crate::web::recording::Kind::Studio => {
                        if self.studio.open {
                            self.studio.import(&bytes, &self.assets)
                        } else {
                            Err("Open Level Studio to import this project".into())
                        }
                    }
                }
            }
        }) {
            if matches!(
                kind,
                crate::web::recording::Kind::Studio | crate::web::recording::Kind::StudioAsset(_)
            ) {
                self.studio.notice = error;
            } else {
                self.state_notice(error);
            }
        }
        self.update_view();
    }

    pub(in crate::window) fn play_rewind_forward(&mut self) -> Result<()> {
        self.state_tools_available()?;
        let tick = self.campaign.game.tick;
        if !self.rewind.entries.iter().any(|e| e.tick > tick) {
            return Err("Already at the latest recorded frame".into());
        }
        self.seek_rewind(tick)?;
        self.reverse.forward = true;
        self.reverse.active = true;
        self.menu.open = false;
        self.finish_recording();
        self.controls.stop_rumble();
        self.release_pointer();
        self.sync_pause();
        self.update_view();
        Ok(())
    }
}
