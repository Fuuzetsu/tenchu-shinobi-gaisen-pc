use bytemuck::{Pod, Zeroable};
use tenchu_assets::{
    model::{Face, Geometry, Part},
    stage::MissionAssets,
};
use tenchu_core::math::{Math, Matrix};

pub const TEXTURED: u32 = 1;
pub const SCREEN: u32 = 2;
pub const TRANSLUCENT: u32 = 4;

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

pub fn append(vertices: &mut Vec<Vertex>, part: &Part, matrix: Matrix) {
    append_geometry(vertices, &part.geometry, matrix);
}

fn append_geometry(vertices: &mut Vec<Vertex>, geometries: &[Geometry], matrix: Matrix) {
    for geometry in geometries {
        for face in &geometry.faces {
            append_face(vertices, geometry, face, matrix);
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
            flags: if face.texture.is_some() { TEXTURED } else { 0 },
        });
    }
}

pub fn stage(mission: &MissionAssets, math: &Math) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for part in &mission.map {
        append(
            &mut vertices,
            part,
            Matrix {
                translation: part.position.map(|v| i32::from(v) * 10),
                ..Matrix::default()
            },
        );
    }
    for placement in &mission.placements {
        append(
            &mut vertices,
            &mission.objects[placement.part],
            Matrix {
                translation: placement.position,
                rotation: math.rotate_yxz([0, placement.facing as i16, 0]),
            },
        );
    }
    vertices
}

pub fn characters(game: &tenchu_core::game::Game) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for &id in &game.visible {
        let actor = game.roster.get(id);
        for (i, (part, matrix)) in game.mission.models[&actor.kind]
            .iter()
            .zip(actor.rendered.iter().copied())
            .enumerate()
        {
            if actor.render_mask & (1 << i) != 0 {
                append(&mut vertices, part, matrix);
            }
        }
        for weapon in actor.weapons[..2].iter().flatten() {
            append_geometry(
                &mut vertices,
                &game.mission.weapons.models[&weapon.kind],
                weapon.rendered,
            );
        }
    }
    vertices
}
