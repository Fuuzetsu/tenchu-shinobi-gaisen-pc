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

mod bounds;
pub(crate) use bounds::Bounds;

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
pub const EMITTER: u32 = 512;
pub const SCENERY: u32 = 1024;
pub const ALPHA: u32 = 2048;
pub const STAIN: u32 = 4096;
pub const SKY: u32 = 8192;
pub const SOFT: u32 = 16384;
pub const ORIGINAL_WEATHER: u32 = 32768;
pub const WATER: u32 = 65536;
pub const WATERFALL: u32 = 131072;
pub const FOLIAGE: u32 = 262144;
pub const FLUTTER: u32 = 524288;
pub const FOOTPRINT: u32 = 1048576;
pub const WET_CONTACT: u32 = 2097152;
pub const ORIGINAL_SPLASH: u32 = 4194304;
pub const UNLIT: u32 = 8388608;
// The high byte holds foliage weights, blood presentation or snowy terrain tags.
pub const BLOOD_HD: u32 = 1 << 24;
pub const SNOW: u32 = 1 << 25;
pub const WEAPON_METAL: u32 = 1 << 26;
pub const WEAPON_COAT: u32 = 1 << 27;
#[cfg(not(target_arch = "wasm32"))]
pub const BAKED_FOG: u32 = 1 << 28;
pub const WIND_WEIGHT_SHIFT: u32 = 24;

pub fn original_hd_blood(flags: u32) -> bool {
    flags & (STAIN | SOFT | FOLIAGE | BLOOD_HD) == STAIN | BLOOD_HD
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, serde::Serialize, serde::Deserialize)]
pub struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) page: u32,
    pub(crate) palette: u32,
    pub(crate) flags: u32,
    pub(crate) material: u32,
    pub(crate) normal: [f32; 3],
    pub(crate) opacity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Pose {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl From<&Vertex> for Pose {
    fn from(vertex: &Vertex) -> Self {
        Self {
            position: vertex.position,
            normal: vertex.normal,
        }
    }
}

#[derive(Default)]
pub struct Indexed {
    pub(crate) poses: Option<crate::visuals::posing::Frame>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub groups: Vec<IndexedGroup>,
}

pub struct IndexedGroup {
    pub vertices: std::ops::Range<usize>,
    pub indices: std::ops::Range<u32>,
}

pub(crate) fn draw_tmd_faces(geometry: &Geometry) -> impl Iterator<Item = &Face> {
    geometry.faces.iter().take_while(|face| face.flags & 1 != 0)
}

pub(crate) fn face_order(face: &Face) -> &'static [usize] {
    if face.corners.len() == 4 {
        &[0, 1, 2, 1, 3, 2]
    } else {
        &[0, 1, 2]
    }
}

