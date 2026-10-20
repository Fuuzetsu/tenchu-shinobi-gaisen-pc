use super::{catalog, pack::Pack, pixels::Image, smooth};
use crate::{
    render_mesh::{self, Vertex},
    settings::Graphics,
};
use glam::{Mat3, Vec3};
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{Result, model::Geometry, stage::MissionAssets};
use tenchu_core::math::Matrix;

pub struct Material {
    pub id: String,
    pub region: Option<catalog::Region>,
    pub image: Arc<Image>,
}

#[derive(Default)]
pub struct Models {
    meshes: BTreeMap<usize, Vec<Vertex>>,
    materials: Vec<Material>,
    pub replacements: usize,
}

impl Models {
    pub fn build(mission: &MissionAssets, pack: &Pack, graphics: &Graphics) -> Result<Self> {
        let mut out = Self::default();
        if !(graphics.asset_pack && !pack.models.is_empty())
            && !graphics.smooth_models
            && graphics.model_detail == 0
        {
            return Ok(out);
        }
        let region_ids = catalog::regions(mission);
        let regions: BTreeMap<_, _> = region_ids.iter().map(|(&r, id)| (id.clone(), r)).collect();
        let mut textures = BTreeMap::new();
        for source in catalog::models(mission) {
            let replacement = graphics
                .asset_pack
                .then(|| pack.models.get(&source.id))
                .flatten();
            let round = source.id.starts_with("actors/") && replacement.is_none();
            if replacement.is_none()
                && !graphics.smooth_models
                && !(round && graphics.model_detail != 0)
            {
                continue;
            }
            if source.geometry.is_empty() {
                if replacement.is_some() {
                    return Err(format!("{} has no replaceable geometry", source.id));
                }
                continue;
            }
            let mut vertices = Vec::new();
            if let Some(model) = replacement {
                out.replacements += 1;
                for triangle in model {
                    let definition = &pack.materials[&triangle.material];
                    let material = if let Some(id) = &definition.texture {
                        let region = regions.get(id).copied();
                        *textures.entry((id.clone(), region)).or_insert_with(|| {
                            out.materials.push(Material {
                                id: id.clone(),
                                region,
                                image: pack.images[id].clone(),
                            });
                            out.materials.len() as u32
                        })
                    } else {
                        0
                    };
                    let region = (material != 0)
                        .then(|| out.materials[material as usize - 1].region)
                        .flatten();
                    let rect = region.map_or([0.0, 0.0, 1.0, 1.0], |r| r.rect.map(f32::from));
                    for corner in triangle.corners {
                        vertices.push(Vertex {
                            position: corner.position,
                            color: corner.color,
                            normal: corner.normal,
                            uv: [
                                corner.uv[0] * rect[2] + rect[0] - 0.5,
                                (1.0 - corner.uv[1]) * rect[3] + rect[1] - 0.5,
                            ],
                            page: region.map_or(0, |r| u32::from(r.page))
                                | u32::from(definition.blend.saturating_sub(1)) << 5,
                            palette: region.map_or(0, |r| u32::from(r.palette)),
                            flags: render_mesh::FOG
                                | if material != 0 {
                                    render_mesh::TEXTURED
                                } else {
                                    0
                                }
                                | if definition.blend != 0 {
                                    render_mesh::TRANSLUCENT
                                } else {
                                    0
                                }
                                | if definition.double_sided {
                                    render_mesh::DOUBLE_SIDED
                                } else {
                                    0
                                },
                            material,
                        });
                    }
                }
            } else {
                for geometry in source.geometry {
                    for face in render_mesh::draw_tmd_faces(geometry) {
                        let start = vertices.len();
                        render_mesh::append_face(
                            &mut vertices,
                            geometry,
                            face,
                            Matrix::default(),
                            true,
                        );
                        if let Some(region) = catalog::face_region(&mission.vram, face) {
                            let id = &region_ids[&region];
                            let material = *textures
                                .entry((id.clone(), Some(region)))
                                .or_insert_with(|| {
                                    out.materials.push(Material {
                                        id: id.clone(),
                                        region: Some(region),
                                        image: Arc::new(Image {
                                            size: [
                                                u32::from(region.rect[2]),
                                                u32::from(region.rect[3]),
                                            ],
                                            rgba: region.pixels(&mission.vram),
                                        }),
                                    });
                                    out.materials.len() as u32
                                });
                            for vertex in &mut vertices[start..] {
                                vertex.material = material;
                            }
                        }
                    }
                }
            }
            smooth::prepare(
                &mut vertices,
                graphics.smooth_models,
                if round { graphics.model_detail } else { 0 },
            );
            out.meshes
                .insert(source.geometry.as_ptr() as usize, vertices);
        }
        Ok(out)
    }

    pub fn bind(&mut self, mut bind: impl FnMut(&Material) -> u32) {
        let handles: Vec<_> = self.materials.iter().map(&mut bind).collect();
        for vertex in self.meshes.values_mut().flatten() {
            if vertex.material != 0 {
                vertex.material = handles[vertex.material as usize - 1];
            }
        }
    }

