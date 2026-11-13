use super::*;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use tenchu_assets::trial::{Waypoint, editing};

impl Editor {
    pub(super) fn prepare_textures(&mut self, ctx: &egui::Context) {
        if !self.textures.is_empty() {
            return;
        }
        let theme = self.theme.as_ref().unwrap();
        self.textures = theme
            .tiles
            .iter()
            .enumerate()
            .map(|(i, tile)| {
                let image = tile
                    .textures
                    .iter()
                    .find(|&&i| i >= 0)
                    .map(|&i| theme.images[i as usize])?;
                let [w, h] = image.size.map(usize::from);
                let mut rgba = Vec::with_capacity(w * h * 4);
                for y in 0..h {
                    for x in 0..w {
                        let mut pixel = tenchu_assets::texture::rgba8(theme.vram.texel(
                            image.page,
                            image.palette,
                            [
                                u16::from(image.uv[0]) + x as u16,
                                u16::from(image.uv[1]) + y as u16,
                            ],
                        ));
                        if pixel[3] != 0 {
                            pixel[3] = 255;
                        }
                        rgba.extend(pixel);
                    }
                }
                Some(ctx.load_texture(
                    format!("trial-tile-{i}"),
                    egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba),
                    egui::TextureOptions::NEAREST,
                ))
            })
            .collect();
    }
    pub(super) fn grid(&mut self, ui: &mut egui::Ui) {
        let size = ui
            .available_width()
            .min(ui.available_height() - 40.0)
            .max(160.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
        let cell_size = size / 32.0;
        let painter = ui.painter_at(rect);
        let cell_rect = |p: Point| {
            Rect::from_min_size(
                rect.min + egui::vec2(p.x as f32, p.z as f32) * cell_size,
                Vec2::splat(cell_size),
            )
        };
        let mission = self.mission.as_ref().unwrap();
        let theme = self.theme.as_ref().unwrap();
        for (i, cell) in mission.map.iter().enumerate() {
            let r = cell_rect(Point {
                x: (i % 32) as i16,
                z: (i / 32) as i16,
            });
            let tile = theme.tile(cell.ground);
            let color = tile.map_or(Color32::BLACK, |tile| match tile.kind() {
                7 => Color32::from_rgb(70, 70, 85),
                8 => Color32::from_rgb(12, 12, 18),
                9 => Color32::from_rgb(40, 85, 145),
                _ => Color32::from_rgb(90, 110, 80),
            });
            painter.rect_filled(r, 0.0, color);
            if cell.ground != 255
                && let Some(Some(texture)) = self.textures.get(usize::from(cell.ground & 63))
            {
                let mut mesh = egui::Mesh::with_texture(texture.id());
                let uvs = [
                    Pos2::new(0.0, 0.0),
                    Pos2::new(1.0, 0.0),
                    Pos2::new(1.0, 1.0),
                    Pos2::new(0.0, 1.0),
                ];
                for (j, pos) in [
                    r.left_top(),
                    r.right_top(),
                    r.right_bottom(),
                    r.left_bottom(),
                ]
                .into_iter()
                .enumerate()
                {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos,
                        uv: uvs[(j + usize::from(cell.ground >> 6)) % 4],
                        color: Color32::from_gray(180),
                    });
                }
                mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                painter.add(mesh);
            }
            if let Some(tile) = tile
                && tile.height() < 0
                && cell_size >= 13.0
            {
                painter.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    if tile.kind() == 7 {
                        "W".into()
                    } else {
                        format!("{}", -tile.height())
                    },
                    FontId::monospace((cell_size * 0.65).min(14.0)),
                    Color32::WHITE,
                );
            }
            if let Some(tile) = theme.tile(cell.ornament) {
                let color = if tile.flags & 0x3000 != 0 {
                    Color32::ORANGE
                } else {
                    Color32::LIGHT_GREEN
                };
                painter.circle_filled(r.center(), cell_size * 0.28, color);
                let angle = f32::from(cell.ornament >> 6) * std::f32::consts::FRAC_PI_2;
                painter.line_segment(
                    [
                        r.center(),
                        r.center() + egui::vec2(angle.sin(), -angle.cos()) * cell_size * 0.45,
                    ],
                    Stroke::new(1.5_f32, color),
                );
            }
        }
        for i in 0..=32 {
            let offset = i as f32 * cell_size;
            let stroke = Stroke::new(
                if i % 4 == 0 { 1.0_f32 } else { 0.5_f32 },
                Color32::from_black_alpha(90),
            );
            painter.line_segment(
                [
                    rect.min + egui::vec2(offset, 0.0),
                    rect.min + egui::vec2(offset, size),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    rect.min + egui::vec2(0.0, offset),
                    rect.min + egui::vec2(size, offset),
                ],
                stroke,
            );
        }
        for (index, placement) in mission.placements.iter().enumerate() {
            let center = cell_rect(placement.point).center();
            let color = if index == 0 {
                Color32::LIGHT_BLUE
            } else if index < mission.objective.reserved() {
                Color32::GOLD
            } else {
                Color32::LIGHT_RED
            };
            painter.circle_filled(center, cell_size * 0.48, Color32::from_black_alpha(230));
            painter.text(
                center,
                Align2::CENTER_CENTER,
                if index == 0 {
                    "P".into()
                } else {
                    index.to_string()
                },
                FontId::proportional(cell_size.min(17.0)),
                color,
            );
            if self.tab == Tab::Patrols && index == self.actor {
                for pair in placement.route.windows(2) {
                    painter.line_segment(
                        [
                            cell_rect(pair[0].point).center(),
                            cell_rect(pair[1].point).center(),
                        ],
                        Stroke::new(2.0_f32, Color32::GOLD),
                    );
                }
                for (n, p) in placement.route.iter().enumerate() {
                    let center = cell_rect(p.point).center();
                    painter.circle_stroke(
                        center,
                        cell_size * 0.4,
                        Stroke::new(1.0_f32, Color32::YELLOW),
                    );
                    if p.wait != 0 {
                        painter.text(
                            center,
                            Align2::CENTER_CENTER,
                            format!("w{n}"),
                            FontId::monospace(10.0),
                            Color32::YELLOW,
                        );
                    }
                }
            }
        }
        painter.rect_stroke(
            cell_rect(self.cursor),
            0.0,
            Stroke::new(2.0_f32, Color32::WHITE),
            StrokeKind::Inside,
        );
        if response.hovered()
            && let Some(pos) = response.hover_pos()
        {
            let xy = (pos - rect.min) / cell_size;
            self.cursor = Point {
                x: (xy.x as i16).clamp(0, 31),
                z: (xy.y as i16).clamp(0, 31),
            };
            if self.unlocked
                && (ui.input(|i| i.pointer.primary_down() || i.pointer.secondary_down())
                    || response.clicked()
                    || response.secondary_clicked())
            {
                let erase =
                    response.secondary_clicked() || ui.input(|i| i.pointer.secondary_down());
                if self.tab == Tab::Terrain || response.clicked() || response.secondary_clicked() {
                    self.use_cell(erase);
                }
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            self.finish_stroke();
        }
        ui.label(format!(
            "Tile {}, {} · {} · arrows move · Enter places · R rotates",
            self.cursor.x,
            self.cursor.z,
            if self.ornament {
                "ornament layer"
            } else {
                "ground layer"
            }
        ));
    }
    pub(super) fn use_cell(&mut self, erase: bool) {
        if !self.unlocked {
            return;
        }
        let before = self.mission.as_ref().unwrap().clone();
        let mission = self.mission.as_mut().unwrap();
        let theme = self.theme.as_ref().unwrap();
        let result = match self.tab {
            Tab::Terrain => editing::paint(
                mission,
                theme,
                self.cursor,
                self.ornament,
                if erase {
                    255
                } else {
                    self.brush | self.rotation << 6
                },
            ),
            Tab::Actors => {
                if let Some(index) = mission
                    .placements
                    .iter()
                    .position(|p| p.point == self.cursor)
                {
                    self.actor = index;
                    if erase && index >= mission.objective.reserved() {
                        mission.placements.remove(index);
                        self.actor = 0;
                    }
                    Ok(())
                } else if !erase {
                    editing::move_actor(mission, theme, self.actor, self.cursor)
                } else {
                    Ok(())
                }
            }
            Tab::Patrols if self.actor != 0 => {
                if erase {
                    let route = &mut mission.placements[self.actor].route;
                    route.pop();
                    if route.len() < 2 {
                        route.clear();
                    }
                    Ok(())
                } else {
                    editing::patrol(
                        mission,
                        theme,
                        self.actor,
                        Waypoint {
                            point: self.cursor,
                            facing: self.patrol_facing,
                            wait: self.patrol_wait,
                        },
                    )
                }
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.notice = error;
            return;
        }
        if self.mission.as_ref() != Some(&before) && self.stroke.is_none() {
            self.stroke = Some(before);
        }
    }
    pub(super) fn add_actor(&mut self) {
        let before = self.mission.as_ref().unwrap().clone();
        let mission = self.mission.as_mut().unwrap();
        match editing::add_actor(
            mission,
            self.theme.as_ref().unwrap(),
            self.cursor,
            self.actor_kind,
        ) {
            Ok(actor) => self.actor = actor,
            Err(error) => {
                self.notice = error;
                return;
            }
        }
        self.edit(before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires TENCHU_DISC; editing protects placements and truncates obstructed patrols"]
    fn trial_editing_preserves_supported_placements_and_routes() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        let mut editor = Editor::new(std::env::temp_dir().join("trial-editor-unsaved-test.json"));
        editor.enter(&assets).unwrap();
        editor.cursor.z = 27;
        assert!(matches!(
            editor.controller(crate::campaign::Action::Confirm, &assets),
            Some(Event::Play { preview: false })
        ));
        assert!(editor.mission.as_ref() == Some(&editor.presets[0]));
        editor.controller(crate::campaign::Action::Down, &assets);
        assert!(editor.mission.as_ref() == Some(&editor.presets[1]));
        editor
            .select(
                Mission::new(0, Objective::ReachLord, 140).unwrap(),
                Origin::Draft,
                &assets,
            )
            .unwrap();
        editor.tab = Tab::Terrain;
        editor.cursor = editor.mission.as_ref().unwrap().placements[0].point;
        editor.use_cell(true);
        assert_ne!(
            editor
                .mission
                .as_ref()
                .unwrap()
                .cell(editor.cursor.x, editor.cursor.z)
                .unwrap()
                .ground,
            255
        );
        editor.tab = Tab::Actors;
        editor.cursor = Point { x: 10, z: 10 };
        editor.add_actor();
        let index = editor.actor;
        editor.tab = Tab::Patrols;
        editor.cursor = Point { x: 11, z: 10 };
        editor.use_cell(false);
        editor.finish_stroke();
        editor.cursor = Point { x: 12, z: 10 };
        editor.use_cell(false);
        editor.finish_stroke();
        assert_eq!(
            editor.mission.as_ref().unwrap().placements[index]
                .route
                .len(),
            3
        );
        editor.tab = Tab::Terrain;
        editor.cursor = Point { x: 11, z: 10 };
        editor.use_cell(true);
        editor.finish_stroke();
        assert!(
            editor.mission.as_ref().unwrap().placements[index]
                .route
                .is_empty()
        );
        editor.undo();
        assert_eq!(
            editor.mission.as_ref().unwrap().placements[index]
                .route
                .len(),
            3
        );
        let bytes = editor.mission.as_ref().unwrap().encode().unwrap();
        assert_eq!(
            Mission::decode(&bytes).unwrap(),
            *editor.mission.as_ref().unwrap()
        );
    }
}