pub(crate) fn append_geometry(
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
    for &i in face_order(face) {
        let corner = face.corners[i];
        vertices.push(Vertex {
            material: 0,
            normal: [0.0; 3],
            opacity: 1.0,
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
    for part in mission.map.iter() {
        visit(
            &part.geometry,
            Matrix {
                translation: part.position.map(|v| i32::from(v) * 10).into(),
                ..Matrix::default()
            },
        );
    }
    for (index, placement) in mission.placements.iter().enumerate() {
        if mission
            .trial
            .as_ref()
            .is_some_and(|t| t.collapsing.contains(&index))
        {
            continue;
        }
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
    for weapon in actor
        .weapons
        .iter()
        .enumerate()
        .filter_map(|(slot, weapon)| {
            (!matches!(slot, 2 | 3))
                .then_some(weapon.as_ref())
                .flatten()
        })
    {
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
    for slot in 2..4 {
        if let Some((part, mount)) = crate::sheathing::stowed_mount(actor, slot) {
            let weapon = actor.weapons[slot].as_ref().unwrap();
            if let Some(grip) = game.mission.weapons.grips.get(&weapon.kind) {
                visit(
                    Part::Actor {
                        root: actor.root_model,
                        model: weapon.model,
                        kind: weapon.kind,
                        inspection,
                    },
                    grip,
                    actor.rendered[part].compose(&mount),
                );
            }
        }
    }
}

pub fn visit_character_parts(
    game: &tenchu_core::game::Game,
    inspection: bool,
    mut visit: impl FnMut(Part, &[Geometry], Matrix),
) {
    if !inspection && game.drawing.step == tenchu_core::drawing::Step::Normal {
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
        // Console draw skips suppress submission, not actor existence. Rebuild only
        // presentation transforms; the source pose, visibility and caches stay intact.
        visit_actor(game, &actor, inspection, &mut visit);
    }
}

pub fn visit_hinged_parts(
    game: &tenchu_core::game::Game,
    mut visit: impl FnMut(Part, &[Geometry], Matrix),
) {
    use tenchu_assets::misc::Kind;
    if let Some(trial) = &game.mission.trial {
        for &index in &trial.collapsing {
            let placement = &game.mission.placements[index];
            if placement.position[1] >= 1000000 {
                continue;
            }
            visit(
                Part::TrialCover(index),
                &game.mission.objects[placement.part].geometry,
                Matrix {
                    translation: placement.position.into(),
                    rotation: game.math.rotate_yxz([0, placement.facing as i16, 0]),
                },
            );
        }
    }
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
    for vertex in &mut vertices {
        vertex.flags |= SCENERY;
    }
    vertices
}

#[cfg(not(target_arch = "wasm32"))]
pub fn shadow_models(game: &tenchu_core::game::Game, models: Option<&Models>) -> Vec<Vertex> {
    shadow_models_presented(game, models, None)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn shadow_models_presented(
    game: &tenchu_core::game::Game,
    models: Option<&Models>,
    frame: Option<&Frame>,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    update_shadow_models(&mut vertices, game, models, frame, false);
    vertices
}

pub fn update_shadow_models(
    vertices: &mut Vec<Vertex>,
    game: &tenchu_core::game::Game,
    models: Option<&Models>,
    frame: Option<&Frame>,
    reuse_opaque: bool,
) {
    vertices.clear();
    let mut append = |key, geometry: &[Geometry], matrix| {
        append_caster(
            vertices,
            geometry,
            Frame::transform(frame, key, matrix),
            models,
        )
    };
    visit_hinged_parts(game, &mut append);
    for vertex in vertices.iter_mut() {
        vertex.flags |= SCENERY;
    }
    crate::visuals::appearance::visit_parts(game, true, models, frame, |key, geometry, matrix| {
        append_character(vertices, geometry, matrix, models, key, true, reuse_opaque);
    });
}

pub fn stage(mission: &MissionAssets, math: &Math, models: Option<&Models>) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    visit_stage(mission, math, |geometry, matrix| {
        append_geometry(&mut vertices, geometry, matrix, models)
    });
    for vertex in &mut vertices {
        vertex.flags |= SCENERY;
    }
    vertices
}

#[cfg(not(target_arch = "wasm32"))]
pub fn characters(game: &tenchu_core::game::Game, models: Option<&Models>) -> Vec<Vertex> {
    characters_presented(game, models, false, None)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn characters_presented(
    game: &tenchu_core::game::Game,
    models: Option<&Models>,
    inspection: bool,
    frame: Option<&Frame>,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    update_characters(&mut vertices, game, models, inspection, frame);
    vertices
}

#[cfg(not(target_arch = "wasm32"))]
pub fn update_characters(
    vertices: &mut Vec<Vertex>,
    game: &tenchu_core::game::Game,
    models: Option<&Models>,
    inspection: bool,
    frame: Option<&Frame>,
) {
    vertices.clear();
    crate::visuals::appearance::visit_parts(
        game,
        inspection,
        models,
        frame,
        |key, geometry, matrix| {
            append_character(vertices, geometry, matrix, models, key, false, false)
        },
    );
}

pub fn update_indexed_characters(
    mesh: &mut Indexed,
    game: &tenchu_core::game::Game,
    models: Option<&Models>,
    inspection: bool,
    frame: Option<&Frame>,
) {
    mesh.vertices.clear();
    mesh.indices.clear();
    mesh.groups.clear();
    if let Some(poses) = &mut mesh.poses {
        poses.clear(models.map(|m| &m.pose_generation));
    }
    let mut previous_root = None;
    crate::visuals::appearance::visit_parts(
        game,
        inspection,
        models,
        frame,
        |key, geometry, matrix| {
            let base = mesh.vertices.len();
            let start = mesh.indices.len() as u32;
            if !models
                .is_some_and(|models| models.append_indexed_part(mesh, key, geometry, matrix, true))
            {
                append_character(
                    &mut mesh.vertices,
                    geometry,
                    matrix,
                    None,
                    key,
                    false,
                    false,
                );
                mesh.indices.extend(base as u32..mesh.vertices.len() as u32);
            }
            let end = mesh.indices.len() as u32;
            if start == end {
                return;
            }
            let root = match key {
                Part::Actor { root, .. } => Some(root),
                _ => None,
            };
            if root.is_some() && root == previous_root {
                let group = mesh.groups.last_mut().unwrap();
                group.vertices.end = mesh.vertices.len();
                group.indices.end = end;
            } else {
                mesh.groups.push(IndexedGroup {
                    vertices: base..mesh.vertices.len(),
                    indices: start..end,
                });
            }
            previous_root = root;
        },
    );
}

fn append_character(
    vertices: &mut Vec<Vertex>,
    geometry: &[Geometry],
    matrix: Matrix,
    models: Option<&Models>,
    key: Part,
    shadow: bool,
    translucent_only: bool,
) {
    if models.is_some_and(|models| {
        models.visit_part(
            key,
            geometry,
            matrix,
            !shadow,
            if translucent_only { TRANSLUCENT } else { 0 },
            |triangle| {
                if shadow || triangle[0].flags & TRANSLUCENT == 0 {
                    vertices.extend(triangle);
                }
            },
        )
    }) {
        return;
    }
    for geometry in geometry {
        for face in draw_tmd_faces(geometry).filter(|face| {
            if translucent_only {
                face.translucent
            } else {
                shadow || !face.translucent
            }
        }) {
            append_face(vertices, geometry, face, matrix, !shadow);
        }
    }
}