    pub fn visit(
        &self,
        geometry: &[Geometry],
        matrix: Matrix,
        fog: bool,
        mut visit: impl FnMut([Vertex; 3]),
    ) -> bool {
        let Some(vertices) = self.meshes.get(&(geometry.as_ptr() as usize)) else {
            return false;
        };
        let rotation = rotation(matrix);
        let normal = rotation.inverse().transpose();
        let translation = Vec3::from(matrix.translation.map(|n| n as f32));
        for triangle in vertices.chunks_exact(3) {
            visit(std::array::from_fn(|i| {
                let mut vertex = triangle[i];
                vertex.position = (rotation * Vec3::from(vertex.position) + translation).to_array();
                vertex.normal = (normal * Vec3::from(vertex.normal))
                    .normalize_or_zero()
                    .to_array();
                vertex.flags =
                    (vertex.flags & !render_mesh::FOG) | if fog { render_mesh::FOG } else { 0 };
                vertex
            }));
        }
        true
    }
}

pub fn rotation(matrix: Matrix) -> Mat3 {
    Mat3::from_cols_array(&std::array::from_fn(|i| {
        f32::from(matrix.rotation[i % 3][i / 3]) / 4096.0
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_assets::game::GameAssets;

    #[test]
    #[ignore = "requires TENCHU_DISC; exported models preserve texture UVs, blends, winding and animation attachments through the replacement path"]
    fn disc_models_round_trip_without_changing_game_state() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        for character in 0..2 {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..8 {
                game.step(0).unwrap();
            }
            let before = crate::session::tests::state(&game);
            let root = std::env::temp_dir().join(format!(
                "tenchu-model-roundtrip-{}-{character}",
                std::process::id()
            ));
            super::super::export(&game.mission, &root).unwrap();
            let mut manifest: super::super::pack::Manifest =
                serde_json::from_slice(&std::fs::read(root.join("pack.json")).unwrap()).unwrap();
            let catalog: serde_json::Value =
                serde_json::from_slice(&std::fs::read(root.join("catalog.json")).unwrap()).unwrap();
            for model in catalog["models"].as_array().unwrap() {
                if model["faces"].as_u64().unwrap() != 0 {
                    manifest.models.insert(
                        model["id"].as_str().unwrap().into(),
                        model["file"].as_str().unwrap().into(),
                    );
                }
            }
            std::fs::write(
                root.join("pack.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            let pack = Pack::load(&root).unwrap();
            let models = Models::build(&game.mission, &pack, &Graphics::default()).unwrap();
            assert!(models.replacements > 500);
            let region_ids = super::catalog::regions(&game.mission);
            let mut checked = 0;
            let mut compare = |geometry: &[Geometry], matrix| {
                let mut original = Vec::new();
                let mut regions = Vec::new();
                for part in geometry {
                    for face in render_mesh::draw_tmd_faces(part) {
                        let start = original.len();
                        render_mesh::append_face(&mut original, part, face, matrix, true);
                        regions.extend(std::iter::repeat_n(
                            super::catalog::face_region(&game.mission.vram, face),
                            original.len() - start,
                        ));
                    }
                }
                let mut edited = Vec::new();
                if !models.visit(geometry, matrix, true, |triangle| edited.extend(triangle)) {
                    assert!(original.is_empty());
                    return;
                }
                assert_eq!(
                    edited.len(),
                    original.len(),
                    "{}: {matrix:?}",
                    super::catalog::models(&game.mission)
                        .into_iter()
                        .find(|m| m.geometry.as_ptr() == geometry.as_ptr())
                        .unwrap()
                        .id
                );
                for ((a, b), region) in edited.iter().zip(&original).zip(regions) {
                    assert_eq!(a.flags, b.flags);
                    if a.flags & render_mesh::TRANSLUCENT != 0 {
                        assert_eq!(a.page & 0x60, b.page & 0x60);
                    }
                    for axis in 0..3 {
                        assert!((a.position[axis] - b.position[axis]).abs() < 1.05);
                        assert!((a.color[axis] - b.color[axis]).abs() < 0.00001);
                    }
                    if let Some(region) = region {
                        let material = &models.materials[a.material as usize - 1];
                        assert_eq!(material.id, region_ids[&region]);
                        let mapped = material.region.unwrap();
                        for axis in 0..2 {
                            let u = (a.uv[axis] - f32::from(mapped.rect[axis]) + 0.5)
                                / f32::from(mapped.rect[axis + 2]);
                            let v = (b.uv[axis] - f32::from(region.rect[axis]) + 0.5)
                                / f32::from(region.rect[axis + 2]);
                            assert!((u - v).abs() < 0.00001, "{u} versus {v}");
                        }
                    }
                }
                checked += edited.len();
            };
            render_mesh::visit_stage(&game.mission, &game.math, &mut compare);
            render_mesh::visit_inspection_characters(&game, &mut compare);
            render_mesh::visit_hinged(&game, &mut compare);
            assert!(checked > 10000);
            assert_eq!(crate::session::tests::state(&game), before);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
