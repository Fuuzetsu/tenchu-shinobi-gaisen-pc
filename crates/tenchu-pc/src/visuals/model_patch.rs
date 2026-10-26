use super::models::Material;
use crate::render_mesh::Vertex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::LazyLock};
use tenchu_assets::Result;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corner {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3],
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub texture: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corners: Option<[Corner; 3]>,
    #[serde(default)]
    pub translation: [f32; 3],
}

pub fn corrections() -> &'static BTreeMap<String, Vec<Patch>> {
    static CORRECTIONS: LazyLock<BTreeMap<String, Vec<Patch>>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../../data/model_corrections.json"))
            .expect("built-in model corrections")
    });
    &CORRECTIONS
}

impl Patch {
    pub fn validate(&self) -> Result<()> {
        if self
            .translation
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        {
            return Err("Invalid model patch translation".into());
        }
        for corner in self.corners.iter().flatten() {
            if corner
                .position
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
                || corner
                    .uv
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                || corner
                    .color
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=2.0).contains(v))
            {
                return Err("Invalid model patch position, UV or color".into());
            }
        }
        Ok(())
    }

    pub fn apply(&self, vertices: &mut [Vertex], materials: &[Material]) {
        for triangle in vertices.as_chunks_mut::<3>().0 {
            let Some(material) = triangle[0]
                .material
                .checked_sub(1)
                .and_then(|i| materials.get(i as usize))
            else {
                continue;
            };
            let Some(region) = material.region.filter(|_| material.id == self.texture) else {
                continue;
            };
            if let Some(corners) = &self.corners {
                let [Some(a), Some(b), Some(c)] = triangle
                    .each_ref()
                    .map(|v| corners.iter().find(|c| c.position == v.position))
                else {
                    continue;
                };
                for (vertex, corner) in triangle.iter_mut().zip([a, b, c]) {
                    vertex.uv = [
                        corner.uv[0] * f32::from(region.rect[2]) + f32::from(region.rect[0]) - 0.5,
                        (1.0 - corner.uv[1]) * f32::from(region.rect[3])
                            + f32::from(region.rect[1])
                            - 0.5,
                    ];
                    vertex.color = corner.color;
                }
            }
            for vertex in triangle {
                vertex.position = std::array::from_fn(|i| vertex.position[i] + self.translation[i]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visuals::{catalog::Region, pixels::Image};
    use bytemuck::Zeroable;
    use std::sync::Arc;

    #[test]
    fn patch_matches_the_complete_face_and_preserves_geometry() {
        let patch = Patch {
            texture: "skin".into(),
            translation: [0.0; 3],
            corners: Some([
                Corner {
                    position: [0.0, 0.0, 0.0],
                    uv: [0.5, 1.0],
                    color: [0.5; 3],
                },
                Corner {
                    position: [10.0, 0.0, 0.0],
                    uv: [1.0, 0.0],
                    color: [0.6; 3],
                },
                Corner {
                    position: [0.0, 10.0, 0.0],
                    uv: [0.0, 0.0],
                    color: [0.7; 3],
                },
            ]),
        };
        let materials = [Material {
            id: "skin".into(),
            region: Some(Region {
                page: 0,
                palette: 0,
                rect: [64, 32, 32, 16],
            }),
            image: Arc::new(Image {
                size: [1, 1],
                rgba: vec![255; 4],
            }),
        }];
        let mut vertices = [2, 0, 1].map(|i| Vertex {
            position: patch.corners.as_ref().unwrap()[i].position,
            material: 1,
            flags: 15,
            normal: [0.0, 0.0, 1.0],
            ..Vertex::zeroed()
        });
        let original = vertices;
        patch.apply(&mut vertices, &materials);
        assert_eq!(vertices[0].uv, [63.5, 47.5]);
        assert_eq!(vertices[1].uv, [79.5, 31.5]);
        assert_eq!(vertices[2].uv, [95.5, 47.5]);
        for (vertex, before) in vertices.iter().zip(original) {
            assert_eq!(vertex.position, before.position);
            assert_eq!(vertex.normal, before.normal);
            assert_eq!(vertex.flags, before.flags);
        }
        vertices = original;
        vertices[0].position[0] += 1.0;
        patch.apply(&mut vertices, &materials);
        assert!(vertices.iter().all(|v| v.uv == [0.0; 2]));
        vertices = original;
        let mut different = materials;
        different[0].id = "cloth".into();
        patch.apply(&mut vertices, &different);
        assert!(vertices.iter().all(|v| v.uv == [0.0; 2]));
    }

    #[test]
    fn material_translation_keeps_other_surfaces_and_attributes() {
        let mut patch: Patch =
            serde_json::from_str(r#"{"texture":"screen","translation":[0,0,256]}"#).unwrap();
        patch.validate().unwrap();
        let materials: Vec<_> = ["screen", "robe"]
            .into_iter()
            .map(|id| Material {
                id: id.into(),
                region: Some(Region {
                    page: 0,
                    palette: 0,
                    rect: [0, 0, 32, 32],
                }),
                image: Arc::new(Image {
                    size: [1, 1],
                    rgba: vec![255; 4],
                }),
            })
            .collect();
        let mut vertices = [1, 2]
            .into_iter()
            .flat_map(|material| {
                [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]].map(|position| Vertex {
                    position,
                    material,
                    uv: [7.5, 12.5],
                    color: [0.5; 3],
                    flags: 19,
                    ..Vertex::zeroed()
                })
            })
            .collect::<Vec<_>>();
        let mut expected = vertices.clone();
        for vertex in &mut expected[..3] {
            vertex.position[2] += 256.0;
        }
        patch.apply(&mut vertices, &materials);
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&vertices),
            bytemuck::cast_slice::<_, u8>(&expected)
        );
        patch.translation[2] = f32::NAN;
        assert!(patch.validate().is_err());
        for patch in corrections().values().flatten() {
            patch.validate().unwrap();
        }
    }
}
