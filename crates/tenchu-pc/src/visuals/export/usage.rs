use super::catalog::{self, Region};
use glam::Vec3;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use tenchu_assets::{Result, stage::MissionAssets};

#[derive(Default, Serialize)]
pub(super) struct Usage {
    triangles: usize,
    blends: BTreeSet<u8>,
}

#[derive(Serialize)]
pub(super) struct Detail {
    size: [u16; 2],
    alpha: BTreeMap<u8, usize>,
    scrolling: bool,
    models: BTreeMap<String, Usage>,
    sprites: BTreeSet<usize>,
    terrain_attributes: BTreeSet<u16>,
    map_samples: Vec<[i32; 3]>,
}

pub(super) fn collect(
    mission: &MissionAssets,
    regions: &BTreeMap<Region, String>,
) -> Result<BTreeMap<String, Detail>> {
    let mut details = BTreeMap::new();
    for (region, id) in regions {
        details.entry(id.clone()).or_insert_with(|| {
            let mut alpha = BTreeMap::new();
            for p in region.pixels(&mission.vram).chunks_exact(4) {
                *alpha.entry(p[3]).or_default() += 1;
            }
            Detail {
                size: [region.rect[2], region.rect[3]],
                alpha,
                scrolling: region.scroll_offset(&mission.vram).is_some(),
                models: BTreeMap::new(),
                sprites: BTreeSet::new(),
                terrain_attributes: BTreeSet::new(),
                map_samples: Vec::new(),
            }
        });
    }
    for (index, &image) in mission.images.iter().enumerate() {
        let region = Region::sprite(&mission.vram, image);
        details
            .get_mut(&regions[&region])
            .unwrap()
            .sprites
            .insert(index);
    }
    let mut terrain = Vec::new();
    for row in &mission.terrain.rows {
        for i in 0..row.count {
            terrain.push(mission.terrain.node(row.list + i * 16)?);
        }
    }
    let prefix = format!("stages/{}/map/", mission.stage.directory);
    for model in catalog::models(mission) {
        let translation = model
            .id
            .strip_prefix(&prefix)
            .and_then(|i| i.parse::<usize>().ok())
            .and_then(|i| mission.map.get(i))
            .map(|part| Vec3::from(part.position.map(|v| f32::from(v) * 10.0)));
        for geometry in model.geometry {
            for face in crate::render_mesh::draw_tmd_faces(geometry) {
                let Some(region) = catalog::face_region(&mission.vram, face) else {
                    continue;
                };
                let [a, b, c] = std::array::from_fn(|i| {
                    Vec3::from(geometry.vertices[face.corners[i].vertex as usize].map(f32::from))
                });
                let normal = (b - a).cross(c - a).normalize_or_zero();
                if normal == Vec3::ZERO {
                    continue;
                }
                let detail = details.get_mut(&regions[&region]).unwrap();
                let usage = detail.models.entry(model.id.clone()).or_default();
                usage.triangles += face.corners.len() - 2;
                usage.blends.insert(if face.translucent {
                    ((face.texture.unwrap().page >> 5) & 3) as u8 + 1
                } else {
                    0
                });
                let Some(translation) = translation else {
                    continue;
                };
                let center = ((a + b + c) / 3.0 + translation)
                    .to_array()
                    .map(|n| n.round() as i32);
                if detail.map_samples.len() < 12
                    && detail.map_samples.iter().all(|p| {
                        Vec3::from(p.map(|v| v as f32))
                            .distance_squared(Vec3::from(center.map(|v| v as f32)))
                            > 25_000_000.0
                    })
                {
                    detail.map_samples.push(center);
                }
                if normal.y.abs() < 0.95 {
                    continue;
                }
                for node in &terrain {
                    if node.contains(center[0] / 10, center[2] / 10) {
                        let level = tenchu_core::terrain::node_level(
                            *node,
                            center[0] / 10,
                            center[2] / 10,
                        )?;
                        if level != tenchu_core::terrain::NONE
                            && (level * 10 - center[1]).abs() <= 20
                        {
                            detail.terrain_attributes.insert(node.attributes);
                        }
                    }
                }
            }
        }
    }
    Ok(details)
}
