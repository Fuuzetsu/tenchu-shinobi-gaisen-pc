pub(crate) mod bandage;
mod combat;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use combat::{blood, gun_impact, impact_flash, soft_patch};
mod style;
mod trial;
mod world;
pub use style::Style;
pub(crate) use style::metal_spark;

use crate::render_mesh::{self, SCREEN, TEXTURED, TRANSLUCENT, Vertex};
use std::ops::Range;
use tenchu_assets::texture::Image;
use tenchu_core::{game::Game, math::Matrix};

#[derive(Clone)]
pub struct Batch {
    pub blend: u8,
    pub range: Range<u32>,
}

pub struct Geometry {
    pub vertices: Vec<Vertex>,
    pub batches: Vec<Batch>,
}

impl Geometry {
    pub fn hide_ui(&mut self) {
        self.remove_flag(render_mesh::UI);
    }

    pub fn remove_flag(&mut self, flag: u32) {
        if !self.batches.iter().any(|batch| {
            self.vertices[batch.range.start as usize..batch.range.end as usize]
                .chunks_exact(3)
                .any(|triangle| triangle[0].flags & flag != 0)
        }) {
            return;
        }
        if self
            .batches
            .windows(2)
            .all(|pair| pair[0].range.end <= pair[1].range.start)
        {
            let mut write = 0;
            self.batches.retain_mut(|batch| {
                let start = write;
                let mut run = batch.range.start as usize;
                let end = run + batch.range.len() / 3 * 3;
                for read in (run..end).step_by(3) {
                    if self.vertices[read].flags & flag != 0 {
                        self.vertices.copy_within(run..read, write);
                        write += read - run;
                        run = read + 3;
                    }
                }
                self.vertices.copy_within(run..end, write);
                write += end - run;
                batch.range = start as u32..write as u32;
                start < write
            });
            self.vertices.truncate(write);
            return;
        }
        // Repeated or reordered batches can still need vertices that an earlier
        // batch would overwrite. Keep their source intact until filtering ends.
        let vertices = std::mem::take(&mut self.vertices);
        let batches = std::mem::take(&mut self.batches);
        self.vertices.reserve(vertices.len());
        self.batches.reserve(batches.len());
        for batch in batches {
            let start = self.vertices.len() as u32;
            self.vertices.extend(
                vertices[batch.range.start as usize..batch.range.end as usize]
                    .chunks_exact(3)
                    .filter(|triangle| triangle[0].flags & flag == 0)
                    .flatten()
                    .copied(),
            );
            let end = self.vertices.len() as u32;
            if start < end {
                self.batches.push(Batch {
                    range: start..end,
                    blend: batch.blend,
                });
            }
        }
    }

