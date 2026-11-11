use super::*;
use crate::tenchu2::native::savestate;

impl Application {
    pub(super) fn state_action(&mut self, save: bool) {
        let result = (|| {
            let path = savestate::path(self.menu.tools.debug.state_slot)?;
            let view = self
                .view
                .as_mut()
                .ok_or("Wait for the game window before using savestates")?;
            if save {
                if self.error.is_some()
                    || view.generation != self.session.generation
                    || self.menu.actor_assets_changed
                {
                    return Err("Wait for the current scene before saving a state".into());
                }
                let bytes = savestate::capture(
                    &self.session,
                    &view.drawing,
                    &self.settings,
                    &self.menu.tools,
                )?;
                crate::storage::atomic_write(&path, &bytes)?;
                self.transport.discard_backlog();
            } else {
                let bytes = crate::savestate::read(&path)?;
                let drawing = savestate::restore(
                    &mut self.session,
                    &bytes,
                    &mut self.settings,
                    &mut self.menu.tools,
                    &mut view.renderer,
                );
                view.drawing = drawing.map_err(|error| {
                    if error.drawing_changed {
                        view.generation = self.session.generation.wrapping_sub(1);
                    }
                    error.message
                })?;
                view.generation = self.session.generation;
                view.graphics = preferences::graphics(&self.settings);
                if let (Some(pack), Some(gallery)) =
                    (self.session.content.embedded_bodies(), &mut self.gallery)
                {
                    gallery.bodies = pack;
                    gallery.picker.clear();
                }
                self.menu
                    .loaded_state(&mut self.controls, &mut self.presets);
                self.transport
                    .restored(&mut self.session, self.audio.as_ref());
                self.rewind.clear();
                self.paused = true;
                self.error = None;
                self.quit = false;
                if let Some(audio) = &self.audio {
                    audio.send(Command::Pause {
                        paused: true,
                        music_while_paused: self.settings.music_while_paused
                            && !self.session.content.cutscene(),
                    });
                }
            }
            Ok(())
        })();
        self.last = Instant::now();
        self.menu.state_result(save, result);
    }
}
