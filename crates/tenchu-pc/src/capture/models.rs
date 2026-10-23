use super::review::{self, Review};
use crate::{
    render_mesh::{self, Vertex},
    visuals::{catalog, models::Models},
};
use glam::{Mat4, Vec3};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use tenchu_assets::{Result, model::Geometry};
use tenchu_core::{
    game::Game,
    math::Matrix,
    motion::{Change, Motion},
};

struct Model<'a> {
    label: String,
    parts: Vec<(&'a [Geometry], Matrix)>,
}

#[derive(Default)]
struct MeshData {
    opaque: Vec<Vertex>,
    translucent: Vec<(u8, Vec<Vertex>)>,
}

impl Model<'_> {
    fn mesh(&self, replacements: Option<&Models>) -> MeshData {
        let mut mesh = MeshData::default();
        for &(geometries, matrix) in &self.parts {
            if replacements.is_some_and(|models| {
                models.visit(geometries, matrix, false, |triangle| {
                    if triangle[0].flags & render_mesh::TRANSLUCENT != 0 {
                        let blend = ((triangle[0].page >> 5) & 3) as u8 + 1;
                        mesh.translucent.push((blend, triangle.to_vec()));
                    } else {
                        mesh.opaque.extend(triangle);
                    }
                })
            }) {
                continue;
            }
            for geometry in geometries {
                for face in render_mesh::draw_tmd_faces(geometry) {
                    if face.translucent {
                        let mut vertices = Vec::new();
                        render_mesh::append_face(&mut vertices, geometry, face, matrix, false);
                        let blend = face.texture.map_or(1, |t| ((t.page >> 5) & 3) as u8 + 1);
                        mesh.translucent.push((blend, vertices));
                    } else {
                        render_mesh::append_face(&mut mesh.opaque, geometry, face, matrix, false);
                    }
                }
            }
        }
        mesh
    }
}

fn models(game: &Game) -> Result<BTreeMap<String, Model<'_>>> {
    let mission = &game.mission;
    let mut models = BTreeMap::new();
    for (&kind, parts) in &mission.models {
        let definition = mission
            .characters
            .iter()
            .find(|c| c.kind == kind)
            .ok_or("Missing actor definition")?;
        let mut motion = Motion::new(parts)?;
        if motion.change(
            tenchu_core::ids::MOT_NORMAL,
            &definition.motions,
            &mission.common_motions,
            &mission.clips,
        ) == Change::Missing
        {
            return Err(format!("Actor {kind:04x} has no initial motion"));
        }
        for _ in 0..30 {
            motion.step(&mission.clips, &game.math.tables.hermite)?;
        }
        let matrices = motion.pose.matrices(&game.math, Matrix::default());
        models.insert(
            format!("actors/{kind:04x}"),
            Model {
                label: definition.model.clone(),
                parts: parts
                    .iter()
                    .zip(matrices)
                    .map(|(part, matrix)| (part.geometry.as_slice(), matrix))
                    .collect(),
            },
        );
    }
    for model in catalog::models(mission) {
        if !model.id.starts_with("actors/") {
            models.insert(
                model.id,
                Model {
                    label: model.label,
                    parts: vec![(model.geometry, Matrix::default())],
                },
            );
        }
    }
    Ok(models)
}

pub fn render(game: &Game, output: &Path, ids: &[&str], grey_background: bool) -> Result<()> {
    let models = models(game)?;
    let mut selected = BTreeSet::new();
    for id in ids {
        let matching: Vec<_> = models
            .keys()
            .filter(|key| key.as_str() == *id || key.starts_with(&format!("{id}/")))
            .cloned()
            .collect();
        if matching.is_empty() {
            return Err(format!("Unknown model or model group {id}"));
        }
        selected.extend(matching);
    }
    review::prepare_directory(output)?;
    let mut review = Review::new(&game.mission)?;
    let replacements = review.models(&game.mission)?;
    let backdrop = grey_background.then(|| review.backdrop(0.4));
    let mut views = Vec::new();
    for id in selected {
        let model = &models[&id];
        let variants = [model.mesh(None), model.mesh(Some(&replacements))];
        let mut textures = BTreeSet::new();
        for &(geometries, _) in &model.parts {
            for geometry in geometries {
                for face in render_mesh::draw_tmd_faces(geometry) {
                    if let Some(region) = catalog::face_region(&game.mission.vram, face) {
                        textures.insert(catalog::image_id(
                            [region.rect[2], region.rect[3]],
                            &region.original_pixels(&game.mission.vram),
                        ));
                    }
                }
            }
        }
        let vertices: Vec<_> = variants
            .iter()
            .flat_map(|mesh| {
                mesh.opaque
                    .iter()
                    .chain(mesh.translucent.iter().flat_map(|(_, v)| v))
            })
            .collect();
        if vertices.is_empty() {
            return Err(format!("Model {id} has no visible faces"));
        }
        let (min, max) = vertices.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(min, max), vertex| {
                let p = Vec3::from(vertex.position);
                (min.min(p), max.max(p))
            },
        );
        let center = (min + max) * 0.5;
        let radius = (max - min).length().max(1.0) * 0.5;
        let meshes = variants
            .each_ref()
            .map(|mesh| review.renderer.upload(&mesh.opaque));
        let stem = id.replace('/', "_");
        for (name, direction) in [
            ("front", Vec3::new(0.0, -0.12, -1.0)),
            ("right", Vec3::new(1.0, -0.12, 0.0)),
            ("back", Vec3::new(0.0, -0.12, 1.0)),
            ("left", Vec3::new(-1.0, -0.12, 0.0)),
            ("above", Vec3::new(0.3, -1.0, -0.4)),
            ("below", Vec3::new(-0.3, 1.0, 0.4)),
        ] {
            let eye = center + direction.normalize() * radius * 2.8;
            let view = Mat4::look_at_rh(eye, center, Vec3::NEG_Y);
            let camera = Mat4::perspective_rh(
                45.0_f32.to_radians(),
                review::SIZE[0] as f32 / review::SIZE[1] as f32,
                radius * 0.001,
                radius * 10.0,
            ) * view;
            for (index, (enabled, suffix)) in [(false, "original"), (true, "pack")]
                .into_iter()
                .enumerate()
            {
                let effects = review::effects(&variants[index].translucent, view);
                let effects = review.renderer.upload_effects(effects);
                let meshes: Vec<_> = backdrop
                    .iter()
                    .chain(std::iter::once(&meshes[index]))
                    .collect();
                review.capture(
                    camera,
                    &meshes,
                    Some(&effects),
                    enabled,
                    &output.join(format!("{stem}-{name}-{suffix}.png")),
                )?;
            }
            views.push(
                serde_json::json!({"model":id,"name":model.label,"stem":stem,
                "view":name,"textures":textures,"eye":eye.to_array(),"target":center.to_array(),
                "pose":"idle after 30 motion updates; no gameplay simulation",
                "background":if grey_background { "grey" } else { "black" }}),
            );
        }
    }
    fs::write(
        output.join("views.json"),
        serde_json::to_vec_pretty(&views).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