    pub fn append(&mut self, mut other: Geometry) {
        if !self.vertices.is_empty()
            && self.vertices.len() < other.vertices.len()
            && self
                .vertices
                .iter()
                .all(|v| v.flags & render_mesh::ALPHA != 0)
        {
            // Alpha draw order comes from batches; the opaque pass ignores these vertices.
            let offset = other.vertices.len() as u32;
            other.vertices.append(&mut self.vertices);
            self.vertices = other.vertices;
            for batch in &mut self.batches {
                batch.range.start += offset;
                batch.range.end += offset;
            }
            self.batches.extend(other.batches);
            return;
        }
        let offset = self.vertices.len() as u32;
        if self.vertices.is_empty() {
            self.vertices = other.vertices;
        } else {
            self.vertices.extend(other.vertices);
        }
        self.batches
            .extend(other.batches.into_iter().map(|batch| Batch {
                blend: batch.blend,
                range: batch.range.start + offset..batch.range.end + offset,
            }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ambient(game: &Game, view: Matrix, style: Style) -> Vec<Surface> {
    let mut surfaces = Vec::new();
    world::sprites(&mut surfaces, game, view, style, None);
    surfaces
}

pub fn panel(font: &tenchu_assets::font::Font, lines: &[(i16, i16, String, [f32; 3])]) -> Geometry {
    let mut vertices = screen_band(-120.0, 120.0, [0.018, 0.025, 0.034], 0);
    for vertex in &mut vertices {
        vertex.position[2] = 0.00001;
        vertex.flags |= render_mesh::UI;
    }
    let count = vertices.len() as u32;
    let mut geometry = Geometry {
        vertices,
        batches: vec![Batch {
            blend: 0,
            range: 0..count,
        }],
    };
    geometry.append(text(font, lines));
    geometry
}

pub fn text(font: &tenchu_assets::font::Font, lines: &[(i16, i16, String, [f32; 3])]) -> Geometry {
    let mut vertices = Vec::new();
    for &(left, top, ref text, color) in lines {
        let mut x = f32::from(left);
        for code in text.bytes() {
            let (glyph, nudge) = font.glyph(code);
            let y = f32::from(top + nudge);
            let right = x + f32::from(glyph.size[0]);
            let bottom = y + f32::from(glyph.size[1]);
            vertices.extend(quad(
                [
                    [x, y, 0.0],
                    [right, y, 0.0],
                    [x, bottom, 0.0],
                    [right, bottom, 0.0],
                ],
                [color; 4],
                Some(glyph),
                0,
            ));
            x += font.width(code) as f32;
        }
    }
    for vertex in &mut vertices {
        vertex.flags |= render_mesh::UI;
    }
    let count = vertices.len() as u32;
    Geometry {
        vertices,
        batches: vec![Batch {
            blend: 0,
            range: 0..count,
        }],
    }
}

pub(crate) struct Surface<V = Vec<Vertex>> {
    pub depth: f32,
    pub blend: u8,
    pub vertices: V,
}

pub(crate) struct Stage {
    vertices: Vec<Vertex>,
}

impl Stage {
    pub fn new(game: &Game, models: &crate::visuals::models::Models) -> Option<Self> {
        #[cfg(test)]
        if std::env::var_os("TENCHU_REBUILD_TRANSLUCENT_STAGE").is_some() {
            return None;
        }
        let mut vertices = Vec::new();
        let mut prepared = true;
        render_mesh::visit_stage(&game.mission, &game.math, |meshes, matrix| {
            if !prepared {
                return;
            }
            prepared = models.visit_filtered(meshes, matrix, true, TRANSLUCENT, |triangle| {
                vertices.extend(triangle.into_iter().map(|vertex| Vertex {
                    flags: vertex.flags | render_mesh::SCENERY,
                    ..*vertex
                }));
            });
        });
        // Unprepared faces keep the original composed integer transform for depth.
        prepared.then_some(Self { vertices })
    }

    fn geometry(&self, dynamic: Vec<Surface>, view: Matrix) -> Geometry {
        let surfaces = self
            .vertices
            .chunks_exact(3)
            .map(|vertices| Surface {
                depth: triangle_depth(std::array::from_fn(|i| &vertices[i]), view),
                blend: ((vertices[0].page >> 5) & 3) as u8 + 1,
                vertices,
            })
            .chain(dynamic.iter().map(|surface| Surface {
                depth: surface.depth,
                blend: surface.blend,
                vertices: surface.vertices.as_slice(),
            }))
            .collect();
        surface_geometry_into(surfaces, Vec::new())
    }
}

fn triangle_depth(triangle: [&Vertex; 3], view: Matrix) -> f32 {
    triangle
        .iter()
        .map(|vertex| {
            (0..3)
                .map(|i| vertex.position[i] * f32::from(view.rotation[2][i]) / 4096.0)
                .sum::<f32>()
                + view.translation[2] as f32
        })
        .sum::<f32>()
        / 3.0
}

pub(crate) fn depth(z: f32) -> f32 {
    let near = 50.0;
    let far = 250_000.0;
    (far / (far - near) - near * far / ((far - near) * z.max(near))).clamp(0.0, 1.0)
}

pub(crate) fn quad(
    points: [[f32; 3]; 4],
    color: [[f32; 3]; 4],
    image: Option<Image>,
    blend: u8,
) -> Vec<Vertex> {
    [0, 1, 2, 1, 3, 2]
        .into_iter()
        .map(|i| {
            let uv = image.map_or([0.0; 2], |image| {
                // Endpoints must stay inside the sprite even after the texel precision bias.
                [
                    f32::from(image.uv[0])
                        + if i & 1 != 0 {
                            f32::from(image.size[0]) - 0.01
                        } else {
                            0.0
                        },
                    f32::from(image.uv[1])
                        + if i & 2 != 0 {
                            f32::from(image.size[1]) - 0.01
                        } else {
                            0.0
                        },
                ]
            });
            Vertex {
                material: 0,
                normal: [0.0; 3],
                opacity: 1.0,
                position: points[i],
                color: color[i],
                uv,
                page: image.map_or(0, |i| u32::from(i.page)),
                palette: image.map_or(0, |i| u32::from(i.palette)),
                flags: SCREEN
                    | if image.is_some() { TEXTURED } else { 0 }
                    | if blend != 0 { TRANSLUCENT } else { 0 },
            }
        })
        .collect()
}

fn line(
    surfaces: &mut Vec<Surface>,
    points: [[i16; 2]; 2],
    sort_depth: i16,
    color: [u8; 3],
    blend: u8,
) {
    let [a, b] = points.map(|p| p.map(f32::from));
    let delta = [b[0] - a[0], b[1] - a[1]];
    let length = delta[0].hypot(delta[1]);
    if length == 0.0 {
        return;
    }
    let offset = [-delta[1] / length * 0.5, delta[0] / length * 0.5];
    let z = f32::from(sort_depth) * 16.0;
    let points = [
        [a[0] - offset[0], a[1] - offset[1], depth(z)],
        [b[0] - offset[0], b[1] - offset[1], depth(z)],
        [a[0] + offset[0], a[1] + offset[1], depth(z)],
        [b[0] + offset[0], b[1] + offset[1], depth(z)],
    ];
    surfaces.push(Surface {
        depth: z,
        blend,
        vertices: quad(
            points,
            [color.map(|v| f32::from(v) / 255.0); 4],
            None,
            blend,
        ),
    });
}

fn screen_band(top: f32, bottom: f32, color: [f32; 3], blend: u8) -> Vec<Vertex> {
    let mut vertices = quad(
        [
            [-160.0, top, 0.0],
            [160.0, top, 0.0],
            [-160.0, bottom, 0.0],
            [160.0, bottom, 0.0],
        ],
        [color; 4],
        None,
        blend,
    );
    for vertex in &mut vertices {
        vertex.flags |= render_mesh::SCREEN_WIDE;
    }
    vertices
}

fn mesh_surfaces(
    surfaces: &mut Vec<Surface>,
    meshes: &[tenchu_assets::model::Geometry],
    matrix: Matrix,
    view: Matrix,
    translucent_only: bool,
    fog: bool,
    models: Option<&crate::visuals::models::Models>,
) {
    if models.is_some_and(|models| {
        models.visit_filtered(
            meshes,
            matrix,
            fog,
            if translucent_only { TRANSLUCENT } else { 0 },
            |triangle| {
                let face = triangle[0];
                let blend = if face.flags & TRANSLUCENT == 0 {
                    0
                } else {
                    ((face.page >> 5) & 3) as u8 + 1
                };
                let depth = triangle_depth(triangle, view);
                surfaces.push(Surface {
                    depth,
                    blend,
                    vertices: triangle.into_iter().copied().collect(),
                });
            },
        )
    }) {
        return;
    }
    let view = view.compose(&matrix);
    for mesh in meshes {
        for face in render_mesh::draw_tmd_faces(mesh) {
            if translucent_only && !face.translucent {
                continue;
            }
            let blend = if face.translucent {
                face.texture.map_or(1, |t| ((t.page >> 5) & 3) as u8 + 1)
            } else {
                0
            };
            let mut vertices = Vec::new();
            render_mesh::append_face(&mut vertices, mesh, face, matrix, fog);
            let depth = face
                .corners
                .iter()
                .map(|c| view.transform(mesh.vertices[c.vertex as usize])[2] as f32)
                .sum::<f32>()
                / face.corners.len() as f32;
            surfaces.push(Surface {
                depth,
                blend,
                vertices,
            });
        }
    }
}

pub fn build(game: &Game) -> Geometry {
    build_with_hud(game, true)
}

pub fn build_with_hud(game: &Game, show_hud: bool) -> Geometry {
    build_for_camera(game, show_hud, None)
}

pub fn build_for_camera(game: &Game, show_hud: bool, camera: Option<Matrix>) -> Geometry {
    build_options(game, show_hud, camera, true, None)
}

pub fn build_options(
    game: &Game,
    show_hud: bool,
    camera: Option<Matrix>,
    blob_shadows: bool,
    models: Option<&crate::visuals::models::Models>,
) -> Geometry {
    build_presented(
        game,
        show_hud,
        camera,
        blob_shadows,
        models,
        None,
        Style::default(),
    )
}

pub fn build_presented(
    game: &Game,
    show_hud: bool,
    camera: Option<Matrix>,
    blob_shadows: bool,
    models: Option<&crate::visuals::models::Models>,
    frame: Option<&crate::visuals::interpolation::Frame>,
    style: Style,
) -> Geometry {
    build_presented_with_stage(
        game,
        show_hud,
        camera,
        blob_shadows,
        models,
        frame,
        style,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_presented_with_stage(
    game: &Game,
    show_hud: bool,
    camera: Option<Matrix>,
    blob_shadows: bool,
    models: Option<&crate::visuals::models::Models>,
    frame: Option<&crate::visuals::interpolation::Frame>,
    style: Style,
    stage: Option<&Stage>,
) -> Geometry {
    let sharp_text = style.sharp_text;
    let inspection = camera.is_some();
    let view = camera.unwrap_or_else(|| frame.map_or(game.rendered_view, |f| f.view));
    let mut surfaces = Vec::new();
    if stage.is_none() {
        render_mesh::visit_stage(&game.mission, &game.math, |meshes, matrix| {
            mesh_surfaces(&mut surfaces, meshes, matrix, view, true, true, models);
        });
        for vertex in surfaces
            .iter_mut()
            .flat_map(|surface| &mut surface.vertices)
        {
            vertex.flags |= render_mesh::SCENERY;
        }
    }
    crate::visuals::appearance::visit_parts(
        game,
        inspection || game.roster.keep_all_active,
        models,
        frame,
        |key, meshes, matrix| {
            let start = surfaces.len();
            mesh_surfaces(&mut surfaces, meshes, matrix, view, true, true, models);
            if let Some(deformation) = models.and_then(|models| models.deformation(key, matrix)) {
                for surface in &mut surfaces[start..] {
                    for vertex in &mut surface.vertices {
                        deformation.vertex(vertex);
                    }
                    surface.depth = surface
                        .vertices
                        .iter()
                        .map(|v| {
                            (0..3)
                                .map(|i| v.position[i] * f32::from(view.rotation[2][i]) / 4096.0)
                                .sum::<f32>()
                                + view.translation[2] as f32
                        })
                        .sum::<f32>()
                        / surface.vertices.len() as f32;
                }
            }
        },
    );
    for (instances, index) in [(&game.effects.shadows, 25), (&game.effects.blood_pools, 31)] {
        if (index == 25 && !blob_shadows)
            || (index == 31
                && (!style.blood.death.uses_original() || style.blood.custom_death(false)))
        {
            continue;
        }
        for instance in instances {
            let scale = if index == 31 {
                (i64::from(instance.scale) * i64::from(style.blood.death_size) / 100) as i32
            } else {
                instance.scale
            };
            let mut matrix = Matrix {
                rotation: game.math.rotate_yxz(instance.rotation),
                translation: instance.position,
            };
            for row in &mut matrix.rotation {
                for v in row {
                    *v = (i32::from(*v).wrapping_mul(scale) >> 12) as i16;
                }
            }
            let start = surfaces.len();
            mesh_surfaces(
                &mut surfaces,
                &game.mission.common_models[&index],
                matrix,
                view,
                false,
                false,
                if index == 31 { None } else { models },
            );
            if index == 31 {
                for vertex in surfaces[start..].iter_mut().flat_map(|s| &mut s.vertices) {
                    vertex.flags |= render_mesh::STAIN;
                    if style.blood.death == crate::settings::BloodStyle::OriginalHd {
                        vertex.flags |= render_mesh::BLOOD_HD;
                    }
                }
            }
        }
    }
    for wire in &game.effects.wires {
        let hook = if inspection {
            wire.world_hook.map(|m| (m, false))
        } else {
            wire.hook
        };
        if let Some((matrix, fog)) = hook {
            mesh_surfaces(
                &mut surfaces,
                &game.mission.common_models[&26],
                matrix,
                view,
                false,
                fog,
                models,
            );
        }
        for &(points, sort_depth) in wire
            .lines
            .iter()
            .filter(|_| !inspection && !models.is_some_and(|m| m.grapple))
        {
            line(&mut surfaces, points, sort_depth, [80, 72, 56], 0);
        }
    }
    if inspection {
        world::sprites(&mut surfaces, game, view, style, frame);
        for bolt in game.effects.bolts.iter().filter(|_| !style.lightning) {
            for points in bolt.points.windows(2) {
                world::line(
                    &mut surfaces,
                    [points[0], points[1]]
                        .map(|p| glam::Vec3::from(p.map(|v| v as f32).to_array())),
                    bolt.color,
                    2,
                    view,
                );
            }
        }
        if !models.is_some_and(|models| models.grapple) {
            for wire in &game.effects.wires {
                for points in wire.points.windows(2) {
                    world::line(
                        &mut surfaces,
                        [points[0], points[1]]
                            .map(|p| glam::Vec3::from(p.map(|v| v as f32).to_array())),
                        [80, 72, 56],
                        0,
                        view,
                    );
                }
            }
        }
    }
    if !inspection {
        world::items(&mut surfaces, game, view);
    }
    for segment in game
        .effects
        .lines
        .iter()
        .filter(|_| !inspection && !style.lightning)
    {
        line(
            &mut surfaces,
            segment.points,
            segment.depth,
            segment.color,
            2,
        );
    }
    for &(index, matrix) in game.items.models.iter().chain(
        game.misc
            .models
            .iter()
            .filter(|_| style.hinged && !style.distant_doors),
    ) {
        let matrix = frame.map_or(matrix, |frame| frame.hinged(index, matrix));
        mesh_surfaces(
            &mut surfaces,
            &game.mission.common_models[&index],
            matrix,
            view,
            false,
            true,
            models,
        );
    }
    if style.hinged && style.distant_doors {
        render_mesh::visit_hinged_parts(game, |key, geometry, matrix| {
            if !matches!(key, crate::visuals::interpolation::Part::Hinge { .. }) {
                return;
            }
            mesh_surfaces(
                &mut surfaces,
                geometry,
                crate::visuals::interpolation::Frame::transform(frame, key, matrix),
                view,
                false,
                true,
                models,
            );
        });
    }
    for &(kind, matrix) in &game.items.weapons {
        mesh_surfaces(
            &mut surfaces,
            &game.mission.weapons.models[&kind],
            matrix,
            view,
            false,
            true,
            models,
        );
    }
    for drawing in crate::visuals::death::blood::drawings(&game.effects, frame)
        .chain(&game.items.drawings)
        .chain(&game.misc.drawings)
        .filter(|drawing| {
            !inspection
                && !world::item_icon(&drawing.graphic)
                && !style.replaces(&drawing.graphic, drawing.color)
        })
    {
        let (image, blend, bottom_origin) = world::material(&drawing.graphic);
        let image = image.map(|i| game.mission.images[i]);
        let z = f32::from(drawing.depth) * 16.0;
        let color = drawing
            .color
            .map(|v| f32::from(v) / if image.is_some() { 128.0 } else { 255.0 });
        let xy = drawing.xy.map(f32::from);
        let angle = (drawing.rotation as f32 / 4096.0).to_radians();
        let (sin, cos) = angle.sin_cos();
        let points = std::array::from_fn(|i| {
            let mut p = if let Some(image) = image {
                let size = image.size.map(f32::from);
                let origin = [
                    f32::from(image.size[0] >> 1),
                    if bottom_origin {
                        size[1]
                    } else {
                        f32::from(image.size[1] >> 1)
                    },
                ];
                [
                    ((i & 1) as f32 * size[0] - origin[0]) * f32::from(drawing.scale[0] as i16)
                        / 4096.0,
                    (((i >> 1) & 1) as f32 * size[1] - origin[1])
                        * f32::from(drawing.scale[1] as i16)
                        / 4096.0,
                ]
            } else {
                [
                    (i & 1) as f32 * drawing.scale[0] as f32,
                    ((i >> 1) & 1) as f32 * drawing.scale[1] as f32,
                ]
            };
            p = [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos];
            [xy[0] + p[0], xy[1] + p[1], depth(z)]
        });
        let mut vertices = quad(points, [color; 4], image, blend);
        for vertex in &mut vertices {
            vertex.flags |= world::flags(&drawing.graphic, drawing.color);
        }
        let blend = style.present(&drawing.graphic, drawing.color, &mut vertices, blend);
        surfaces.push(Surface {
            depth: z,
            blend,
            vertices,
        });
    }
    let actor_trails = game
        .visible
        .iter()
        .flat_map(|&id| game.roster.get(id).trails.iter().flatten());
    for trail in actor_trails.chain(&game.items.trails) {
        if inspection {
            world::trail(&mut surfaces, trail, game.mission.images[10], view);
            continue;
        }
        if trail.depth == 0 {
            continue;
        }
        let z = f32::from(trail.depth) * 4.0;
        let mut previous = 127;
        for i in 1..trail.count {
            let shade = ((trail.count - i) * 127 / trail.count) as u8;
            let points = [
                trail.points[i][0],
                trail.points[i - 1][0],
                trail.points[i][1],
                trail.points[i - 1][1],
            ]
            .map(|p| [f32::from(p[0]), f32::from(p[1]), depth(z)]);
            let colors = [shade, previous, shade, previous].map(|v| [f32::from(v) / 128.0; 3]);
            surfaces.push(Surface {
                depth: z,
                blend: 2,
                vertices: quad(points, colors, Some(game.mission.images[10]), 2),
            });
            previous = shade;
        }
    }
    combat::draw(&mut surfaces, game, view, style, frame);
    let ui_start = surfaces.len();
    if show_hud {
        bandage::draw(&mut surfaces, game);
    }
    if let Some(shade) = game.hud.map.dim.filter(|_| !inspection) {
        let mut vertices = screen_band(-120.0, 120.0, [f32::from(shade) / 255.0; 3], 3);
        for vertex in &mut vertices {
            vertex.position[2] = 0.000002;
        }
        surfaces.push(Surface {
            depth: 2.0,
            blend: 3,
            vertices,
        });
        for line in &game.hud.map.lines {
            let [a, b] = line.points.map(|p| p.map(f32::from));
            let delta = [b[0] - a[0], b[1] - a[1]];
            let length = delta[0].hypot(delta[1]).max(1.0);
            let offset = [-delta[1] / length * 0.5, delta[0] / length * 0.5];
            surfaces.push(Surface {
                depth: 0.0,
                blend: 0,
                vertices: quad(
                    [
                        [a[0] + offset[0], a[1] + offset[1], 0.0],
                        [b[0] + offset[0], b[1] + offset[1], 0.0],
                        [a[0] - offset[0], a[1] - offset[1], 0.0],
                        [b[0] - offset[0], b[1] - offset[1], 0.0],
                    ],
                    [line.color.map(|v| f32::from(v) / 255.0); 4],
                    None,
                    0,
                ),
            });
        }
    }
    for sprite in game
        .items
        .sprites
        .iter()
        // Retail prepends HUD packets, so equal-depth sprites draw in reverse submission order.
        .chain(game.hud.sprites.iter().rev().filter(|sprite| {
            show_hud
                && (style.show_trial_timer
                    || crate::fonts::trial::timer(game, sprite.image).is_none())
                && !(sharp_text
                    && (crate::fonts::digit(game, sprite).is_some()
                        || crate::fonts::trial::timer(game, sprite.image).is_some()))
        }))
        .chain(
            game.cinematic.sprites.iter().skip(
                if sharp_text
                    || style
                        .subtitle_language
                        .is_some_and(|language| language != game.mission.scripts.language)
                {
                    crate::fonts::caption_sprites(game)
                } else {
                    0
                },
            ),
        )
    {
        if crate::fonts::trial::extended_counter(game).is_some()
            && crate::fonts::trial::candle(game, sprite).is_some()
        {
            continue;
        }
        if style.hd_hud
            && sprite.rotation == 0
            && let Some(frame) = crate::fonts::trial::candle(game, sprite)
        {
            surfaces.push(trial::candle(sprite, frame));
            continue;
        }
        let (sin, cos) = (sprite.rotation as f32 / 4096.0).to_radians().sin_cos();
        let points = std::array::from_fn(|i| {
            let x = ((i & 1) as f32 * f32::from(sprite.image.size[0])
                - f32::from(sprite.origin[0]))
                * f32::from(sprite.scale[0])
                / 4096.0;
            let y = ((i >> 1) as f32 * f32::from(sprite.image.size[1])
                - f32::from(sprite.origin[1]))
                * f32::from(sprite.scale[1])
                / 4096.0;
            [
                f32::from(sprite.xy[0]) + x * cos - y * sin,
                f32::from(sprite.xy[1]) + x * sin + y * cos,
                f32::from(sprite.depth) * 0.000001,
            ]
        });
        surfaces.push(Surface {
            depth: f32::from(sprite.depth),
            blend: sprite.blend,
            vertices: quad(
                points,
                [sprite.color.map(|v| f32::from(v) / 128.0); 4],
                Some(sprite.image),
                sprite.blend,
            ),
        });
    }
    for surface in &mut surfaces[ui_start..] {
        for vertex in &mut surface.vertices {
            vertex.flags |= render_mesh::UI;
        }
    }
    for &(priority, color) in &game.effects.fades {
        surfaces.push(Surface {
            depth: priority as f32 * 4.0,
            blend: 2,
            vertices: screen_band(-120.0, 120.0, color.map(|v| f32::from(v) / 255.0), 2),
        });
    }
    if game.cinematic.frame && !inspection {
        for (top, bottom) in [(-120.0, -90.0), (90.0, 120.0)] {
            let mut vertices = screen_band(top, bottom, [1.0 / 255.0; 3], 0);
            for vertex in &mut vertices {
                vertex.flags |= render_mesh::UI;
            }
            surfaces.push(Surface {
                depth: 1.0,
                blend: 0,
                vertices,
            });
        }
    }
    if let Some(stage) = stage {
        stage.geometry(surfaces, view)
    } else {
        surface_geometry(surfaces)
    }
}

pub(crate) fn retained_blood<'a>(
    game: &Game,
    view: Matrix,
    blood: crate::settings::Blood,
    sprites: impl Iterator<Item = &'a tenchu_core::effects::Billboard>,
) -> Geometry {
    let mut surfaces = Vec::new();
    world::billboards(
        &mut surfaces,
        game,
        view,
        Style {
            blood,
            ..Default::default()
        },
        sprites,
    );
    surface_geometry(surfaces)
}

pub(crate) fn surface_geometry(surfaces: Vec<Surface>) -> Geometry {
    surface_geometry_into(surfaces, Vec::new())
}

pub(crate) fn surface_geometry_into<V: AsRef<[Vertex]>>(
    mut surfaces: Vec<Surface<V>>,
    mut vertices: Vec<Vertex>,
) -> Geometry {
    // Coplanar stains compose in source order, independent of camera movement.
    // Draw them before airborne effects, which still need back-to-front sorting.
    surfaces.sort_by_cached_key(|surface| {
        if surface.vertices.as_ref().first().is_some_and(|v| {
            v.flags & (render_mesh::STAIN | render_mesh::SCREEN) == render_mesh::STAIN
        }) {
            return 0;
        }
        // Integer equivalent of descending f32::total_cmp, including signed zero and NaNs.
        let bits = surface.depth.to_bits();
        let ascending = bits ^ if bits >> 31 != 0 { u32::MAX } else { 1 << 31 };
        u64::from(!ascending) + 1
    });
    vertices.clear();
    vertices.reserve(
        surfaces
            .iter()
            .map(|surface| surface.vertices.as_ref().len())
            .sum(),
    );
    let mut out = Geometry {
        vertices,
        batches: Vec::new(),
    };
    for surface in surfaces {
        if surface.vertices.as_ref().is_empty() {
            continue;
        }
        let start = out.vertices.len() as u32;
        out.vertices.extend_from_slice(surface.vertices.as_ref());
        let end = out.vertices.len() as u32;
        if let Some(last) = out.batches.last_mut()
            && last.blend == surface.blend
        {
            last.range.end = end;
        } else {
            out.batches.push(Batch {
                blend: surface.blend,
                range: start..end,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    #[ignore = "requires TENCHU_DISC; retained translucent terrain must match rebuilt effects across cameras, presets and interpolation"]
    fn cached_translucent_stage_matches_rebuilt_effects() {
        use crate::visuals::{models::Models, pack::Pack};
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut cached_triangles = 0;
        for mission in [1, 4, 6] {
            let mut game = crate::session::Options {
                mission,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..12 {
                game.step(0).unwrap();
            }
            let mut history = crate::visuals::interpolation::History::default();
            history.reset(&game);
            game.step(0).unwrap();
            history.tick(&game);
            let interpolated = history.sample(game.tick, 0.4);
            let before = crate::session::tests::state(&game);
            let pack = Pack::from_env(&game.mission).unwrap();
            for preset in [
                0,
                crate::settings::presets::MODERN_HIGH,
                crate::settings::presets::MODERN_ULTRA,
            ] {
                let mut settings = crate::settings::Settings::default();
                settings.graphics.set_preset(preset);
                let models = Models::build(&game.mission, &pack, &settings.graphics).unwrap();
                let stage = Stage::new(&game, &models);
                if preset == 0 {
                    assert!(
                        stage.is_none(),
                        "original faces retain integer depth transforms"
                    );
                } else {
                    cached_triangles += stage.as_ref().unwrap().vertices.len() / 3;
                }
                for camera in [
                    None,
                    Some(Matrix::default()),
                    Some(Matrix {
                        rotation: game.math.rotate_yxz([128, 768, -256]),
                        translation: [15000, -3000, -8000].into(),
                    }),
                ] {
                    for frame in [None, interpolated.as_ref()] {
                        let rebuilt = build_presented(
                            &game,
                            true,
                            camera,
                            settings.graphics.blob_shadows,
                            Some(&models),
                            frame,
                            Style::from(&settings),
                        );
                        let cached = build_presented_with_stage(
                            &game,
                            true,
                            camera,
                            settings.graphics.blob_shadows,
                            Some(&models),
                            frame,
                            Style::from(&settings),
                            stage.as_ref(),
                        );
                        assert!(
                            bytemuck::cast_slice::<_, u8>(&cached.vertices)
                                == bytemuck::cast_slice::<_, u8>(&rebuilt.vertices),
                            "mission {mission}, preset {preset}"
                        );
                        assert_eq!(cached.batches.len(), rebuilt.batches.len());
                        for (a, b) in cached.batches.iter().zip(&rebuilt.batches) {
                            assert_eq!((a.blend, &a.range), (b.blend, &b.range));
                        }
                    }
                }
            }
            assert_eq!(before, crate::session::tests::state(&game));
        }
        assert!(cached_triangles > 1000);
        eprintln!("Retained translucent terrain: {cached_triangles} cached triangles");
    }

    #[test]
    fn surface_sort_retains_depth_bits_and_coplanar_submission_order() {
        let depths = [
            0,
            0x8000_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0000,
            0xffc0_0000,
            0x7f80_0001,
            0xff80_0001,
            1,
            0x8000_0001,
            0x3f80_0000,
            0xbf80_0000,
        ];
        let mut expected = Vec::new();
        let surfaces: Vec<_> = (0..4096)
            .map(|i| {
                let bits = if i % 2 == 0 {
                    depths[i / 2 % depths.len()]
                } else {
                    (i as u32).wrapping_mul(0x9e37_79b9)
                };
                let flags = match i % 5 {
                    0 => render_mesh::STAIN,
                    1 => render_mesh::STAIN | render_mesh::SCREEN,
                    2 => render_mesh::SCREEN,
                    _ => 0,
                };
                let depth = f32::from_bits(bits);
                let blend = (i % 4) as u8;
                expected.push((flags == render_mesh::STAIN, depth, blend, i as f32));
                Surface {
                    depth,
                    blend,
                    vertices: vec![
                        Vertex {
                            position: [i as f32, 0.0, 0.0],
                            flags,
                            ..Vertex::zeroed()
                        };
                        3
                    ],
                }
            })
            .collect();
        expected.sort_by(|a, b| match (a.0, b.0) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => b.1.total_cmp(&a.1),
        });
        let borrowed = surfaces
            .iter()
            .map(|surface| Surface {
                depth: surface.depth,
                blend: surface.blend,
                vertices: surface.vertices.as_slice(),
            })
            .collect();
        for actual in [
            surface_geometry_into(borrowed, Vec::new()),
            surface_geometry(surfaces),
        ] {
            let actual: Vec<_> = actual
                .batches
                .iter()
                .flat_map(|batch| {
                    actual.vertices[batch.range.start as usize..batch.range.end as usize]
                        .chunks_exact(3)
                        .map(|triangle| (batch.blend, triangle[0].position[0]))
                })
                .collect();
            assert_eq!(
                actual,
                expected.iter().map(|s| (s.2, s.3)).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; distant doors retain live/interpolated poses without changing gameplay or duplicating geometry"]
    fn distant_doors_preserve_poses_and_retail_state() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut hidden = 0;
        for mission in [1, 4, 6] {
            let mut game = crate::session::Options {
                mission,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..12 {
                game.step(0).unwrap();
            }
            let mut history = crate::visuals::interpolation::History::default();
            history.reset(&game);
            game.step(0).unwrap();
            history.tick(&game);
            let interpolated = history.sample(game.tick, 0.4);
            let before = crate::session::tests::state(&game);
            let original = game.misc.models.clone();
            let mut all = Vec::new();
            render_mesh::visit_hinged_parts(&game, |key, _, matrix| {
                if let crate::visuals::interpolation::Part::Hinge { mesh, .. } = key {
                    all.push((mesh, matrix));
                }
            });
            hidden += all.len().saturating_sub(original.len());
            for frame in [None, interpolated.as_ref()] {
                let capture = |game: &Game, settings: &crate::settings::Settings| {
                    build_presented(game, false, None, false, None, frame, Style::from(settings))
                };
                let mut settings = crate::settings::Settings::default();
                let retail = capture(&game, &settings);
                settings.graphics.fog = false;
                let actual = capture(&game, &settings);
                settings.graphics.distant_doors = false;
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&capture(&game, &settings).vertices),
                    bytemuck::cast_slice::<_, u8>(&retail.vertices)
                );
                game.misc.models.clone_from(&all);
                let expected = capture(&game, &settings);
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&actual.vertices),
                    bytemuck::cast_slice::<_, u8>(&expected.vertices)
                );
                assert!(
                    actual
                        .batches
                        .iter()
                        .zip(&expected.batches)
                        .all(|(a, b)| { a.blend == b.blend && a.range == b.range })
                );
                assert_eq!(actual.batches.len(), expected.batches.len());
                game.misc.models.clear();
                settings.graphics.distant_doors = true;
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&capture(&game, &settings).vertices),
                    bytemuck::cast_slice::<_, u8>(&actual.vertices)
                );
                game.misc.models.clone_from(&original);
            }
            assert_eq!(before, crate::session::tests::state(&game));
        }
        assert!(
            hidden > 0,
            "fixture must include doors outside retail's draw list"
        );
    }

    #[test]
    fn filtering_preserves_triangle_and_blend_order_for_shared_batch_ranges() {
        let cases = [
            vec![0..9, 9..18],
            vec![0..8, 8..17],
            vec![0..12, 6..18],
            vec![9..18, 0..12],
            vec![0..0, 3..9, 12..18],
            vec![0..18, 0..18],
        ];
        for ranges in cases {
            for mask in 0..64 {
                let source: Vec<_> = (0..18)
                    .map(|i| Vertex {
                        position: [i as f32, 0.0, 0.0],
                        flags: if mask & (1 << (i / 3)) != 0 {
                            render_mesh::ORIGINAL_SPLASH
                        } else {
                            0
                        },
                        ..Vertex::zeroed()
                    })
                    .collect();
                let mut geometry = Geometry {
                    vertices: source.clone(),
                    batches: ranges
                        .iter()
                        .enumerate()
                        .map(|(i, range)| Batch {
                            blend: i as u8,
                            range: range.clone(),
                        })
                        .collect(),
                };
                let expected: Vec<_> = geometry
                    .batches
                    .iter()
                    .flat_map(|batch| {
                        source[batch.range.start as usize..batch.range.end as usize]
                            .chunks_exact(3)
                            .filter(|triangle| {
                                triangle[0].flags & render_mesh::ORIGINAL_SPLASH == 0
                            })
                            .flatten()
                            .map(|v| (batch.blend, v.position[0]))
                    })
                    .collect();
                geometry.remove_flag(render_mesh::ORIGINAL_WEATHER);
                geometry.remove_flag(render_mesh::ORIGINAL_SPLASH);
                let actual: Vec<_> = geometry
                    .batches
                    .iter()
                    .flat_map(|batch| {
                        geometry.vertices[batch.range.start as usize..batch.range.end as usize]
                            .chunks_exact(3)
                            .flatten()
                            .map(|v| (batch.blend, v.position[0]))
                    })
                    .collect();
                assert_eq!(actual, expected, "{ranges:?}, mask {mask}");
            }
        }
    }
}
