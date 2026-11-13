use super::super::{
    campaign::Action,
    selection::{Edit, Selection},
};
use crate::{
    frontend::{
        draw::{Artwork, Frame},
        raster::{Blend, Canvas, Sprite, Vertex},
    },
    settings::Graphics,
    visuals::{catalog, pack::Pack},
};
use glam::{Vec2, Vec3};
use std::sync::Arc;
use tenchu_assets::{Result, model::Texture};
use tenchu_core::{
    math::{Matrix, Position},
    motion::Pose,
};

pub(crate) struct Drawing {
    pub frame: Arc<Frame>,
    pub text: Vec<crate::fonts::Text>,
    artwork: Arc<Artwork>,
    tick: u32,
    circle_confirm: bool,
}

impl Drawing {
    pub fn new(selection: &Selection, graphics: &Graphics) -> Result<Self> {
        let resources = &selection.resources;
        let mut artwork = super::artwork::load(
            resources.images.iter().chain([
                &resources.names,
                &resources.buttons,
                &resources.labels,
            ]),
            graphics,
        )?;
        if graphics.asset_pack {
            for (name, model) in ["rikimaru", "ayame", "tatumaru"]
                .into_iter()
                .zip(&selection.resources.models)
            {
                let sources: Vec<_> = model
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(i, part)| catalog::ModelSource {
                        role: catalog::Role::Actor,
                        hero_part: None,
                        id: format!("tenchu2/actors/{name}/{i}"),
                        label: name.into(),
                        geometry: &part.geometry,
                    })
                    .collect();
                artwork.extend(Pack::for_models(&model.vram, &sources)?.textures);
            }
        }
        let artwork = Arc::new(artwork);
        let (frame, text) = build(selection, artwork.clone(), true)?;
        Ok(Self {
            frame,
            text,
            artwork,
            tick: selection.state.tick,
            circle_confirm: true,
        })
    }

    pub fn update(&mut self, selection: &Selection, circle_confirm: bool) -> Result<()> {
        if self.tick != selection.state.tick || self.circle_confirm != circle_confirm {
            (self.frame, self.text) = build(selection, self.artwork.clone(), circle_confirm)?;
            self.tick = selection.state.tick;
            self.circle_confirm = circle_confirm;
        }
        Ok(())
    }
}

fn build(
    selection: &Selection,
    artwork: Arc<Artwork>,
    circle_confirm: bool,
) -> Result<(Arc<Frame>, Vec<crate::fonts::Text>)> {
    let state = &selection.state;
    let resources = &selection.resources;
    let mut canvas = Canvas::new(320);
    canvas.frame = Some(Frame::new([320, 240], artwork));
    canvas.sprite(Sprite::new(&resources.images[0], 0.0, 0.0));
    characters(selection, &mut canvas)?;
    canvas.sprite(Sprite::new(&resources.images[5], 0.0, 65.0));
    let mut sprites: Vec<_> = resources
        .sprites
        .iter()
        .enumerate()
        .filter(|(_, s)| s.flags & 1 != 0)
        .collect();
    sprites.sort_by_key(|&(i, s)| (std::cmp::Reverse(s.layer), std::cmp::Reverse(i)));
    for (i, descriptor) in sprites {
        let enabled = match i {
            0..4 => state.available(state.page + i, &resources.rules),
            4 => state.page > 0,
            5 => state.available(state.page + 4, &resources.rules),
            9..=16 => {
                let mission = state.page + (i - 9) % 4;
                state.available(mission, &resources.rules)
                    && state.progress.records[state.hero][mission].rank == 6
            }
            _ => true,
        };
        if !enabled {
            continue;
        }
        let [x, y] = descriptor.position.map(f32::from);
        let mut sprite = Sprite::new(&resources.images[descriptor.image], x, y);
        if descriptor.flags & 0x80 != 0 {
            sprite = sprite.row(
                if i == 8 {
                    state.hero
                } else {
                    descriptor.selected
                },
                descriptor.slices,
            );
        }
        if descriptor.image == 6 {
            sprite = sprite.angle((state.tick as f32 * 16.0).to_radians());
        }
        canvas.sprite(sprite);
    }
    for row in 0..4 {
        let mission = state.page + row;
        if !state.available(mission, &resources.rules) {
            continue;
        }
        let name = usize::from(resources.rules.names[state.hero][mission].unwrap());
        canvas.sprite(
            Sprite::new(&resources.names, -73.0, -65.0 + row as f32 * 25.0)
                .region([name / 20 * 104, name % 20 * 12, 104, 12])
                .light(if mission == state.mission { 128 } else { 32 })
                .blend(Blend::Add),
        );
    }
    for (row, text) in resources.rules.descriptions[state.hero * 11 + state.mission]
        .iter()
        .enumerate()
    {
        canvas.text.push(crate::fonts::Text {
            position: egui::pos2(-128.0, 40.0 + row as f32 * 16.0),
            text: text.clone(),
            color: egui::Color32::WHITE,
            size: 15.0,
            width: 256.0,
            centered: false,
        });
    }
    let width = resources.buttons.size[0] + resources.labels.size[0] + 8;
    let mut x = -(width as f32);
    for (button, label) in [(1, 0), (2, 2)] {
        let button = if circle_confirm { button } else { 3 - button };
        let w = resources.buttons.size[0] as f32;
        canvas.sprite(
            Sprite::new(
                &resources.buttons,
                x + w * 0.5,
                f32::from(resources.legend_y),
            )
            .row(button, 5),
        );
        x += w;
        let w = resources.labels.size[0] as f32;
        canvas.sprite(
            Sprite::new(
                &resources.labels,
                x + w * 0.5,
                f32::from(resources.legend_y),
            )
            .row(label, 12),
        );
        x += w + 8.0;
    }
    canvas.fade((i32::from(state.fade).abs() + 1) / 2);
    Ok((Arc::new(canvas.frame.unwrap()), canvas.text))
}

