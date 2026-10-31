use super::{Appearance, library::Library};
use crate::{
    frontend::{
        draw::Frame,
        gpu,
        raster::{Blend, Vertex},
    },
    settings::{
        Graphics, Settings,
        menu::{Item, Menu},
        overlay::Edit,
    },
    visuals::{pack::Pack, pixels::Image},
};
use glam::{Mat3, Vec2, Vec3};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tenchu_assets::{Result, motion::Clip};
use tenchu_core::{game::Game, math::Matrix, motion::Motion};

#[derive(Default)]
pub struct Picker {
    search: String,
    previews: BTreeMap<String, Result<Preview>>,
}

pub struct State<'a> {
    pub picker: &'a mut Picker,
    pub library: Option<&'a Library>,
    pub bodies: &'a Pack,
    pub game: &'a Game,
}

struct Preview {
    appearance: Appearance,
    motion: Motion,
    clips: Vec<Clip>,
    center: Vec3,
    extent: f32,
    textures: Vec<(String, Arc<Image>)>,
}

impl Preview {
    fn new(id: &str, state: &State<'_>) -> Result<Self> {
        let original = format!("disc:{}", state.game.player().rendered_mesh_kind);
        let mut appearance = Appearance::build(
            state.game,
            if id.is_empty() { &original } else { id },
            &Pack::default(),
            state.bodies,
            &Graphics::default(),
            if id.is_empty() { None } else { state.library },
        )?
        .ok_or("Model unavailable")?;
        appearance.meshes.detach_textures();
        let mut motion = Motion::new(&appearance.parts)?;
        let body = state
            .library
            .and_then(|l| l.get(if id.is_empty() { &original } else { id }));
        let clip = if let Some(body) = body.filter(|_| !id.is_empty()) {
            body.idle.map(|i| &body.clips[i])
        } else {
            let definition = &state.game.mission.characters[state.game.player().definition];
            definition
                .motions
                .iter()
                .find(|r| r.action == tenchu_core::ids::MOT_NORMAL)
                .and_then(|r| state.game.mission.clips.iter().find(|c| c.id == r.clip))
        };
        let clips: Vec<_> = clip.cloned().into_iter().collect();
        if !clips.is_empty() {
            motion.clip = Some(0);
            motion.step(&clips, &state.game.math.tables.hermite)?;
        }
        let matrices = motion.pose.matrices(&state.game.math, Matrix::default());
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for (part, matrix) in appearance.parts.iter().zip(matrices) {
            appearance
                .meshes
                .visit(&part.geometry, matrix, false, 0, None, |triangle| {
                    for vertex in triangle {
                        let p = Vec3::from(vertex.position);
                        min = min.min(p);
                        max = max.max(p);
                    }
                });
        }
        if !min.is_finite() {
            return Err("No visible geometry".into());
        }
        let textures = appearance
            .meshes
            .materials()
            .iter()
            .map(|m| (crate::frontend::draw::identity(&m.image), m.image.clone()))
            .collect();
        Ok(Self {
            textures,
            appearance,
            motion,
            clips,
            center: (min + max) * 0.5,
            extent: (max - min).length().max(1.0) * 0.62,
        })
    }

