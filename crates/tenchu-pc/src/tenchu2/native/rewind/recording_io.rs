use super::*;

fn executable() -> &'static str {
    tenchu_assets::tenchu2::Edition::Japan.game_hash()
}

impl Rewind {
    pub(super) fn export(&mut self, report: String) -> Result<String> {
        let range = self
            .timeline()
            .ok_or("No rewind history available")?
            .selection;
        let name = format!(
            "tenchu2-jp-rewind-{}-{}-{}.treplay",
            env!("CARGO_PKG_VERSION"),
            range[0],
            range[1]
        );
        #[cfg(not(target_os = "android"))]
        let path = {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Export rewind recording (contains game data; share privately)")
                .add_filter("Tenchu rewind recording", &["treplay"])
                .set_file_name(&name)
                .save_file()
            else {
                return Ok("Export cancelled".into());
            };
            path
        };
        let report = self.source_report.clone().unwrap_or(report);
        let bytes = self.history.export_for(range, report, executable())?;
        #[cfg(not(target_os = "android"))]
        crate::storage::atomic_write(&path, &bytes)?;
        #[cfg(target_os = "android")]
        crate::android::export(&name, &bytes)?;
        Ok(format!(
            "Exported ticks {}–{} · {:.1} MiB · contains game data; share privately",
            range[0],
            range[1],
            bytes.len() as f64 / 1048576.0
        ))
    }

    pub fn load(
        &mut self,
        bytes: &[u8],
        state: &mut State<'_, impl ReadAt>,
    ) -> std::result::Result<String, savestate::RestoreError> {
        let (history, report) = History::import_for(bytes, executable())?;
        let metadata: serde_json::Value =
            serde_json::from_str(&report).map_err(|e| format!("Invalid recording report: {e}"))?;
        if metadata["format"] != crate::diagnostics::FORMAT {
            return Err("Recording has no supported diagnostic report"
                .to_string()
                .into());
        }
        let version = metadata["build"]["revision"].as_str().unwrap_or("unknown");
        let notice = format!(
            "Loaded {} recorded frames · build {version} · inspect timeline or F3 to resume",
            history.entries.len()
        );
        let tick = history.entries.front().unwrap().tick;
        let mut incoming = Self {
            history,
            recorded_visuals: Some(true),
            source_report: Some(report),
            ..Default::default()
        };
        incoming.seek(tick, state)?;
        *self = incoming;
        Ok(notice)
    }

    pub(super) fn import(
        &mut self,
        state: &mut State<'_, impl ReadAt>,
    ) -> std::result::Result<Option<String>, savestate::RestoreError> {
        #[cfg(not(target_os = "android"))]
        {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Import native T2 rewind recording")
                .add_filter("Tenchu rewind recording", &["treplay"])
                .pick_file()
            else {
                return Ok(None);
            };
            self.load(&crate::savestate::read(&path)?, state).map(Some)
        }
        #[cfg(target_os = "android")]
        {
            let _ = state;
            crate::android::import("recording")?;
            Ok(None)
        }
    }
}
