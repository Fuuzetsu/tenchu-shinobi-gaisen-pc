use bytemuck::{Pod, Zeroable};
use tenchu_assets::{
    model::{Face, Geometry},
    stage::MissionAssets,
};
use tenchu_core::math::{Math, Matrix};

pub const TEXTURED: u32 = 1;
pub const SCREEN: u32 = 2;
pub const TRANSLUCENT: u32 = 4;
pub const SCREEN_WIDE: u32 = 8;
pub const DOUBLE_SIDED: u32 = 16;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) page: u32,
    pub(crate) palette: u32,
    pub(crate) flags: u32,
}

fn append_geometry(vertices: &mut Vec<Vertex>, geometries: &[Geometry], matrix: Matrix) {
    for geometry in geometries {
        for face in &geometry.faces {
            if !face.translucent {
                append_face(vertices, geometry, face, matrix);
            }
        }
    }
}

pub(crate) fn append_face(
    vertices: &mut Vec<Vertex>,
    geometry: &Geometry,
    face: &Face,
    matrix: Matrix,
) {
    let points: Vec<_> = face
        .corners
        .iter()
        .map(|c| {
            matrix
                .transform(geometry.vertices[c.vertex as usize])
                .map(|x| x as f32)
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
            position: points[i],
            color: corner
                .color
                .map(|c| f32::from(c) / if face.texture.is_some() { 128.0 } else { 255.0 }),
            uv: corner.uv.map(f32::from),
            page: face.texture.map_or(0, |t| u32::from(t.page)),
            palette: face.texture.map_or(0, |t| u32::from(t.palette)),
            flags: if face.texture.is_some() { TEXTURED } else { 0 }
                | if face.double_sided { DOUBLE_SIDED } else { 0 }
                | if face.translucent { TRANSLUCENT } else { 0 },
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
                translation: part.position.map(|v| i32::from(v) * 10),
                ..Matrix::default()
            },
        );
    }
    for placement in &mission.placements {
        visit(
            &mission.objects[placement.part].geometry,
            Matrix {
                translation: placement.position,
                rotation: math.rotate_yxz([0, placement.facing as i16, 0]),
            },
        );
    }
}

pub fn visit_characters(
    game: &tenchu_core::game::Game,
    mut visit: impl FnMut(&[Geometry], Matrix),
) {
    for &id in &game.visible {
        let actor = game.roster.get(id);
        for (i, (part, matrix)) in game.mission.models[&actor.rendered_mesh_kind]
            .iter()
            .zip(actor.rendered.iter().copied())
            .enumerate()
        {
            if actor.render_mask & (1 << i) != 0 {
                visit(&part.geometry, matrix);
            }
        }
        for weapon in actor.weapons[..2].iter().flatten() {
            visit(&game.mission.weapons.models[&weapon.kind], weapon.rendered);
        }
    }
}

pub fn stage(mission: &MissionAssets, math: &Math) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    visit_stage(mission, math, |geometry, matrix| {
        append_geometry(&mut vertices, geometry, matrix)
    });
    vertices
}

pub fn characters(game: &tenchu_core::game::Game) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    visit_characters(game, |geometry, matrix| {
        append_geometry(&mut vertices, geometry, matrix)
    });
    vertices
}
