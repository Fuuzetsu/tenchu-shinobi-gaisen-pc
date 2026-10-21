use crate::visuals::{
    interpolation::{Frame, Part},
    models::Models,
};
use bytemuck::{Pod, Zeroable};
use tenchu_assets::{
    model::{Face, Geometry},
    stage::MissionAssets,
};
use tenchu_core::math::{Math, Matrix};

#[cfg(test)]
mod tests;

pub const TEXTURED: u32 = 1;
pub const SCREEN: u32 = 2;
pub const TRANSLUCENT: u32 = 4;
pub const SCREEN_WIDE: u32 = 8;
pub const DOUBLE_SIDED: u32 = 16;
pub const FOG: u32 = 32;
pub const DEBUG: u32 = 64;
pub const XRAY: u32 = 128;
pub const UI: u32 = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) page: u32,
    pub(crate) palette: u32,
    pub(crate) flags: u32,
    pub(crate) material: u32,
    pub(crate) normal: [f32; 3],
}

pub(crate) fn draw_tmd_faces(geometry: &Geometry) -> impl Iterator<Item = &Face> {
    geometry.faces.iter().take_while(|face| face.flags & 1 != 0)
}

fn append_geometry(
    vertices: &mut Vec<Vertex>,
    geometries: &[Geometry],
    matrix: Matrix,
    models: Option<&Models>,
) {
    if models.is_some_and(|models| {
        models.visit(geometries, matrix, true, |triangle| {
            if triangle[0].flags & TRANSLUCENT == 0 {
                vertices.extend(triangle);
            }
        })
    }) {
        return;
    }
    for geometry in geometries {
        for face in draw_tmd_faces(geometry) {
            if !face.translucent {
                append_face(vertices, geometry, face, matrix, true);
            }
        }
    }
}

pub(crate) fn append_face(
    vertices: &mut Vec<Vertex>,
    geometry: &Geometry,
    face: &Face,
    matrix: Matrix,
    fog: bool,
) {
    let points: Vec<_> = face
        .corners
        .iter()
        .map(|c| {
            matrix
                .transform(geometry.vertices[c.vertex as usize])
                .map(|x| x as f32)
                .to_array()
        })
        .collect();
    let a = glam::Vec3::from(points[0]);
    let b = glam::Vec3::from(points[1]);
    let c = glam::Vec3::from(points[2]);
    if (b - a).cross(c - a).length_squared() == 0.0 {
        return;
    }
    let order = if face.corners.len() == 4 {
        &[0, 1, 2, 1, 3, 2][..]
    } else {
        &[0, 1, 2][..]
    };
    for &i in order {
        let corner = face.corners[i];
        vertices.push(Vertex {
            material: 0,
            normal: [0.0; 3],
            position: points[i],
            color: corner
                .color
                .map(|c| f32::from(c) / if face.texture.is_some() { 128.0 } else { 255.0 }),
            uv: corner.uv.map(f32::from),
            page: face.texture.map_or(0, |t| u32::from(t.page)),
            palette: face.texture.map_or(0, |t| u32::from(t.palette)),
            flags: if face.texture.is_some() { TEXTURED } else { 0 }
                | if face.double_sided { DOUBLE_SIDED } else { 0 }
                | if face.translucent { TRANSLUCENT } else { 0 }
                | if fog { FOG } else { 0 },
        });
    }
}

pub fn visit_stage(
    mission: &MissionAssets,
    math: &Math,
    mut visit: impl FnMut(&[Geometry], Matrix),
) {
    for part in &mission.map {
        visit(
            &part.geometry,
            Matrix {
                translation: part.position.map(|v| i32::from(v) * 10).into(),
                ..Matrix::default()
            },
        );
    }
    for placement in &mission.placements {
        visit(
            &mission.objects[placement.part].geometry,
            Matrix {
                translation: placement.position.into(),
                rotation: math.rotate_yxz([0, placement.facing as i16, 0]),
            },
        );
    }
}

fn visit_actor(
    game: &tenchu_core::game::Game,
    actor: &tenchu_core::actor::Actor,
    inspection: bool,
    visit: &mut impl FnMut(Part, &[Geometry], Matrix),
) {
    for (i, (part, matrix)) in game.mission.models[&actor.rendered_mesh_kind]
        .iter()
        .zip(actor.rendered.iter().copied())
        .enumerate()
    {
        if actor.render_mask & (1 << i) != 0 {
            visit(
                Part::Actor {
                    root: actor.root_model,
                    model: actor.parts[i],
                    kind: actor.rendered_mesh_kind,
                    inspection,
                },
                &part.geometry,
                matrix,
            );
        }
    }
    for weapon in actor.weapons[..2].iter().flatten() {
        visit(
            Part::Actor {
                root: actor.root_model,
                model: weapon.model,
                kind: weapon.kind,
                inspection,
            },
            &game.mission.weapons.models[&weapon.kind],
            weapon.rendered,
        );
    }
}

