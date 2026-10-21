use crate::render_mesh::{self, SCREEN, TEXTURED, TRANSLUCENT, Vertex};
use std::ops::Range;
use tenchu_assets::texture::Image;
use tenchu_core::{effects::Graphic, game::Game, math::Matrix};

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
        let vertices = std::mem::take(&mut self.vertices);
        let batches = std::mem::take(&mut self.batches);
        for batch in batches {
            let start = self.vertices.len() as u32;
            self.vertices.extend(
                vertices[batch.range.start as usize..batch.range.end as usize]
                    .chunks_exact(3)
                    .filter(|triangle| triangle[0].flags & render_mesh::UI == 0)
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

    pub fn append(&mut self, other: Geometry) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.batches
            .extend(other.batches.into_iter().map(|batch| Batch {
                blend: batch.blend,
                range: batch.range.start + offset..batch.range.end + offset,
            }));
    }
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

struct Surface {
    depth: f32,
    blend: u8,
    vertices: Vec<Vertex>,
}

fn depth(z: f32) -> f32 {
    let near = 50.0;
    let far = 250_000.0;
    (far / (far - near) - near * far / ((far - near) * z.max(near))).clamp(0.0, 1.0)
}

fn quad(
    points: [[f32; 3]; 4],
    color: [[f32; 3]; 4],
    image: Option<Image>,
    blend: u8,
) -> Vec<Vertex> {
    [0, 1, 2, 1, 3, 2]
        .into_iter()
        .map(|i| {
            let uv = image.map_or([0.0; 2], |image| {
                [
                    f32::from(image.uv[0])
                        + if i & 1 != 0 {
                            f32::from(image.size[0])
                        } else {
                            0.0
                        },
                    f32::from(image.uv[1])
                        + if i & 2 != 0 {
                            f32::from(image.size[1])
                        } else {
                            0.0
                        },
                ]
            });
            Vertex {
                material: 0,
                normal: [0.0; 3],
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
        models.visit(meshes, matrix, fog, |triangle| {
            let face = triangle[0];
            if translucent_only && face.flags & TRANSLUCENT == 0 {
                return;
            }
            let blend = if face.flags & TRANSLUCENT == 0 {
                0
            } else {
                ((face.page >> 5) & 3) as u8 + 1
            };
            let depth = triangle
                .iter()
                .map(|vertex| {
                    (0..3)
                        .map(|i| vertex.position[i] * f32::from(view.rotation[2][i]) / 4096.0)
                        .sum::<f32>()
                        + view.translation[2] as f32
                })
                .sum::<f32>()
                / 3.0;
            surfaces.push(Surface {
                depth,
                blend,
                vertices: triangle.to_vec(),
            });
        })
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
    build_presented(game, show_hud, camera, blob_shadows, models, None)
}

pub fn build_presented(
    game: &Game,
    show_hud: bool,
    camera: Option<Matrix>,
    blob_shadows: bool,
    models: Option<&crate::visuals::models::Models>,
    frame: Option<&crate::visuals::interpolation::Frame>,
) -> Geometry {
    let inspection = camera.is_some();
    let view = camera.unwrap_or_else(|| frame.map_or(game.rendered_view, |f| f.view));
    let mut surfaces = Vec::new();
    let mut translucent = |meshes: &[tenchu_assets::model::Geometry], matrix| {
        mesh_surfaces(&mut surfaces, meshes, matrix, view, true, true, models);
    };
    render_mesh::visit_stage(&game.mission, &game.math, &mut translucent);
    crate::visuals::appearance::visit(game, inspection, models, frame, &mut translucent);
    for (instances, index) in [(&game.effects.shadows, 25), (&game.effects.blood_pools, 31)] {
        if index == 25 && !blob_shadows {
            continue;
        }
        for instance in instances {
            let mut matrix = Matrix {
                rotation: game.math.rotate_yxz(instance.rotation),
                translation: instance.position,
            };
            for row in &mut matrix.rotation {
                for v in row {
                    *v = (i32::from(*v).wrapping_mul(instance.scale) >> 12) as i16;
                }
            }
            mesh_surfaces(
                &mut surfaces,
                &game.mission.common_models[&index],
                matrix,
                view,
                false,
                false,
                models,
            );
        }
    }
    for wire in &game.effects.wires {
        if let Some((matrix, fog)) = wire.hook {
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
        for &(points, sort_depth) in wire.lines.iter().filter(|_| !inspection) {
            line(&mut surfaces, points, sort_depth, [80, 72, 56], 0);
        }
    }
    for segment in game.effects.lines.iter().filter(|_| !inspection) {
        line(
            &mut surfaces,
            segment.points,
            segment.depth,
            segment.color,
            2,
        );
    }
    for &(index, matrix) in game.items.models.iter().chain(&game.misc.models) {
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
    for drawing in game
        .effects
        .drawings
        .iter()
        .chain(&game.items.drawings)
        .chain(&game.misc.drawings)
        .filter(|_| !inspection)
    {
        let (image, blend, bottom_origin) = match drawing.graphic {
            Graphic::Sprite { image, blend } => (Some(usize::from(image)), blend, false),
            Graphic::Explosion(index) => (Some(7 + usize::from(index)), 2, false),
            Graphic::Smoke(index) => (Some([6, 58][usize::from(index)]), 2, false),
            Graphic::Splash => (Some(14), 2, true),
            Graphic::Bleed => (None, 0, false),
            Graphic::Flash(index) => (Some(40 + index as usize), 2, false),
            Graphic::Impact(index) => (Some([15, 11, 12, 59, 60][usize::from(index)]), 2, false),
            Graphic::Blood {
                sprite,
                stain,
                fading,
            } => (
                Some([0, 2, 4, 56][usize::from(sprite)] + usize::from(stain)),
                if stain {
                    3
                } else if fading {
                    2
                } else {
                    0
                },
                false,
            ),
        };
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
        surfaces.push(Surface {
            depth: z,
            blend,
            vertices: quad(points, [color; 4], image, blend),
        });
    }
    let actor_trails = game
        .visible
        .iter()
        .flat_map(|&id| game.roster.get(id).trails.iter().flatten());
    for trail in actor_trails
        .chain(&game.items.trails)
        .filter(|_| !inspection)
    {
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
    let ui_start = surfaces.len();
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
        .chain(game.hud.sprites.iter().filter(|_| show_hud))
        .chain(&game.cinematic.sprites)
    {
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
    surfaces.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    let mut out = Geometry {
        vertices: Vec::new(),
        batches: Vec::new(),
    };
    for surface in surfaces {
        if surface.vertices.is_empty() {
            continue;
        }
        let start = out.vertices.len() as u32;
        out.vertices.extend(surface.vertices);
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
