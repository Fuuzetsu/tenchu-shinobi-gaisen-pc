use super::{Catalog, Game};
use crate::storage;
use std::path::PathBuf;
use tenchu_assets::Result;

struct Card {
    id: u64,
    name: String,
    summary: Result<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_SAVES_REVIEW and GPU; inspect named campaign summaries on desktop and narrow screens"]
    fn campaign_chooser_captures() {
        let output = PathBuf::from(std::env::var_os("TENCHU_SAVES_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let root = output.join("fixtures");
        for game in [Game::Tenchu1, Game::Tenchu2] {
            let name = if game == Game::Tenchu1 { "t1" } else { "t2" };
            let path = root.join(name);
            if game == Game::Tenchu1 {
                let mut save = tenchu_core::progress::save::Save::default();
                save.selection.character = 1;
                save.progress.highest = [3, 7];
                storage::Storage::at(path.clone()).write(&save).unwrap();
            } else {
                let mut save = tenchu_core::tenchu2::progress::Progress::default();
                for record in &mut save.records[1][..4] {
                    record.rank = 6;
                    record.time_tenths = 1234;
                }
                save.tatsumaru_unlocked = true;
                crate::tenchu2::native::save::SaveFile::at(path.clone())
                    .write(&save)
                    .unwrap();
            }
            let mut catalog = Catalog::load(path).unwrap();
            catalog.rename(0, "Ayame · evening campaign").unwrap();
            catalog.create("Practice route", Some(0)).unwrap();
            catalog.create("Fresh campaign", None).unwrap();
            let mut manager = Manager {
                open: true,
                game,
                catalog: Some(catalog),
                ..Default::default()
            };
            manager.refresh().unwrap();
            manager.choose(0);
            for size in [[1280, 720], [800, 450], [390, 844]] {
                let instance = wgpu::Instance::new(&Default::default());
                let adapter =
                    pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
                let format = wgpu::TextureFormat::Rgba8Unorm;
                let mut renderer = pollster::block_on(crate::renderer::Renderer::new(
                    &adapter,
                    format,
                    size,
                    &Default::default(),
                ))
                .unwrap();
                let ctx = egui::Context::default();
                crate::fonts::install(&ctx);
                ctx.set_theme(egui::Theme::Dark);
                ctx.style_mut(|style| style.animation_time = 0.0);
                let mut overlay =
                    egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
                let mut free = Vec::new();
                for _ in 0..2 {
                    let frame = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(size[0] as f32, size[1] as f32),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            assert!(manager.show(ctx, "").is_none());
                        },
                    );
                    let jobs = ctx.tessellate(frame.shapes, frame.pixels_per_point);
                    renderer.update_overlay_textures(&mut overlay, &mut free, frame.textures_delta);
                    crate::capture::render_to_png(
                        &mut renderer,
                        size,
                        format,
                        &output.join(format!("{name}-{}x{}.png", size[0], size[1])),
                        |renderer, view| {
                            renderer.draw_overlay(
                                view,
                                &mut overlay,
                                &jobs,
                                &egui_wgpu::ScreenDescriptor {
                                    size_in_pixels: size,
                                    pixels_per_point: frame.pixels_per_point,
                                },
                            );
                            Ok(())
                        },
                    )
                    .unwrap();
                }
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

enum Edit {
    New,
    Duplicate,
    Rename,
}

#[derive(Default)]
pub struct Manager {
    pub open: bool,
    game: Game,
    catalog: Option<Catalog>,
    cards: Vec<Card>,
    selected: u64,
    name: String,
    notice: String,
}

impl Manager {
    pub fn open(&mut self, game: Game) -> Result<()> {
        self.open = true;
        self.game = game;
        self.notice.clear();
        self.catalog = None;
        self.cards.clear();
        let catalog = Catalog::load(game.path()?)?;
        self.selected = catalog.index.active;
        self.catalog = Some(catalog);
        self.refresh()?;
        self.choose(self.selected);
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or("Campaign catalog unavailable")?;
        self.cards = catalog
            .index
            .slots
            .iter()
            .map(|slot| {
                let summary = catalog
                    .path(slot.id)
                    .and_then(|path| match storage::read(&path) {
                        Ok(bytes) => self.game.summary(&bytes),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            Ok(vec!["New campaign · no progress yet".into()])
                        }
                        Err(e) => Err(e.to_string()),
                    });
                Card {
                    id: slot.id,
                    name: slot.name.clone(),
                    summary,
                }
            })
            .collect();
        Ok(())
    }

    fn choose(&mut self, id: u64) {
        self.selected = id;
        if let Some(card) = self.cards.iter().find(|c| c.id == id) {
            self.name.clone_from(&card.name);
        }
    }

    pub fn path(&self, id: u64) -> Result<PathBuf> {
        self.catalog
            .as_ref()
            .ok_or("Campaign catalog unavailable")?
            .path(id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn catalog(&mut self) -> Result<&mut Catalog> {
        self.catalog
            .as_mut()
            .ok_or("Campaign catalog unavailable".into())
    }

    pub fn activate(&mut self, id: u64) -> Result<()> {
        self.catalog
            .as_mut()
            .ok_or("Campaign catalog unavailable")?
            .select(id)?;
        self.open = false;
        Ok(())
    }

    pub fn show(&mut self, ctx: &egui::Context, notice: &str) -> Option<u64> {
        if !self.open {
            return None;
        }
        let mut selected = None;
        let mut action = None;
        let mut choose = None;
        let id = egui::Id::new("campaign-saves");
        let response = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).default_height(ctx.screen_rect().height() - 48.0))
            .show(ctx, |ui| {
            ui.set_width((ctx.screen_rect().width() - 48.0).min(600.0));
            ui.horizontal(|ui| {
                ui.heading("Campaign saves");
                if ui.button("Done").clicked() { self.open = false; }
            });
            egui::ScrollArea::vertical().id_salt("campaign-body").max_height((ctx.screen_rect().height() - 100.0).max(100.0)).show(ui, |ui| {
            ui.label(self.game.name());
            ui.weak("Records and supplies are saved automatically to the active campaign. These are campaign saves, not mid-mission snapshots.");
            let active = self.catalog.as_ref().map(|c| c.index.active);
            egui::ScrollArea::vertical().id_salt("campaign-list").max_height((ctx.screen_rect().height() * 0.4).max(100.0)).show(ui, |ui| {
                for card in &self.cards {
                    let label = if active == Some(card.id) { format!("{}  ·  Active", card.name) } else { card.name.clone() };
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if ui.selectable_label(self.selected == card.id, egui::RichText::new(label).strong()).clicked() { choose = Some(card.id); }
                        match &card.summary {
                            Ok(lines) => for line in lines { ui.label(line); },
                            Err(error) => { ui.colored_label(egui::Color32::LIGHT_RED, format!("Cannot load: {error}")); }
                        }
                    });
                }
            });
            let valid = self.cards.iter().any(|c| c.id == self.selected && c.summary.is_ok());
            ui.add_enabled_ui(self.catalog.is_some(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.name);
                });
                ui.horizontal_wrapped(|ui| {
                    if ui.button("New campaign").clicked() { action = Some(Edit::New); }
                    if ui.add_enabled(valid, egui::Button::new("Duplicate selected")).clicked() { action = Some(Edit::Duplicate); }
                    if ui.button("Rename selected").clicked() { action = Some(Edit::Rename); }
                });
                ui.weak("Enter a new name above to create a fresh campaign or duplicate the selected save.");
                if ui.add_enabled(valid, egui::Button::new("Use selected campaign · return to title")).clicked() { selected = Some(self.selected); }
                ui.weak("Switching leaves the current mission. Its last campaign checkpoint is kept.");
            });
            if !self.notice.is_empty() { ui.colored_label(egui::Color32::LIGHT_RED, &self.notice); }
            if !notice.is_empty() { ui.colored_label(egui::Color32::LIGHT_RED, notice); }
            });
        });
        if response.should_close() {
            self.open = false;
        }
        if let Some(id) = choose {
            self.choose(id);
        }
        if let Some(action) = action {
            let catalog = self.catalog.as_mut().unwrap();
            let result = match action {
                Edit::New => catalog.create(&self.name, None),
                Edit::Duplicate => catalog.create(&self.name, Some(self.selected)),
                Edit::Rename => catalog
                    .rename(self.selected, &self.name)
                    .map(|_| self.selected),
            };
            match result.and_then(|id| {
                self.refresh()?;
                Ok(id)
            }) {
                Ok(id) => {
                    self.choose(id);
                    self.notice.clear();
                }
                Err(error) => self.notice = error,
            }
        }
        selected
    }
}
