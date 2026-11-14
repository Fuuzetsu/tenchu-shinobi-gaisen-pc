use super::*;
use crate::visuals::appearance::picker::{Preview, State, Thumbnail};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Character(String),
    Scenery(AssetGame, usize, usize),
    Campaign(String, usize),
    Model(custom::Model),
    Texture(String),
}

enum Content {
    Model(Thumbnail),
    Image(egui::TextureHandle),
}

struct Request {
    key: Key,
    rect: egui::Rect,
    painter: egui::Painter,
}

#[derive(Default)]
pub(super) struct Previews {
    requests: Vec<Request>,
    cache: BTreeMap<Key, Result<Content>>,
}

impl Previews {
    pub fn begin(&mut self) {
        self.requests.clear();
    }

    pub fn show(&mut self, ui: &mut egui::Ui, key: Key, size: egui::Vec2) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, 4.0, egui::Color32::from_rgb(23, 29, 35));
            self.requests.push(Request {
                key,
                rect,
                painter: ui.painter().with_clip_rect(ui.clip_rect().intersect(rect)),
            });
        }
        response
    }

    pub fn tooltip(&mut self, response: egui::Response, key: Key, name: &str) -> egui::Response {
        response.on_hover_ui(|ui| {
            ui.strong(name);
            self.show(ui, key, egui::vec2(220.0, 200.0));
        })
    }
}

impl Key {
    pub fn scenery(prop: &project::Scenery) -> Self {
        prop.custom
            .clone()
            .map_or(Self::Scenery(prop.game, prop.stage, prop.part), Self::Model)
    }

