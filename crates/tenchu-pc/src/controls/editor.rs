use super::{
    Controls,
    bindings::{Bindings, PadInput, Target},
};
use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Device {
    #[default]
    Keyboard,
    Controller,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Capture {
    target: Target,
    slot: usize,
    device: Device,
}

#[derive(Default)]
pub struct Editor {
    pub open: bool,
    pub notice: String,
    device: Device,
    capture: Option<Capture>,
}

impl Controls {
    pub fn capturing(&self) -> bool {
        self.editor.open && self.editor.capture.is_some()
    }

    pub fn cancel_capture(&mut self) {
        self.editor.capture = None;
    }

    pub fn close_editor(&mut self) {
        self.editor.open = false;
        self.cancel_capture();
        self.clear();
    }

    pub fn editor_key(&mut self, code: KeyCode) {
        if code == KeyCode::Escape {
            if self.capturing() {
                self.cancel_capture();
            } else {
                self.close_editor();
            }
        } else if let Some(Capture {
            target,
            slot,
            device: Device::Keyboard,
        }) = self.editor.capture
        {
            match self.bindings.bind_key(target, slot, code) {
                Ok(()) => {
                    self.cancel_capture();
                    self.save_bindings();
                }
                Err(error) => self.editor.notice = error,
            }
        }
    }

    pub(super) fn editor_button(&mut self, input: PadInput) {
        if let Some(Capture {
            target,
            slot,
            device: Device::Controller,
        }) = self.editor.capture
        {
            self.bindings.bind_controller(target, slot, input);
            self.cancel_capture();
            self.save_bindings();
        }
    }

    pub fn show_bindings(&mut self, ctx: &egui::Context) {
        if !self.editor.open {
            return;
        }
        let capture = self.editor.capture;
        let changed = self.editor.show(ctx, &mut self.bindings);
        if changed {
            self.save_bindings();
        }
        if capture != self.editor.capture || !self.editor.open {
            self.clear();
        }
    }
}

impl Editor {
    fn show(&mut self, ctx: &egui::Context, bindings: &mut Bindings) -> bool {
        let mut changed = false;
        egui::Modal::new(egui::Id::new("control-bindings")).show(ctx, |ui| {
            ui.set_width(580.0_f32.min(ctx.screen_rect().width() - 48.0));
            ui.horizontal(|ui| {
                ui.heading("Controls");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Done").clicked() { self.open = false; self.capture = None; }
                });
            });
            ui.label("Click a binding, then press a key, button or stick direction. Changes save automatically. × clears a binding; Escape cancels capture.");
            ui.weak("Actions shown for original layout 1. Other original layouts still apply. Assigning an input moves it from its previous action.");
            ui.horizontal(|ui| {
                for (device, label) in [(Device::Keyboard, "Keyboard"), (Device::Controller, "Controller")] {
                    if ui.selectable_value(&mut self.device, device, label).clicked() {
                        self.capture = None;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height((ctx.screen_rect().height() - 300.0).max(120.0)).show(ui, |ui| {
                egui::Grid::new("binding-rows").striped(true).spacing([8.0, 5.0]).show(ui, |ui| {
                    ui.strong("Action"); ui.strong("Primary"); ui.strong("Alternate"); ui.end_row();
                    for target in Target::ALL {
                        ui.label(target.description().0);
                        for slot in 0..2 {
                            let capture = Capture { target, slot, device: self.device };
                            let row = bindings.0.get_mut(&target).unwrap();
                            let label = if self.capture == Some(capture) {
                                "Press input…".into()
                            } else {
                                match self.device {
                                    Device::Keyboard => row.keys[slot].map(|key| format!("{key:?}").trim_start_matches("Key").to_owned()),
                                    Device::Controller => row.controller[slot].map(PadInput::label),
                                }.unwrap_or_else(|| "Unbound".into())
                            };
                            ui.horizontal(|ui| {
                                if ui.add_sized([146.0, 23.0], egui::Button::new(label).selected(self.capture == Some(capture))).clicked() {
                                    self.capture = Some(capture);
                                    self.notice = "Release any held inputs, then press the new input.".into();
                                }
                                if ui.small_button("×").on_hover_text("Clear binding").clicked() {
                                    match self.device {
                                        Device::Keyboard => row.keys[slot] = None,
                                        Device::Controller => row.controller[slot] = None,
                                    }
                                    self.capture = None;
                                    changed = true;
                                }
                            });
                        }
                        ui.end_row();
                    }
                });
            });
            ui.separator();
            if ui.button(match self.device { Device::Keyboard => "Reset keyboard to defaults", Device::Controller => "Reset controller to defaults" }).clicked() {
                for (target, default) in Bindings::default().0 {
                    let row = bindings.0.get_mut(&target).unwrap();
                    match self.device {
                        Device::Keyboard => row.keys = default.keys,
                        Device::Controller => row.controller = default.controller,
                    }
                }
                self.capture = None;
                changed = true;
            }
            ui.weak("Escape / Enter / F1–F12 keep their menu and tool shortcuts. Backspace keeps cutscene fast-forward. Keyboard bindings use physical key positions.");
            if !self.notice.is_empty() { ui.label(&self.notice); }
        });
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::input;

    #[test]
    fn captures_save_without_playing_the_input_and_escape_cancels_without_saving() {
        let path =
            std::env::temp_dir().join(format!("tenchu-bindings-{}.json", std::process::id()));
        let mut controls = Controls {
            bindings_path: Some(path.clone()),
            ..Default::default()
        };
        controls.editor.open = true;
        controls.editor.capture = Some(Capture {
            target: Target::Cross,
            slot: 0,
            device: Device::Keyboard,
        });
        controls.editor_key(KeyCode::Enter);
        assert!(controls.capturing());
        controls.editor_key(KeyCode::KeyK);
        assert!(!controls.capturing());
        assert_eq!(controls.buttons(), 0);
        let saved = std::fs::read(&path).unwrap();
        assert_eq!(
            Bindings::decode(&saved).unwrap().key(KeyCode::KeyK),
            input::CROSS
        );
        controls.editor.capture = Some(Capture {
            target: Target::Square,
            slot: 0,
            device: Device::Keyboard,
        });
        controls.editor_key(KeyCode::Escape);
        assert!(controls.editor.open);
        assert!(!controls.capturing());
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        controls.editor.capture = Some(Capture {
            target: Target::Cross,
            slot: 0,
            device: Device::Controller,
        });
        controls.editor_button(PadInput::Button(gilrs::Button::RightThumb));
        let restored = Bindings::decode(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            restored.controller(|b| b == gilrs::Button::RightThumb, [0; 4]),
            input::CROSS
        );
        assert_eq!(controls.buttons(), 0);
        controls.editor_key(KeyCode::Escape);
        assert!(!controls.editor.open);
        controls.key(KeyCode::KeyK, true);
        controls.key(KeyCode::KeyK, false);
        assert_eq!(controls.buttons(), input::CROSS);
        assert_eq!(controls.buttons(), 0);
        std::fs::remove_file(path).unwrap();
    }
}
