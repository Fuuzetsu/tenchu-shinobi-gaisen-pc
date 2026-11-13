use super::*;
use tenchu_assets::model::{Geometry, Part};

impl Model {
    pub(crate) fn build(&self, assets: &Assets, id: &str) -> Result<(Part, Arc<pack::Pack>)> {
        self.validate(assets)?;
        let source = get(assets, &self.asset, Kind::Model)?.model()?;
        let mut model = Vec::with_capacity(source.len());
        let mut pack = pack::Pack::default();
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for triangle in source {
            let mut corners = triangle.corners;
            for corner in &mut corners {
                for axis in 0..3 {
                    corner.position[axis] *= self.scale as f32;
                    if self.y_up && axis == 1 {
                        corner.position[axis] = -corner.position[axis];
                        corner.normal[axis] = -corner.normal[axis];
                    }
                    if corner.position[axis].abs() > 32_000.0 {
                        return Err("Model extends beyond 32 metres from its origin; reduce its scale or split the model".into());
                    }
                    min[axis] = min[axis].min(corner.position[axis]);
                    max[axis] = max[axis].max(corner.position[axis]);
                }
            }
            if self.y_up {
                corners.swap(1, 2);
            }
            model.push(obj::Triangle {
                corners,
                material: triangle.material.clone(),
            });
            let texture = self.materials.get(&triangle.material).cloned().flatten();
            if let Some(id) = &texture {
                pack.images
                    .insert(id.clone(), get(assets, id, Kind::Texture)?.image()?);
            }
            pack.materials.insert(
                triangle.material.clone(),
                pack::Material {
                    texture,
                    blend: 0,
                    double_sided: true,
                },
            );
        }
        pack.models.insert(id.into(), model);
        let vertices = (0..8)
            .map(|i| {
                std::array::from_fn(|a| {
                    if i & (1 << a) == 0 {
                        min[a].floor() as i16
                    } else {
                        max[a].ceil() as i16
                    }
                })
            })
            .collect();
        let part = Part {
            parent: -1,
            child: -1,
            position: [0; 3],
            geometry: vec![Geometry {
                vertices,
                joints: vec![],
                normals: vec![],
                faces: vec![],
            }],
        };
        Ok((part, Arc::new(pack)))
    }
}
