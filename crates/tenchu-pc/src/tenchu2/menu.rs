use tenchu_assets::tenchu2::layout::Layout;

#[derive(Default)]
pub struct Menu {
    pub open: bool,
    pub path: String,
    pub notice: String,
    pub character: i16,
    pub map: usize,
    pub sequence: Option<usize>,
    pub guard: Option<i16>,
    pub source_models: bool,
    pub maps: Vec<usize>,
    pub layouts: Vec<Layout>,
    pub connected: String,
}

pub enum Event {
    Open,
    Close,
    Browse,
    Connect,
    Play,
}

impl Menu {
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Event> {
        if !self.open {
            return None;
        }
        let mut event = None;
        let mut open = true;
        let width = (ctx.screen_rect().width() - 32.0).clamp(240.0, 540.0);
        egui::Window::new("Tenchu 2 — scenery sandbox")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(width)
            .max_width(width)
            .show(ctx, |ui| {
                ui.set_width(width);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                egui::ScrollArea::vertical()
                    .max_height((ctx.screen_rect().height() - 100.0).max(160.0))
                    .show(ui, |ui| {
                        ui.label("Explore Tenchu 2 areas with Gaisen movement and combat. Story missions, objectives and Tenchu 2 AI are not included.");
                        ui.label("Requires your Tenchu 2 Europe disc (SLES-02452), in addition to Shinobi Gaisen.");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.path)
                                .desired_width((width - 150.0).max(100.0))
                                .hint_text("Tenchu 2 CUE path"));
                            if ui.button("Browse…").clicked() { event = Some(Event::Browse); }
                            if ui.button("Load maps").clicked() { event = Some(Event::Connect); }
                        });
                        if !self.notice.is_empty() { ui.label(&self.notice); }
                        if self.maps.is_empty() || self.path != self.connected { return; }
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Area");
                            egui::ComboBox::from_id_salt("t2-map")
                                .selected_text(format!("Map {}", self.map))
                                .show_ui(ui, |ui| {
                                    for &map in &self.maps {
                                        if ui.selectable_value(&mut self.map, map, format!("Map {map}")).changed() {
                                            self.sequence = self.layouts.iter().find(|l| l.map == map).map(|l| l.sequence);
                                        }
                                    }
                                });
                        });
                        let layouts: Vec<_> = self.layouts.iter().filter(|l| l.map == self.map).collect();
                        if !layouts.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("Placement / patrol layout");
                                egui::ComboBox::from_id_salt("t2-layout")
                                    .selected_text(self.sequence.map_or_else(|| "Default".into(), |s| format!("Sequence {s}")))
                                    .show_ui(ui, |ui| {
                                        for layout in &layouts {
                                            ui.selectable_value(&mut self.sequence, Some(layout.sequence), format!("Sequence {} · {} actors", layout.sequence, layout.actors.len()));
                                        }
                                    });
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.label("Player controls");
                            ui.selectable_value(&mut self.character, 0, "Rikimaru");
                            ui.selectable_value(&mut self.character, 1, "Ayame");
                        });
                        ui.add_enabled_ui(!layouts.is_empty(), |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Guards (Gaisen AI)");
                                for (kind, label) in [(None, "None"), (Some(0x10), "Sword"), (Some(0x12), "Spear"), (Some(0x11), "Archer")] {
                                    ui.selectable_value(&mut self.guard, kind, label);
                                }
                            });
                            ui.checkbox(&mut self.source_models, "Use Tenchu 2 character models for guards");
                        });
                        ui.label("F1: settings and player model · F5: restart · Back: leave sandbox. Campaign progress is preserved.");
                        if ui.button("Start exploring").clicked() { event = Some(Event::Play); }
                    });
            });
        if !open {
            event = Some(Event::Close);
        }
        event
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn disc_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    if let Some(path) = crate::android::data_dir()
        .ok()
        .and_then(|root| std::fs::read_to_string(root.join("tenchu2-path.txt")).ok())
    {
        return Some(path.trim().into());
    }
    let path = crate::storage::UserFile::Tenchu2.path().ok()?;
    serde_json::from_slice(&crate::storage::read(&path).ok()?).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remember(path: &std::path::Path) -> tenchu_assets::Result<()> {
    crate::storage::atomic_write(
        &crate::storage::UserFile::Tenchu2.path()?,
        &serde_json::to_vec(path).map_err(|e| e.to_string())?,
    )
}
