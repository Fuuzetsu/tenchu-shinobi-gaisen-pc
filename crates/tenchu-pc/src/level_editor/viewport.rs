use super::*;
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use glam::Vec3;

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
                let [width, height] = image.size.map(usize::from);
                let mut rgba = Vec::with_capacity(width * height * 4);
                for y in 0..height {
                    for x in 0..width {
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
                    format!("studio-tile-{i}"),
                    egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba),
                    egui::TextureOptions::NEAREST,
                ))
            })
            .collect();
    }

    pub(super) fn viewport(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let position = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|pos| rect.contains(*pos));
        let mut cell = None;
        let mut surface_point = None;
        let mut targets = vec![];
        if self.grid {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(15, 20, 26));
            let base = rect.width().min(rect.height()) / 34.0;
            if response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let previous = self.zoom;
                    self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.5, 12.0);
                    if let Some(pointer) = position {
                        self.pan = (self.pan - (pointer - rect.center())) * (self.zoom / previous)
                            + (pointer - rect.center());
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Middle) {
                self.pan += ui.input(|i| i.pointer.delta());
            }
            let size = base * self.zoom;
            let origin = rect.center() + self.pan - Vec2::splat(size * 16.0);
            let cell_rect = |point: Point| {
                Rect::from_min_size(
                    origin + Vec2::new(point.x as f32, point.z as f32) * size,
                    Vec2::splat(size),
                )
            };
            let theme = self.theme.as_ref().unwrap();
            for (i, tile) in self.project.mission.map.iter().enumerate() {
                let r = cell_rect(Point {
                    x: (i % 32) as i16,
                    z: (i / 32) as i16,
                });
                if !r.intersects(rect) {
                    continue;
                }
                painter.rect_filled(r, 0.0, Color32::from_rgb(38, 52, 51));
                if let Some(Some(texture)) = (tile.ground != 255)
                    .then(|| self.textures.get((tile.ground & 63) as usize))
                    .flatten()
                {
                    let mut mesh = egui::Mesh::with_texture(texture.id());
                    let uv = [
                        Pos2::new(0.0, 0.0),
                        Pos2::new(1.0, 0.0),
                        Pos2::new(1.0, 1.0),
                        Pos2::new(0.0, 1.0),
                    ];
                    for (n, pos) in [
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
                            uv: uv[(n + (tile.ground >> 6) as usize) % 4],
                            color: Color32::WHITE,
                        });
                    }
                    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                    painter.add(mesh);
                }
                if let Some(ground) = theme.tile(tile.ground)
                    && ground.height() != 0
                    && size >= 18.0
                {
                    let text = if ground.kind() == 7 {
                        "W".into()
                    } else if ground.kind() == 8 {
                        "×".into()
                    } else {
                        format!("{}", -ground.height())
                    };
                    painter.text(
                        r.left_top() + Vec2::splat(2.0),
                        egui::Align2::LEFT_TOP,
                        text,
                        egui::FontId::monospace(12.0),
                        Color32::WHITE,
                    );
                }
                if tile.ornament != 255 {
                    painter.circle_filled(r.center(), size * 0.23, Color32::from_rgb(230, 165, 72));
                }
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(0.5_f32, Color32::from_black_alpha(90)),
                    StrokeKind::Inside,
                );
            }
            if let Some((a, b)) = self.selection {
                let (a, b) = building::bounds(a, b);
                let area = cell_rect(a).union(cell_rect(b));
                painter.rect_filled(area, 0.0, Color32::from_rgba_unmultiplied(70, 170, 230, 45));
                painter.rect_stroke(
                    area,
                    0.0,
                    Stroke::new(2.0_f32, Color32::LIGHT_BLUE),
                    StrokeKind::Inside,
                );
            }
            let selected = &self.project.mission.placements[self.actor];
            for pair in selected.route.windows(2).filter(|_| self.guides) {
                painter.line_segment(
                    [
                        cell_rect(pair[0].point).center(),
                        cell_rect(pair[1].point).center(),
                    ],
                    Stroke::new(2.0_f32, Color32::GOLD),
                );
            }
            for (i, actor) in self.project.mission.placements.iter().enumerate() {
                targets.push((false, i, cell_rect(actor.point).center()));
                if self.guides {
                    marker(
                        &painter,
                        cell_rect(actor.point).center(),
                        i,
                        self.actor == i,
                    );
                }
            }
            for (i, prop) in self.project.scenery.iter().enumerate() {
                let pos = origin
                    + Vec2::new(
                        prop.position[0] as f32 / 2000.0,
                        -prop.position[2] as f32 / 2000.0,
                    ) * size;
                targets.push((true, i, pos));
            }
            if let Some(pos) = position {
                let point = (pos - origin) / size;
                let point = Point {
                    x: point.x.floor() as i16,
                    z: point.y.floor() as i16,
                };
                if point.valid() {
                    cell = Some(point);
                    painter.rect_stroke(
                        cell_rect(point),
                        0.0,
                        Stroke::new(2.0_f32, Color32::WHITE),
                        StrokeKind::Inside,
                    );
                }
            }
        } else {
            self.navigate(ui, &response);
            let screen = ui.ctx().screen_rect();
            let matrix = self.camera.matrix(screen.width() / screen.height());
            let project = |point: Vec3| -> Option<Pos2> {
                let clip = matrix * point.extend(1.0);
                if clip.w <= 0.0 {
                    return None;
                }
                let point = clip.truncate() / clip.w;
                Some(
                    screen.min
                        + Vec2::new(
                            (point.x + 1.0) * screen.width() * 0.5,
                            (1.0 - point.y) * screen.height() * 0.5,
                        ),
                )
            };
            let world = |point: Point| {
                let mut position = point.world().map(|v| v as f32);
                position[1] = f32::from(self.theme.as_ref().unwrap().height(
                    &self.project.mission,
                    point.x,
                    point.z,
                    false,
                )) * 1000.0
                    - 50.0;
                Vec3::from_array(position)
            };
            for pair in self.project.mission.placements[self.actor]
                .route
                .windows(2)
                .filter(|_| self.guides)
            {
                if let (Some(a), Some(b)) =
                    (project(world(pair[0].point)), project(world(pair[1].point)))
                {
                    painter.line_segment([a, b], Stroke::new(2.0_f32, Color32::GOLD));
                }
            }
            for (i, actor) in self.project.mission.placements.iter().enumerate() {
                if let Some(pos) = project(world(actor.point)) {
                    targets.push((false, i, pos));
                    if self.guides {
                        marker(&painter, pos, i, self.actor == i);
                    }
                }
            }
            for (i, prop) in self.project.scenery.iter().enumerate() {
                if let Some(pos) = project(Vec3::from_array(prop.position.map(|v| v as f32))) {
                    targets.push((true, i, pos));
                }
            }
            for event in self
                .project
                .events
                .iter()
                .filter(|_| self.guides && self.panel == Panel::Events)
            {
                if let project::Trigger::Enter { min, max } = event.trigger {
                    for axis in 0..3 {
                        let a = (axis + 1) % 3;
                        let b = (axis + 2) % 3;
                        for bits in 0..4 {
                            let mut start = min.map(|v| v as f32);
                            if bits & 1 != 0 {
                                start[a] = max[a] as f32;
                            }
                            if bits & 2 != 0 {
                                start[b] = max[b] as f32;
                            }
                            let mut end = start;
                            end[axis] = max[axis] as f32;
                            if let (Some(start), Some(end)) =
                                (project(start.into()), project(end.into()))
                            {
                                painter.line_segment(
                                    [start, end],
                                    Stroke::new(1.0_f32, Color32::from_rgb(100, 200, 240)),
                                );
                            }
                        }
                    }
                }
            }
            if let Some((a, b)) = self.selection {
                let (a, b) = building::bounds(a, b);
                let corners: Vec<_> = [
                    (a.x, a.z),
                    (b.x + 1, a.z),
                    (b.x + 1, b.z + 1),
                    (a.x, b.z + 1),
                ]
                .into_iter()
                .filter_map(|(x, z)| {
                    project(glam::vec3(
                        f32::from(x) * 2000.0,
                        world(a).y,
                        -f32::from(z) * 2000.0,
                    ))
                })
                .collect();
                if corners.len() == 4 {
                    painter.add(egui::Shape::closed_line(
                        corners,
                        Stroke::new(2.0_f32, Color32::LIGHT_BLUE),
                    ));
                }
            }
            if let Some(pos) = position {
                let ndc = [
                    (pos.x - screen.min.x) / screen.width() * 2.0 - 1.0,
                    1.0 - (pos.y - screen.min.y) / screen.height() * 2.0,
                ];
                let (origin, direction) = crate::debug::camera::ray(matrix, ndc);
                let point = crate::debug::terrain::pick(&self.surfaces, origin, direction)
                    .map(|p| Vec3::from_array(p.to_array().map(|v| v as f32)))
                    .or_else(|| {
                        let distance = -origin.y / direction.y;
                        (direction.y.abs() > 0.0001 && (0.0..250_000.0).contains(&distance))
                            .then_some(origin + direction * distance)
                    });
                if let Some(point) = point {
                    surface_point = Some(point.to_array());
                    let cursor = Point {
                        x: (point.x / 2000.0).floor() as i16,
                        z: (-point.z / 2000.0).floor() as i16,
                    };
                    if cursor.valid() {
                        cell = Some(cursor);
                        let center = world(cursor);
                        let corners: Vec<_> = [
                            (-1000.0, -1000.0),
                            (1000.0, -1000.0),
                            (1000.0, 1000.0),
                            (-1000.0, 1000.0),
                        ]
                        .into_iter()
                        .filter_map(|(x, z)| project(center + glam::vec3(x, 0.0, z)))
                        .collect();
                        if corners.len() == 4 {
                            painter.add(egui::Shape::closed_line(
                                corners,
                                Stroke::new(2.0_f32, Color32::WHITE),
                            ));
                        }
                    }
                }
            }
        }
        let mut selected = false;
        let mut dragging = false;
        for (scenery, index, point) in targets {
            if !rect.contains(point) || scenery && self.panel != Panel::Scenery {
                continue;
            }
            if scenery && self.guides {
                let color = if self.scenery_selection == Some(index) {
                    Color32::GOLD
                } else {
                    Color32::LIGHT_GREEN
                };
                painter.rect_filled(
                    Rect::from_center_size(point, Vec2::splat(18.0)),
                    3.0,
                    Color32::from_black_alpha(220),
                );
                painter.text(
                    point,
                    egui::Align2::CENTER_CENTER,
                    "S",
                    egui::FontId::monospace(12.0),
                    color,
                );
            }
            if self.tool != Tool::Select {
                continue;
            }
            let hit = ui.interact(
                Rect::from_center_size(point, Vec2::splat(24.0)),
                ui.id().with(("studio-selection", scenery, index)),
                Sense::click_and_drag(),
            );
            if hit.clicked() || hit.drag_started() {
                if scenery {
                    self.scenery_selection = Some(index);
                    self.panel = Panel::Scenery;
                } else {
                    self.actor = index;
                    self.panel = Panel::Actors;
                }
                selected = true;
            }
            dragging |= hit.dragged();
        }
        if let Some(point) = cell {
            self.cursor = point;
            if self.tool == Tool::Area
                && (response.drag_started_by(egui::PointerButton::Primary) || response.clicked())
            {
                self.selection = Some((point, point));
            }
            if self.tool == Tool::Stamp
                && let Some(stamp) = &self.clipboard
            {
                painter.text(
                    rect.right_top() + Vec2::new(-16.0, 16.0),
                    egui::Align2::RIGHT_TOP,
                    format!("Place {} × {} stamp · R rotates", stamp.width, stamp.height),
                    egui::FontId::proportional(18.0),
                    Color32::LIGHT_BLUE,
                );
            }
            if self.tool == Tool::Select
                && self.panel == Panel::Scenery
                && !self.grid
                && !selected
                && (response.clicked() || dragging)
                && let Some(target) = surface_point
            {
                self.move_scenery(target);
                return;
            }
            if !selected
                && (response.clicked()
                    || dragging
                    || matches!(self.tool, Tool::Paint | Tool::Erase | Tool::Area)
                        && response.dragged_by(egui::PointerButton::Primary))
            {
                if matches!(self.tool, Tool::Paint | Tool::Erase)
                    && let Some(previous) = self.last_cell
                {
                    let steps = (point.x - previous.x)
                        .abs()
                        .max((point.z - previous.z).abs());
                    for step in 1..steps {
                        self.use_cell(Point {
                            x: previous.x + (point.x - previous.x) * step / steps,
                            z: previous.z + (point.z - previous.z) * step / steps,
                        });
                    }
                }
                if self.last_cell != Some(point) || response.clicked() {
                    self.use_cell(point);
                }
                self.last_cell = Some(point);
            }
        }
    }

    fn navigate(&mut self, ui: &egui::Ui, response: &egui::Response) {
        ui.input(|input| {
            let delta = input.pointer.delta();
            if response.dragged_by(egui::PointerButton::Secondary)
                || response.hovered() && input.pointer.secondary_down()
            {
                self.camera.look(f64::from(delta.x), f64::from(delta.y));
                if !ui.ctx().wants_keyboard_input() {
                    let axis = |positive, negative| {
                        f32::from(input.key_down(positive)) - f32::from(input.key_down(negative))
                    };
                    let forward = self.camera.forward();
                    let right = forward.cross(Vec3::NEG_Y).normalize();
                    let direction = forward * axis(egui::Key::W, egui::Key::S)
                        + right * axis(egui::Key::D, egui::Key::A)
                        + Vec3::NEG_Y * axis(egui::Key::E, egui::Key::Q);
                    self.camera.position += direction.clamp_length_max(1.0)
                        * input.stable_dt.min(0.1)
                        * if input.modifiers.shift {
                            48_000.0
                        } else {
                            12_000.0
                        };
                }
            }
            if response.dragged_by(egui::PointerButton::Middle) {
                let right = self.camera.forward().cross(Vec3::NEG_Y).normalize();
                let up = right.cross(self.camera.forward()).normalize();
                self.camera.position += (-right * delta.x + up * delta.y) * 25.0;
            }
            if response.hovered() {
                self.camera.position += self.camera.forward() * input.smooth_scroll_delta.y * 20.0;
            }
            self.camera.position = self
                .camera
                .position
                .clamp(Vec3::splat(-320_000.0), Vec3::splat(320_000.0));
        });
    }
}

fn marker(painter: &egui::Painter, point: Pos2, index: usize, selected: bool) {
    let color = if selected {
        Color32::GOLD
    } else if index == 0 {
        Color32::LIGHT_BLUE
    } else {
        Color32::WHITE
    };
    painter.circle_filled(point, 11.0, Color32::from_black_alpha(220));
    painter.circle_stroke(point, 11.0, Stroke::new(1.5_f32, color));
    painter.text(
        point,
        egui::Align2::CENTER_CENTER,
        if index == 0 {
            "P".into()
        } else {
            index.to_string()
        },
        egui::FontId::monospace(12.0),
        color,
    );
}
