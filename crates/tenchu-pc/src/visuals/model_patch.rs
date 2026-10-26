use super::models::Material;
use crate::render_mesh::Vertex;
use serde::{Deserialize, Serialize};
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
    pub corners: [Corner; 3],
}

impl Patch {
    pub fn validate(&self) -> Result<()> {
        for corner in &self.corners {
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
        for triangle in vertices.chunks_exact_mut(3) {
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
            let corners: Option<Vec<_>> = triangle
                .iter()
                .map(|v| self.corners.iter().find(|c| c.position == v.position))
                .collect();
            let Some(corners) = corners else { continue };
            for (vertex, corner) in triangle.iter_mut().zip(corners) {
                vertex.uv = [
                    corner.uv[0] * f32::from(region.rect[2]) + f32::from(region.rect[0]) - 0.5,
                    (1.0 - corner.uv[1]) * f32::from(region.rect[3]) + f32::from(region.rect[1])
                        - 0.5,
                ];
                vertex.color = corner.color;
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
            corners: [
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
            ],
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
            position: patch.corners[i].position,
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
}