pub fn visit_character_parts(
    game: &tenchu_core::game::Game,
    inspection: bool,
    mut visit: impl FnMut(Part, &[Geometry], Matrix),
) {
    if !inspection {
        for &id in &game.visible {
            visit_actor(game, game.roster.get(id), false, &mut visit);
        }
        return;
    }
    for &id in &game.roster.order {
        let source = game.roster.get(id);
        if source.attribute & tenchu_core::ids::ATTR_SUSPEND != 0 {
            continue;
        }
        let mut actor = source.clone();
        actor.initialize_rendered(&game.scene, &game.math);
        actor.render_mask = actor.parts.iter().enumerate().fold(0, |mask, (i, &part)| {
            mask | if game.scene.models[part].attribute & 1 == 0 {
                1 << i
            } else {
                0
            }
        });
        visit_actor(game, &actor, true, &mut visit);
    }
}

pub fn visit_hinged_parts(
    game: &tenchu_core::game::Game,
    mut visit: impl FnMut(Part, &[Geometry], Matrix),
) {
    use tenchu_assets::misc::Kind;
    for (index, object) in game.misc.objects.iter().enumerate() {
        let (data, yaw) = match object.kind {
            Kind::Door { kind, rotation } => (&game.mission.misc.doors[kind], rotation),
            Kind::Pitfall { kind, rotation } => (&game.mission.misc.pitfalls[kind], rotation),
            _ => continue,
        };
        let root = Matrix {
            translation: object.position,
            rotation: game.math.rotate_yxz([0, yaw, 0]),
        };
        for (part, (mesh, local)) in data
            .models
            .into_iter()
            .zip(object.hinged_matrices(data, &game.math))
            .enumerate()
        {
            if let Some(mesh) = mesh {
                visit(
                    Part::Hinge {
                        object: index,
                        part,
                        mesh,
                    },
                    &game.mission.common_models[&mesh],
                    root.compose(&local),
                );
            }
        }
    }
}

fn append_caster(
    vertices: &mut Vec<Vertex>,
    geometries: &[Geometry],
    matrix: Matrix,
    models: Option<&Models>,
) {
    if models.is_some_and(|models| {
        models.visit(geometries, matrix, false, |triangle| {
            vertices.extend(triangle)
        })
    }) {
        return;
    }
    for geometry in geometries {
        for face in draw_tmd_faces(geometry) {
            append_face(vertices, geometry, face, matrix, false);
        }
    }
}

pub fn shadow_stage(mission: &MissionAssets, math: &Math, models: Option<&Models>) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    visit_stage(mission, math, |geometry, matrix| {
        append_caster(&mut vertices, geometry, matrix, models)
    });
    vertices
}

pub fn shadow_models(game: &tenchu_core::game::Game, models: Option<&Models>) -> Vec<Vertex> {
    shadow_models_presented(game, models, None)
}

pub fn shadow_models_presented(
    game: &tenchu_core::game::Game,
    models: Option<&Models>,
    frame: Option<&Frame>,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let mut append = |key, geometry: &[Geometry], matrix| {
        append_caster(
            &mut vertices,
            geometry,
            Frame::transform(frame, key, matrix),
            models,
        )
    };
    visit_hinged_parts(game, &mut append);
    crate::visuals::appearance::visit(game, true, models, frame, |geometry, matrix| {
        append_caster(&mut vertices, geometry, matrix, models);
    });
    vertices
}

pub fn stage(mission: &MissionAssets, math: &Math, models: Option<&Models>) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    visit_stage(mission, math, |geometry, matrix| {
        append_geometry(&mut vertices, geometry, matrix, models)
    });
    vertices
}

pub fn characters(game: &tenchu_core::game::Game, models: Option<&Models>) -> Vec<Vertex> {
    characters_presented(game, models, false, None)
}

pub fn characters_presented(
    game: &tenchu_core::game::Game,
    models: Option<&Models>,
    inspection: bool,
    frame: Option<&Frame>,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    crate::visuals::appearance::visit(game, inspection, models, frame, |geometry, matrix| {
        append_geometry(&mut vertices, geometry, matrix, models)
    });
    vertices
}