    fn paint(
        &mut self,
        frame: &mut Frame,
        rect: egui::Rect,
        origin: egui::Pos2,
        time: f64,
        game: &Game,
    ) {
        if let Some(clip) = self.clips.first() {
            self.motion.count = ((time * 30.0) as i64 % (i64::from(clip.duration) + 1)) as i16;
            self.motion.loops = 0;
            let _ = self.motion.step(&self.clips, &game.math.tables.hermite);
        }
        let matrices = self.motion.pose.matrices(&game.math, Matrix::default());
        let camera = Mat3::from_rotation_x(-0.10) * Mat3::from_rotation_y(-0.35);
        let scale = rect.width().min(rect.height()) * 0.48 / self.extent;
        let center = Vec2::new(rect.center().x - origin.x, rect.center().y - origin.y);
        let mut triangles = Vec::new();
        for (part, matrix) in self.appearance.parts.iter().zip(matrices) {
            self.appearance
                .meshes
                .visit(&part.geometry, matrix, false, 0, None, |triangle| {
                    let depth = triangle
                        .iter()
                        .map(|v| (camera * Vec3::from(v.position)).z)
                        .sum::<f32>();
                    triangles.push((depth, triangle));
                });
        }
        triangles.sort_by(|a, b| b.0.total_cmp(&a.0));
        let white = Arc::new(Image {
            size: [1, 1],
            rgba: vec![255; 4],
        });
        for (_, triangle) in triangles {
            let material = triangle[0]
                .material
                .checked_sub(1)
                .and_then(|i| self.textures.get(i as usize));
            let (id, image) = material
                .map_or(("model-preview-white", white.clone()), |(id, image)| {
                    (id.as_str(), image.clone())
                });
            let vertices = triangle.map(|v| {
                let p = camera * (Vec3::from(v.position) - self.center);
                let light = 0.7
                    + 0.3
                        * Vec3::from(v.normal)
                            .dot(Vec3::new(-0.4, -0.6, -0.7).normalize())
                            .max(0.0);
                Vertex {
                    position: center + Vec2::new(p.x, p.y) * scale,
                    uv: Vec2::from(v.uv) + Vec2::splat(0.5),
                    color: Vec3::from(v.color) * 128.0 * light,
                }
            });
            frame.textured(id, image, vertices, Blend::Opaque);
        }
    }
}

impl Picker {
    pub fn clear(&mut self) {
        self.previews.clear();
    }
}

