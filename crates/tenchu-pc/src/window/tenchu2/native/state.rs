use super::*;
use crate::tenchu2::native::savestate;

impl Application {
    pub(super) fn native_state_action(&mut self, save: bool) {
        let result = (|| {
            let native = self
                .native_tenchu2
                .as_mut()
                .ok_or("Open native T2 before using its savestates")?;
            let path = savestate::path(native.menu.tools.debug.state_slot)?;
            let view = self
                .view
                .as_mut()
                .filter(|view| !view.preparing())
                .ok_or("Wait for the game window before using savestates")?;
            if save {
                if native.error.is_some()
                    || native.generation != Some(native.playback.generation)
                    || native.menu.actor_assets_changed
                {
                    return Err("Wait for the current scene before saving a state".into());
                }
                let drawing = native
                    .drawing
                    .as_ref()
                    .ok_or("Wait for the current scene before saving a state")?;
                let bytes = savestate::capture(
                    &native.playback,
                    drawing,
                    &self.settings,
                    &native.menu.tools,
                )?;
                crate::storage::atomic_write(&path, &bytes)?;
                native.transport.discard_backlog();
            } else {
                let bytes = crate::savestate::read(&path)?;
                let drawing = savestate::restore(
                    &mut native.playback,
                    &bytes,
                    &mut self.settings,
                    &mut native.menu.tools,
                    &mut view.renderer,
                );
                native.drawing = Some(drawing.map_err(|error| {
                    if error.drawing_changed {
                        native.generation = None;
                    }
                    error.message
                })?);
                native.generation = Some(native.playback.generation);
                native.graphics = preferences::graphics(&self.settings);
                if let Some(pack) = native.playback.content.embedded_bodies() {
                    view.bodies = pack;
                    view.model_picker.clear();
                }
                native
                    .menu
                    .loaded_state(&mut self.controls, &mut self.presets);
                native.transport.restored(
                    &mut native.playback,
                    self.audio.as_ref().map(|audio| audio.output()),
                );
                native.error = None;
                self.paused = true;
                self.touch.clear();
                self.release_pointer();
                self.sync_pause();
            }
            Ok(())
        })();
        self.last_frame = Instant::now();
        if let Some(native) = &mut self.native_tenchu2 {
            native.menu.state_result(save, result);
        }
    }
}