    fn build(&self, editor: &Editor, ctx: &egui::Context, state: &State<'_>) -> Result<Content> {
        use crate::visuals::pack::Pack;
        let preview = match self {
            Self::Character(id) => Preview::new(id, state)?,
            Self::Scenery(game, stage, part) => {
                let source = editor
                    .scenery_library
                    .sources
                    .get(&(*game, *stage))
                    .ok_or("Browse this source area to load its preview")?;
                let part = source.parts.get(*part).ok_or("Missing object")?.clone();
                Preview::object(
                    part,
                    "studio-preview",
                    &source.vram,
                    &Pack::default(),
                    state.game,
                )?
            }
            Self::Campaign(source, part) => {
                let cache = editor
                    .campaign_cache
                    .as_ref()
                    .filter(|c| c.source.label() == *source)
                    .ok_or("Open this campaign to preview its objects")?;
                let part = cache
                    .mission
                    .objects
                    .get(*part)
                    .ok_or("Missing object")?
                    .clone();
                Preview::object(
                    part,
                    "studio-preview",
                    cache
                        .mission
                        .world_vram
                        .as_deref()
                        .unwrap_or(&cache.mission.vram),
                    &Pack::default(),
                    state.game,
                )?
            }
            Self::Model(model) => {
                let (part, pack) = model.build(&editor.project.assets, "studio-preview")?;
                Preview::object(
                    part,
                    "studio-preview",
                    &Default::default(),
                    &pack,
                    state.game,
                )?
            }
            Self::Texture(id) => {
                let image = editor
                    .project
                    .assets
                    .get(id)
                    .ok_or("Missing texture")?
                    .image()?;
                return Ok(Content::Image(ctx.load_texture(
                    id,
                    egui::ColorImage::from_rgba_unmultiplied(
                        image.size.map(|n| n as usize),
                        &image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                )));
            }
        };
        Ok(Content::Model(preview.thumbnail(ctx, state.game)))
    }
}

impl Editor {
    pub(crate) fn paint_previews(&mut self, ctx: &egui::Context, state: &State<'_>) {
        let requests = std::mem::take(&mut self.previews.requests);
        let visible: BTreeSet<_> = requests.iter().map(|r| r.key.clone()).collect();
        self.previews.cache.retain(|key, _| visible.contains(key));
        // Decode at most one newly visible model per frame, never the full library.
        if let Some(key) = requests
            .iter()
            .map(|r| &r.key)
            .find(|key| !self.previews.cache.contains_key(key))
        {
            let content = key.build(self, ctx, state);
            self.previews.cache.insert(key.clone(), content);
            ctx.request_repaint();
        }
        for request in requests {
            let Request { key, rect, painter } = request;
            match self.previews.cache.get(&key) {
                Some(Ok(Content::Model(model))) => model.paint(&painter, rect),
                Some(Ok(Content::Image(image))) => {
                    let size = image.size_vec2();
                    let size = size * (rect.width() / size.x).min(rect.height() / size.y);
                    painter.image(
                        image.id(),
                        egui::Rect::from_center_size(rect.center(), size),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                Some(Err(error)) => {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        error,
                        egui::FontId::proportional(11.0),
                        egui::Color32::LIGHT_RED,
                    );
                }
                None => {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Loading preview…",
                        egui::FontId::proportional(11.0),
                        egui::Color32::GRAY,
                    );
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    CharacterType,
    Appearance,
    Scenery,
}

impl Editor {
    pub(super) fn asset_gallery(&mut self, ctx: &egui::Context, assets: &GameAssets<DiscSource>) {
        let Some(target) = self.gallery else {
            return;
        };
        let mut open = true;
        let mut choice = None;
        egui::Window::new(match target { Target::CharacterType => "Choose character type", Target::Appearance => "Choose character appearance", Target::Scenery => "Choose scenery" })
            .id(egui::Id::new("studio-asset-gallery"))
            .open(&mut open).default_width(700.0).resizable(true).show(ctx, |ui| {
                crate::ui::theme::style(ui);
                self.history.begin(&self.project);
                if target == Target::Scenery {
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.scenery_game, AssetGame::Tenchu, "Tenchu 1");
                        ui.selectable_value(&mut self.scenery_game, AssetGame::Tenchu2, "Tenchu 2");
                        ui.add(egui::DragValue::new(&mut self.scenery_stage).range(0..=if self.scenery_game == AssetGame::Tenchu { 18 } else { 23 }).prefix("Source area "));
                        if ui.button("Load area").clicked() && let Err(error) = self.scenery_library.load(assets, self.scenery_game, self.scenery_stage) { self.notice = error; }
                    });
                    ui.small("Decorative objects · click a preview to place at the cursor");
                } else if target == Target::Appearance {
                    ui.horizontal_wrapped(|ui| {
                        for (value, label) in [(0, "All"), (1, "Tenchu 1"), (2, "Tenchu 2 · EU"), (3, "Tenchu 2 · JP")] {
                            if value == 0 || self.asset_names.iter().any(|(id, _)| source(id) == value) {
                                ui.selectable_value(&mut self.gallery_source, value, label);
                            }
                        }
                    });
                    if ui.button("Use original appearance").clicked() { choice = Some((String::new(), 0)); }
                } else { ui.small("Character types choose the original behavior. Appearance can be changed separately."); }
                let previous_search = self.gallery_search.clone();
                ui.add(egui::TextEdit::singleline(&mut self.gallery_search).id(egui::Id::new("studio-gallery-search")).hint_text("Search names or IDs…").desired_width(f32::INFINITY));
                let query = self.gallery_search.to_lowercase();
                let entries: Vec<(Key, String, String, usize)> = match target {
                    Target::Appearance => self.asset_names.iter().filter(|(id, _)| self.gallery_source == 0 || source(id) == self.gallery_source).map(|(id, name)| (Key::Character(id.clone()), name.clone(), id.clone(), 0)).collect(),
                    Target::CharacterType => {
                        let kinds: Vec<i16> = if let Some(cache) = self.campaign_cache.as_ref().filter(|_| self.project.campaign.is_some()) { cache.mission.models.keys().copied().collect() } else { self.tables.as_ref().unwrap().appearance[self.project.mission.theme].to_vec() };
                        kinds.into_iter().collect::<BTreeSet<_>>().into_iter().filter(|k| *k > 6 && *k != 0xa9 && *k != 170).map(|kind| {
                            let name = self.tables.as_ref().unwrap().characters.iter().find(|c| c.kind == kind).map_or_else(|| format!("Character {kind}"), |c| c.model.clone());
                            let id = format!("disc:{kind}");
                            (Key::Character(id.clone()), name, id, kind as usize)
                        }).collect()
                    }
                    Target::Scenery => self.scenery_library.sources.get(&(self.scenery_game, self.scenery_stage)).into_iter().flat_map(|s| s.names.iter().enumerate().filter(|(i, _)| !s.parts[*i].geometry.is_empty()).map(|(i, name)| (Key::Scenery(self.scenery_game, self.scenery_stage, i), name.clone(), format!("object:{i}"), i))).collect(),
                };
                let entries: Vec<_> = entries.into_iter().filter(|(_, name, id, _)| query.split_whitespace().all(|q| name.to_lowercase().contains(q) || id.to_lowercase().contains(q))).collect();
                ui.weak(format!("{} assets", entries.len()));
                if entries.is_empty() { ui.label(if target == Target::Scenery { "Load an area or clear your search to see its models." } else { "No models match this search." }); }
                let columns = ((ui.available_width() + 8.0) / 140.0).floor().max(1.0) as usize;
                let mut scroll = egui::ScrollArea::vertical().id_salt(("studio-asset-gallery-scroll", self.gallery_source, self.scenery_game == AssetGame::Tenchu, self.scenery_stage)).max_height((ctx.screen_rect().height() - 260.0).clamp(140.0, 540.0)).auto_shrink([false, false]);
                if previous_search != self.gallery_search { scroll = scroll.vertical_scroll_offset(0.0); }
                scroll.show_rows(ui, 168.0, entries.len().div_ceil(columns), |ui, rows| {
                    let width = (ui.available_width() - 8.0 * (columns - 1) as f32) / columns as f32;
                    for row in rows {
                        ui.horizontal(|ui| {
                            for (key, name, id, index) in entries.iter().skip(row * columns).take(columns) {
                                ui.push_id(id, |ui| {
                                    let card = ui.allocate_ui_with_layout(egui::vec2(width, 160.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                                        self.previews.show(ui, key.clone(), egui::vec2(width, 120.0));
                                        ui.allocate_ui(egui::vec2(width, 32.0), |ui| {
                                            ui.spacing_mut().item_spacing.y = 0.0;
                                            ui.add_sized([width, 16.0], egui::Label::new(name.split(" · ").next().unwrap_or(name)).truncate());
                                            let label = match key {
                                                Key::Character(id) => match source(id) { 2 => "Tenchu 2 · EU", 3 => "Tenchu 2 · JP", _ => "Tenchu 1" },
                                                Key::Scenery(AssetGame::Tenchu2, _, _) => "Tenchu 2",
                                                _ => "Tenchu 1",
                                            };
                                            ui.add_sized([width, 14.0], egui::Label::new(egui::RichText::new(label).small().weak()));
                                        });
                                    });
                                    let rect = card.response.rect;
                                    let selected = match target {
                                        Target::CharacterType => self.actor_kind as usize == *index,
                                        Target::Appearance => self.project.actors.get(&self.actor) == Some(id),
                                        Target::Scenery => self.scenery_selection.and_then(|i| self.project.scenery.get(i)).is_some_and(|p| p.custom.is_none() && p.game == self.scenery_game && p.stage == self.scenery_stage && p.part == *index),
                                    };
                                    let response = ui.interact(rect, ui.id().with("choose"), egui::Sense::click());
                                    if selected || response.hovered() { ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, crate::ui::theme::GOLD), egui::StrokeKind::Inside); }
                                    if response.clicked() { choice = Some((id.clone(), *index)); }
                                    self.previews.tooltip(response, key.clone(), name);
                                });
                            }
                        });
                    }
                });
            });
        if let Some((id, index)) = choice {
            self.history.begin(&self.project);
            match target {
                Target::CharacterType => {
                    self.actor_kind = index as i16;
                    open = false;
                }
                Target::Appearance => {
                    if id.is_empty() {
                        self.project.actors.remove(&self.actor);
                    } else {
                        self.project.actors.insert(self.actor, id);
                    }
                    open = false;
                }
                Target::Scenery => {
                    if self.project.scenery.len() < self.project.limits.scenery {
                        let source =
                            &self.scenery_library.sources[&(self.scenery_game, self.scenery_stage)];
                        self.scenery_selection = Some(self.project.scenery.len());
                        self.campaign_prop = None;
                        self.project.scenery.push(project::Scenery {
                            custom: None,
                            name: source.names[index].clone(),
                            game: self.scenery_game,
                            stage: self.scenery_stage,
                            part: index,
                            position: self.world_cursor(),
                            yaw: 0,
                        });
                        self.tool = Tool::Select;
                    } else {
                        self.notice =
                            "Scenery limit reached. Adjust project Limits to raise it.".into();
                    }
                }
            }
        }
        if !open {
            self.gallery = None;
        }
    }
}

fn source(id: &str) -> u8 {
    if id.starts_with("tenchu2-jp:") {
        3
    } else if id.starts_with("tenchu2:") {
        2
    } else {
        1
    }
}
