use bytemuck::{Pod, Zeroable};
use tenchu_assets::{
    model::{Geometry, Part},
    stage::MissionAssets,
};
use tenchu_core::math::{Math, Matrix};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    uv: [f32; 2],
    page: u32,
    palette: u32,
    textured: u32,
}

pub fn append(vertices: &mut Vec<Vertex>, part: &Part, matrix: Matrix) {
    append_geometry(vertices, &part.geometry, matrix);
}

fn append_geometry(vertices: &mut Vec<Vertex>, geometries: &[Geometry], matrix: Matrix) {
    for geometry in geometries {
        for face in &geometry.faces {
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
                continue;
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
                    textured: u32::from(face.texture.is_some()),
                });
            }
        }
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

pub fn player(game: &tenchu_core::game::Game) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let actor = game.player();
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
    vertices
}