impl State<'_> {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        menu: &mut Menu,
        settings: &Settings,
        edits: &mut Vec<Edit>,
    ) {
        let previous_search = self.picker.search.clone();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Clear").clicked() {
                    self.picker.search.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.picker.search)
                        .hint_text("Search model name or ID…")
                        .desired_width(f32::INFINITY),
                );
            });
        });
        let query = self.picker.search.to_lowercase();
        let choices: Vec<_> = menu
            .player_models
            .iter()
            .filter(|(id, name)| {
                query.split_whitespace().all(|word| {
                    name.to_lowercase().contains(word) || id.to_lowercase().contains(word)
                })
            })
            .cloned()
            .collect();
        ui.weak(format!(
            "{} models · {}",
            choices.len(),
            if settings.model_moveset {
                "Actor moves where available; player controls"
            } else {
                "Appearance only; player moves"
            }
        ));
        let selected = self
            .library
            .and_then(|l| l.get(&settings.player_model))
            .map_or(settings.player_model.as_str(), |b| b.id.as_str());
        let columns = ((ui.available_width() + 6.0) / 112.0).floor().max(1.0) as usize;
        let mut visible = BTreeSet::new();
        let mut tiles = Vec::new();
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("model-gallery")
            .auto_shrink([false, true])
            .max_height(
                (ui.ctx().screen_rect().bottom() - ui.cursor().top() - 100.0).clamp(120.0, 420.0),
            );
        if previous_search != self.picker.search {
            scroll = scroll.vertical_scroll_offset(0.0);
        }
        let result = scroll.show_rows(ui, 152.0, choices.len().div_ceil(columns), |ui, rows| {
            let width = (ui.available_width() - 6.0 * (columns - 1) as f32) / columns as f32;
            for row in rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (id, name) in choices.iter().skip(row * columns).take(columns) {
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(width, 152.0), egui::Sense::click());
                        if !ui.is_rect_visible(rect) {
                            continue;
                        }
                        visible.insert(id.clone());
                        let active = id == selected;
                        let color = if active {
                            egui::Color32::from_rgb(61, 77, 94)
                        } else {
                            egui::Color32::from_rgb(31, 37, 44)
                        };
                        ui.painter().rect_filled(rect.shrink(1.0), 4.0, color);
                        if active || response.hovered() {
                            ui.painter().rect_stroke(
                                rect.shrink(1.0),
                                4.0,
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(234, 198, 133)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let label = if id.is_empty() {
                            "Original"
                        } else {
                            name.split(" · ").next().unwrap_or(name)
                        };
                        let galley = ui.painter().layout(
                            label.into(),
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                            width - 8.0,
                        );
                        ui.painter().galley(
                            egui::pos2(
                                rect.center().x - galley.size().x / 2.0,
                                rect.bottom() - 32.0,
                            ),
                            galley,
                            egui::Color32::WHITE,
                        );
                        if response.clicked() {
                            edits.push(Edit::PlayerModel(id.clone()));
                        }
                        if response.hovered() {
                            menu.hovered = Some(Item::PlayerModel);
                            menu.row = 0;
                        }
                        response
                            .on_hover_text(format!("{name}\n{id}\nClick to apply · R: Original"));
                        tiles.push((
                            id.clone(),
                            egui::Rect::from_min_max(
                                rect.min + egui::vec2(4.0, 4.0),
                                rect.max - egui::vec2(4.0, 34.0),
                            ),
                        ));
                    }
                });
            }
        });
        let viewport = result.inner_rect;
        let mut frame = Frame::new(
            [
                viewport.width().ceil() as usize,
                viewport.height().ceil() as usize,
            ],
            Arc::default(),
        );
        let time = ui.input(|i| i.time);
        for (id, rect) in tiles {
            if !self.picker.previews.contains_key(&id) {
                let preview = Preview::new(&id, self);
                self.picker.previews.insert(id.clone(), preview);
            }
            match self.picker.previews.get_mut(&id).unwrap() {
                Ok(preview) => preview.paint(&mut frame, rect, viewport.min, time, self.game),
                Err(error) => {
                    ui.painter().with_clip_rect(viewport).text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        error,
                        egui::FontId::proportional(11.0),
                        egui::Color32::LIGHT_RED,
                    );
                }
            }
        }
        self.picker.previews.retain(|id, _| visible.contains(id));
        if !frame.vertices.is_empty() {
            ui.painter()
                .with_clip_rect(viewport)
                .add(egui_wgpu::Callback::new_paint_callback(
                    viewport,
                    gpu::Paint::Models(Arc::new(frame)),
                ));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        if choices.is_empty() {
            ui.label("No models match this search.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_assets::game::GameAssets;

    #[test]
    #[ignore = "requires TENCHU_DISC; checks repeated gallery layout, filtering, clipping and paused previews"]
    fn gallery_stays_bounded_and_only_animates_visible_models() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        let library = Library::load(&assets).unwrap();
        let game = crate::session::Options::default().load(&assets).unwrap();
        let debug = crate::debug::Debug::new(&game).unwrap();
        let before = crate::session::tests::state(&game);
        let bodies = Pack::default();
        for size in [
            egui::vec2(800.0, 600.0),
            egui::vec2(1280.0, 720.0),
            egui::vec2(3840.0, 2160.0),
        ] {
            let ctx = egui::Context::default();
            let mut picker = Picker::default();
            let mut menu = Menu {
                open: true,
                page: 10,
                player_models: super::super::choices(&game, &bodies, Some(&library)),
                ..Default::default()
            };
            let mut settled_width = 0.0_f32;
            let mut animated = false;
            for frame in 0..90 {
                menu.row = usize::from((20..50).contains(&frame));
                if frame == 40 {
                    picker.search = "onikage".into();
                } else if frame == 60 {
                    picker.search.clear();
                }
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        time: Some(f64::from(frame) / 60.0),
                        ..Default::default()
                    },
                    |ctx| {
                        let edits = crate::settings::overlay::show(
                            ctx,
                            &mut menu,
                            crate::settings::overlay::State {
                                settings: &Settings::default(),
                                debug: &debug,
                                paused: true,
                                lights: 0,
                                samples: &[1],
                            },
                            None,
                            Some(&mut State {
                                picker: &mut picker,
                                library: Some(&library),
                                bodies: &bodies,
                                game: &game,
                            }),
                        );
                        assert!(edits.is_empty());
                    },
                );
                let rect = ctx
                    .memory(|m| m.area_rect(egui::Id::new("Settings & tools")))
                    .unwrap();
                if frame == 10 {
                    settled_width = rect.width();
                    assert!(
                        settled_width < 750.0,
                        "unexpected gallery width: {settled_width}"
                    );
                } else if frame > 10 {
                    assert!(
                        (rect.width() - settled_width).abs() < 1.0,
                        "frame {frame}: window grew from {settled_width} to {}",
                        rect.width()
                    );
                }
                assert!(picker.previews.len() < menu.player_models.len() / 2);
                assert!(picker.previews.values().all(Result::is_ok));
                if (40..60).contains(&frame) {
                    assert_eq!(picker.previews.len(), 1);
                    let preview = picker.previews["disc:135"].as_ref().unwrap();
                    animated |= preview.motion.count > 0;
                }
            }
            assert!(animated);
        }
        assert_eq!(before, crate::session::tests::state(&game));
    }
}