fn panel(selection: &Selection, canvas: &mut Canvas) {
    let state = &selection.state;
    let resources = &selection.resources;
    let [sin, cos] = selection.math.pair(state.phase as i16);
    let width = ((cos * 60) >> 12) as f32;
    let shear = ((sin * 5) >> 12) as f32;
    let points = [
        [82.0 - width, -83.0 - shear],
        [82.0 + width, -83.0 + shear],
        [82.0 - width, 31.0 + shear],
        [82.0 + width, 31.0 - shear],
    ];
    let image = &resources.images[1];
    let corners = std::array::from_fn::<_, 4, _>(|i| Vertex {
        position: Vec2::from_array(points[i]) + Vec2::new(160.0, 120.0),
        uv: Vec2::new(
            (i & 1) as f32 * image.size[0] as f32,
            (i >> 1) as f32 * image.size[1] as f32,
        ),
        color: Vec3::splat(128.0),
    });
    canvas.image_triangles(
        image,
        &[
            corners[0], corners[1], corners[2], corners[1], corners[3], corners[2],
        ],
        Blend::Opaque,
    );
}

type Triangle = (f32, usize, [Vertex; 3], Option<Texture>, Blend);

#[cfg(test)]
mod tests;

fn characters(selection: &Selection, canvas: &mut Canvas) -> Result<()> {
    let mut triangles: Vec<Triangle> = Vec::new();
    let state = &selection.state;
    for hero in 0..3 {
        if hero != state.displayed && hero != state.hero {
            continue;
        }
        let model = &selection.resources.models[hero];
        let bind = Pose::new(&model.parts)?.matrices(&selection.math, Matrix::default());
        let pose = selection.motions[hero].pose.matrices(
            &selection.math,
            Matrix {
                // MENU.EXE's +0x1c/+0x20 stores are root X/Y; the camera is at Z=-5000.
                translation: Position::new(1200, 500, 5000),
                rotation: selection.math.rotate_yxz([50, state.yaw[hero], 0]),
            },
        );
        for (owner, part) in model.parts.iter().enumerate() {
            for geometry in &part.geometry {
                for face in &geometry.faces {
                    let mut corners = Vec::with_capacity(4);
                    let mut depth = 0.0;
                    for corner in &face.corners {
                        let vertex = usize::from(corner.vertex);
                        let joint = geometry
                            .joints
                            .get(vertex)
                            .map_or(owner, |&j| usize::from(j));
                        let local: Position = geometry.vertices[vertex].map(i32::from).into();
                        let local = local
                            .wrapping_add(bind[owner].translation)
                            .wrapping_sub(bind[joint].translation);
                        let point = pose[joint]
                            .apply_large(local)
                            .wrapping_add(pose[joint].translation);
                        if point.z <= 0 {
                            break;
                        }
                        let z = point.z as f32;
                        depth += z;
                        corners.push(Vertex {
                            position: Vec2::new(
                                160.0 + point.x as f32 * 300.0 / z,
                                120.0 + point.y as f32 * 300.0 / z,
                            ),
                            uv: Vec2::from_array(corner.uv.map(f32::from)),
                            color: Vec3::from_array(corner.color.map(f32::from)),
                        });
                    }
                    if corners.len() != face.corners.len() {
                        continue;
                    }
                    let area = (corners[1].position - corners[0].position)
                        .perp_dot(corners[2].position - corners[0].position);
                    if area <= 0.0 && !face.double_sided {
                        continue;
                    }
                    depth /= corners.len() as f32;
                    let blend = if !face.translucent {
                        Blend::Opaque
                    } else {
                        match face.texture.map_or(0, |t| t.page >> 5 & 3) {
                            0 => Blend::Average,
                            1 => Blend::Add,
                            2 => Blend::Subtract,
                            _ => Blend::Quarter,
                        }
                    };
                    triangles.push((
                        depth,
                        hero,
                        [corners[0], corners[1], corners[2]],
                        face.texture,
                        blend,
                    ));
                    if corners.len() == 4 {
                        triangles.push((
                            depth,
                            hero,
                            [corners[1], corners[3], corners[2]],
                            face.texture,
                            blend,
                        ));
                    }
                }
            }
        }
    }
    triangles.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut panel_drawn = false;
    for (depth, hero, vertices, texture, blend) in triangles {
        if !panel_drawn && depth <= 0x13b as f32 * 16.0 {
            panel(selection, canvas);
            panel_drawn = true;
        }
        canvas.model_triangle(
            &selection.resources.models[hero].vram,
            texture,
            vertices,
            blend,
        );
    }
    if !panel_drawn {
        panel(selection, canvas);
    }
    Ok(())
}

