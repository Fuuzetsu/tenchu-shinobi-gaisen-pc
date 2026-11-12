#[cfg(not(target_arch = "wasm32"))]
pub use super::discs::{connect, disc_path, known_discs, remember};
use tenchu_assets::tenchu2::{Edition, layout::Layout};

mod launcher;

pub struct Menu {
    pub open: bool,
    pub title: bool,
    pub path: String,
    pub notice: String,
    pub character: i16,
    pub map: usize,
    pub sequence: Option<usize>,
    pub guard: Option<i16>,
    pub source_models: bool,
    pub music: Option<usize>,
    pub music_tracks: usize,
    pub maps: Vec<usize>,
    pub layouts: Vec<Layout>,
    #[cfg(not(target_arch = "wasm32"))]
    pub connected: String,
    pub discs: Vec<(Edition, String)>,
    pub selected: Option<Edition>,
    pub sandbox: bool,
    pub gaisen_t2: bool,
    pub gaisen_hero: usize,
    pub gaisen_water_depth: i16,
    initialized: bool,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            open: false,
            title: false,
            path: String::new(),
            notice: String::new(),
            character: 0,
            map: 0,
            sequence: None,
            guard: Some(0x10),
            source_models: true,
            music: None,
            music_tracks: 0,
            maps: Vec::new(),
            layouts: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            connected: String::new(),
            discs: Vec::new(),
            selected: None,
            sandbox: false,
            gaisen_t2: false,
            gaisen_hero: 0,
            gaisen_water_depth: tenchu_assets::tenchu2::collision::Gaisen::DEFAULT_WATER_DEPTH,
            initialized: false,
        }
    }
}

pub enum Event {
    Open,
    Close,
    Browse,
    Connect,
    Play,
    MissionSelection,
    Select(Option<Edition>),
    Options(crate::session::Options),
    Gaisen(bool),
    GaisenTenchu2,
    Quick,
    #[cfg(not(target_arch = "wasm32"))]
    Title,
}

impl Menu {
    pub fn imported(&self) -> bool {
        self.selected.is_none() && self.gaisen_t2 && !cfg!(target_arch = "wasm32")
    }

    pub fn japanese_disc(&self) -> Option<&str> {
        self.discs
            .iter()
            .find(|(edition, _)| *edition == Edition::Japan)
            .map(|(_, path)| path.as_str())
    }

    pub fn native(&self) -> bool {
        self.selected == Some(Edition::Japan) && !self.sandbox
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<Event> {
        if !self.open {
            return None;
        }
        if self.title {
            return self.title_import(ctx);
        }
        let mut event = None;
        let mut open = true;
        let width = (ctx.screen_rect().width() - 32.0).clamp(240.0, 540.0);
        egui::Window::new("Connect Tenchu 2 discs")
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
                        ui.label("Connect your Tenchu 2 Europe or Japan disc. Both editions stay available in F6 and in the F1 model gallery.");
                        ui.horizontal_wrapped(|ui| {
                            for (name, path) in &self.discs {
                                if ui.selectable_label(self.path == *path, name.name()).clicked() {
                                    self.path.clone_from(path);
                                    event = Some(Event::Connect);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.path)
                                .desired_width((width - 150.0).max(100.0))
                                .hint_text("CUE, disc folder or archive path"));
                            if ui.button("Browse…").clicked() { event = Some(Event::Browse); }
                            if ui.button("Connect").clicked() { event = Some(Event::Connect); }
                        });
                        if !self.notice.is_empty() { ui.label(&self.notice); }

                    });
            });
        if !open {
            event = Some(Event::Close);
        }
        event
    }

    fn title_import(&mut self, ctx: &egui::Context) -> Option<Event> {
        let mut event = None;
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            ctx.screen_rect(),
            0.0,
            egui::Color32::BLACK,
        );
        egui::Window::new("Tenchu 2")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .max_width((ctx.screen_rect().width() - 32.0).clamp(240.0, 520.0))
            .show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.label("Import your Japanese Tenchu 2 disc to open its title menu and campaign.");
                ui.add(
                    egui::TextEdit::singleline(&mut self.path)
                        .hint_text("CUE, disc folder or archive path"),
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Browse…").clicked() {
                        event = Some(Event::Browse);
                    }
                    if ui.button("Import disc").clicked() {
                        event = Some(Event::Connect);
                    }
                });
                if !self.notice.is_empty() {
                    ui.label(&self.notice);
                }
                ui.separator();
                if ui.button("◀ Shinobi Gaisen · Left / Right ▶").clicked() {
                    event = Some(Event::Close);
                }
            });
        event
    }
}
