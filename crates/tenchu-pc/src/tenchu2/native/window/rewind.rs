use super::*;
use crate::tenchu2::native::{
    rewind::{Action, State},
    savestate::RestoreError,
};

impl Application {
    fn rewind_result(&mut self, result: std::result::Result<Option<String>, RestoreError>) {
        match result {
            Ok(notice) => {
                if let Some(view) = &mut self.view
                    && view.generation != self.session.generation
                {
                    view.generation = self.session.generation;
                    view.graphics = preferences::graphics(&self.settings);
                    if let (Some(pack), Some(gallery)) =
                        (self.session.content.embedded_bodies(), &mut self.gallery)
                    {
                        gallery.bodies = pack;
                        gallery.picker.clear();
                    }
                    let menu_open = self.menu.panel.open;
                    self.menu
                        .loaded_state(&mut self.controls, &mut self.presets);
                    self.menu.panel.open = menu_open;
                    self.error = None;
                    self.quit = false;
                }
                if let Some(notice) = notice {
                    self.menu.notice(notice);
                }
            }
            Err(error) => {
                if error.drawing_changed
                    && let Some(view) = &mut self.view
                {
                    view.generation = self.session.generation.wrapping_sub(1);
                }
                self.rewind.stop();
                self.menu.notice(error.message);
            }
        }
        self.rewind.sync(&mut self.menu.tools);
        if self.rewind.preview {
            self.transport
                .preview(&mut self.session, self.audio.as_ref());
        }
    }

    pub(super) fn rewind_action(&mut self, action: Action) {
        let report = if matches!(action, Action::Export) {
            match self
                .diagnostic_report(true)
                .and_then(|r| String::from_utf8(r).map_err(|e| e.to_string()))
            {
                Ok(report) => Some(report),
                Err(error) => {
                    self.menu.notice(error);
                    return;
                }
            }
        } else {
            None
        };
        if !matches!(action, Action::Select(_)) {
            self.paused = true;
            self.controls.clear();
            self.controls.stop_rumble();
            if let Some(audio) = &self.audio {
                audio.send(Command::Pause {
                    paused: true,
                    music_while_paused: false,
                });
            }
        }
        let result = (|| {
            let view = self
                .view
                .as_mut()
                .ok_or_else(|| "Wait for the game window before using rewind".to_string())?;
            self.rewind.apply(
                action,
                &mut State {
                    playback: &mut self.session,
                    drawing: &mut view.drawing,
                    settings: &mut self.settings,
                    tools: &mut self.menu.tools,
                    renderer: &mut view.renderer,
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
                self.menu.panel.open = false;
            }
        }
        self.rewind_result(result);
        self.last = Instant::now();
    }

    pub(super) fn advance_rewind(&mut self, elapsed: std::time::Duration) {
        if !self.rewind.active
            || !self.focused && self.settings.pause_on_defocus
            || self.menu.panel.open && self.settings.pause_in_menu
        {
            return;
        }
        let Some(view) = &mut self.view else {
            return;
        };
        let result = self.rewind.advance(
            elapsed,
            &mut State {
                playback: &mut self.session,
                drawing: &mut view.drawing,
                settings: &mut self.settings,
                tools: &mut self.menu.tools,
                renderer: &mut view.renderer,
            },
        );
        if !matches!(result, Ok(false)) {
            self.rewind_result(result.map(|_| None));
        }
    }
}