pub(super) fn controls(
    selection: &Selection,
    ctx: &egui::Context,
    confirm: &str,
    back: &str,
) -> Option<Action> {
    if selection.state.fade != 0 || selection.state.finished {
        return None;
    }
    let mut action = None;
    let rect = super::film::rect(ctx);
    let scale = rect.height() / 240.0;
    let mut hit = |id, center: [f32; 2], size: [f32; 2], edit, tip: &str| {
        let size = Vec2::from_array(size) * scale;
        let position = rect.center() + egui::vec2(center[0], center[1]) * scale
            - egui::vec2(size.x, size.y) * 0.5;
        egui::Area::new(egui::Id::new(("jp-mission-control", id)))
            .movable(false)
            .fixed_pos(position)
            .show(ctx, |ui| {
                let response = ui
                    .allocate_response(egui::vec2(size.x, size.y), egui::Sense::click())
                    .on_hover_text(tip);
                if response.clicked() {
                    action = Some(Action::MissionSelection(edit));
                }
            });
    };
    for row in 0..4 {
        let mission = selection.state.page + row;
        if selection
            .state
            .available(mission, &selection.resources.rules)
        {
            hit(
                row,
                [-73.0, -65.0 + row as f32 * 25.0],
                [164.0, 24.0],
                Edit::Mission(mission),
                "Select mission",
            );
        }
    }
    hit(
        4,
        [30.0, -95.0],
        [24.0, 24.0],
        Edit::Hero(false),
        "Previous character",
    );
    hit(
        5,
        [136.0, -95.0],
        [24.0, 24.0],
        Edit::Hero(true),
        "Next character",
    );
    hit(
        6,
        [-73.0, -83.0],
        [30.0, 12.0],
        Edit::Scroll(false),
        "Previous mission",
    );
    hit(
        7,
        [-73.0, 30.0],
        [30.0, 16.0],
        Edit::Scroll(true),
        "Next mission",
    );
    egui::Area::new("jp-mission-actions".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -6.0])
        .show(ctx, |ui| {
            ui.spacing_mut().interact_size.y = 36.0;
            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(format!("Start · Enter / {confirm}")).clicked() {
                        action = Some(Action::Continue);
                    }
                    if ui.button(format!("Back · {back}")).clicked() {
                        action = Some(Action::Quit);
                    }
                });
            });
        });
    action
}
