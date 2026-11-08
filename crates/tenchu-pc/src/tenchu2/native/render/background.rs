use super::{Surface, catalog, render_effects, render_mesh};
use crate::{renderer, visuals::models::Meshes};
use tenchu_assets::tenchu2::level::{Level, background::Background};
use tenchu_core::math::{Math, Matrix};

pub(super) fn sources(level: &Level) -> Vec<catalog::ModelSource<'_>> {
    let mut sources = Vec::new();
    for (index, sky) in level.background.sky.iter().enumerate() {
        sources.extend(super::sources(
            &format!("map{}/sky{index}", level.id),
            std::slice::from_ref(&sky.model),
        ));
    }
    if let Some(landscape) = &level.background.landscape {
        sources.extend(super::sources(
            &format!("map{}/background", level.id),
            std::slice::from_ref(landscape),
        ));
    }
    sources
}

pub(super) fn geometry(
    background: &Background,
    meshes: &Meshes,
    view: Matrix,
    math: &Math,
    tick: u32,
) -> Vec<render_effects::Geometry> {
    let eye = renderer::view_matrix(view)
        .inverse()
        .transform_point3(glam::Vec3::ZERO);
    let mut layers = Vec::new();
    for sky in &background.sky {
        let yaw = sky.yaw.wrapping_add(if sky.drift {
            tick.div_ceil(2) as i16
        } else {
            0
        });
        let matrix = Matrix {
            rotation: math.rotate_yxz([0, yaw, 0]),
            translation: sky.model.position.map(i32::from).into(),
        };
        layers.push(layer(meshes, &sky.model, matrix, 1.0, eye, view));
    }
    if let Some(landscape) = &background.landscape {
        layers.push(layer(
            meshes,
            landscape,
            Matrix::default(),
            1024.0,
            glam::Vec3::ZERO,
            view,
        ));
    }
    layers
}

fn layer(
    meshes: &Meshes,
    part: &tenchu_assets::model::Part,
    matrix: Matrix,
    scale: f32,
    offset: glam::Vec3,
    view: Matrix,
) -> render_effects::Geometry {
    let mut surfaces = Vec::new();
    meshes.visit(&part.geometry, matrix, false, 0, None, |triangle| {
        let vertices = triangle.map(|v| {
            let mut v = *v;
            v.position = (glam::Vec3::from(v.position) * scale + offset).to_array();
            v.flags &= render_mesh::TEXTURED | render_mesh::TRANSLUCENT | render_mesh::DOUBLE_SIDED;
            v.flags |= render_mesh::SKY | render_mesh::UNLIT;
            v
        });
        let depth = vertices
            .iter()
            .map(|v| {
                (0..3)
                    .map(|i| f32::from(view.rotation[2][i]) * v.position[i] / 4096.0)
                    .sum::<f32>()
                    + view.translation.z as f32
            })
            .sum::<f32>()
            / 3.0;
        surfaces.push(Surface {
            depth,
            blend: if vertices[0].flags & render_mesh::TRANSLUCENT != 0 {
                ((vertices[0].page >> 5) & 3) as u8 + 1
            } else {
                0
            },
            vertices: vertices.to_vec(),
        });
    });
    render_effects::surface_geometry(surfaces)
}
