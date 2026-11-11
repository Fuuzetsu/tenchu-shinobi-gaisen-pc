use super::*;
use crate::tenchu2::native::{
    rewind::{Action, State},
    savestate::RestoreError,
};

impl Application {
    fn native_rewind_result(&mut self, result: std::result::Result<Option<String>, RestoreError>) {
        let native = self.native_tenchu2.as_mut().unwrap();
        match result {
            Ok(notice) => {
                if native.generation != Some(native.playback.generation) {
                    native.generation = Some(native.playback.generation);
                    native.graphics = preferences::graphics(&self.settings);
                    if let Some(view) = &mut self.view
                        && let Some(pack) = native.playback.content.embedded_bodies()
                    {
                        view.bodies = pack;
                        view.model_picker.clear();
                    }
                    let menu_open = native.menu.panel.open;
                    native
                        .menu
                        .loaded_state(&mut self.controls, &mut self.presets);
                    native.menu.panel.open = menu_open;
                    native.error = None;
                    self.touch.clear();
                }
                if let Some(notice) = notice {
                    native.menu.notice(notice);
                }
            }
            Err(error) => {
                if error.drawing_changed {
                    native.generation = None;
                }
                native.rewind.stop();
                native.menu.notice(error.message);
            }
        }
        native.rewind.sync(&mut native.menu.tools);
        if native.rewind.preview {
            native.transport.preview(
                &mut native.playback,
                self.audio.as_ref().map(|a| a.output()),
            );
        }
        self.sync_audio_pause();
    }

    pub(super) fn native_rewind_action(&mut self, action: Action) {
        let report = if matches!(action, Action::Export) {
            match self
                .native_diagnostic_report()
                .and_then(|r| String::from_utf8(r).map_err(|e| e.to_string()))
            {
                Ok(report) => Some(report),
                Err(error) => {
                    self.native_tenchu2.as_mut().unwrap().menu.notice(error);
                    return;
                }
            }
        } else {
            None
        };
        if !matches!(action, Action::Select(_)) {
            self.paused = true;
            self.controls.stop_rumble();
            self.sync_pause();
        }
        let result = (|| {
            let native = self.native_tenchu2.as_mut().unwrap();
            let renderer = &mut self
                .view
                .as_mut()
                .filter(|v| !v.preparing())
                .ok_or_else(|| "Wait for the game window before using rewind".to_string())?
                .renderer;
            let drawing = native
                .drawing
                .as_mut()
                .ok_or_else(|| "Wait for the current scene before using rewind".to_string())?;
            native.rewind.apply(
                action,
                &mut State {
                    playback: &mut native.playback,
                    drawing,
                    settings: &mut self.settings,
                    tools: &mut native.menu.tools,
                    renderer,
                },
                report,
            )
        })();
        if result.is_ok() {
            if matches!(action, Action::Resume) {
                self.paused = false;
            }
            if matches!(
                action,
                Action::Toggle | Action::Forward | Action::Resume | Action::Import
            ) {
                self.native_tenchu2.as_mut().unwrap().menu.panel.open = false;
            }
        }
        self.native_rewind_result(result);
        self.release_pointer();
        self.sync_pause();
    }

    pub(super) fn advance_native_rewind(&mut self, elapsed: std::time::Duration) {
        let native = self.native_tenchu2.as_mut().unwrap();
        if !native.rewind.active
            || !self.focused && self.settings.pause_on_defocus
            || native.menu.panel.open && self.settings.pause_in_menu
        {
            return;
        }
        let Some(view) = &mut self.view else {
            return;
        };
        let Some(drawing) = &mut native.drawing else {
            return;
        };
        let result = native.rewind.advance(
            elapsed,
            &mut State {
                playback: &mut native.playback,
                drawing,
                settings: &mut self.settings,
                tools: &mut native.menu.tools,
                renderer: &mut view.renderer,
            },
        );
        if !matches!(result, Ok(false)) {
            self.native_rewind_result(result.map(|_| None));
        }
    }

    #[cfg(target_os = "android")]
    pub(in crate::window) fn load_native_recording(&mut self, bytes: &[u8]) -> Result<()> {
        self.paused = true;
        let native = self.native_tenchu2.as_mut().unwrap();
        let view = self
            .view
            .as_mut()
            .filter(|v| !v.preparing())
            .ok_or("Wait for the game window before importing")?;
        let drawing = native
            .drawing
            .as_mut()
            .ok_or("Wait for the current scene before importing")?;
        let result = native.rewind.load(
            bytes,
            &mut State {
                playback: &mut native.playback,
                drawing,
                settings: &mut self.settings,
                tools: &mut native.menu.tools,
                renderer: &mut view.renderer,
            },
        );
        if result.is_ok() {
            native.menu.panel.open = false;
        }
        self.native_rewind_result(result.map(Some));
        self.release_pointer();
        self.sync_pause();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC and TENCHU2_DISC; automatic seeks must include restore time in the next host interval and keep an open settings panel"]
    fn native_rewind_restoration_keeps_host_clock_and_open_menu() {
        let mut app = crate::window::tests::application(Default::default());
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let playback = Playback::load(disc, 9, 2, 17, Some([0; 21]), None).unwrap();
        let mut rewind = crate::tenchu2::native::rewind::Rewind::default();
        rewind.preview = true;
        let mut menu = preferences::Menu::default();
        menu.panel.open = true;
        app.native_tenchu2 = Some(Native {
            playback,
            drawing: None,
            generation: None,
            graphics: preferences::graphics(&app.settings),
            menu,
            transport: Default::default(),
            rewind,
            error: None,
            donor: None,
        });
        app.paused = true;
        app.last_frame = Instant::now() - std::time::Duration::from_secs(1);
        let before = app.last_frame;
        app.native_rewind_result(Ok(None));
        assert_eq!(app.last_frame, before);
        let native = app.native_tenchu2.as_ref().unwrap();
        assert!(native.menu.panel.open && native.rewind.preview);
        assert_eq!(native.generation, Some(native.playback.generation));
    }
}
