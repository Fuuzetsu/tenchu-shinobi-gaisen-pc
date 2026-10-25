use crate::render_mesh::Vertex;
use crate::visuals::{models::Material, vegetation::Kind};
use glam::Vec3;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub struct Canopy {
    pub min: Vec3,
    pub max: Vec3,
    pub color: [f32; 3],
    pub blossom: bool,
}
impl Canopy {
    pub fn local(
        vertices: &[Vertex],
        materials: &[Material],
        rules: &BTreeMap<String, Kind>,
    ) -> Vec<Self> {
        [Kind::Blossoms, Kind::Autumn]
            .into_iter()
            .filter_map(|kind| {
                let mut min = Vec3::splat(f32::INFINITY);
                let mut max = Vec3::splat(f32::NEG_INFINITY);
                let mut color = Vec3::ZERO;
                let mut count: f32 = 0.0;
                for (i, m) in materials
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| rules.get(&m.id) == Some(&kind))
                {
                    let mut used = false;
                    for v in vertices.iter().filter(|v| v.material == i as u32 + 1) {
                        min = min.min(v.position.into());
                        max = max.max(v.position.into());
                        used = true;
                    }
                    if used {
                        for p in m
                            .image
                            .rgba
                            .chunks_exact(4)
                            .filter(|p| p[3] > 200 && p[0] > p[1] && p[0] > 30)
                        {
                            color += Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
                            count += 1.0;
                        }
                    }
                }
                if !min.is_finite() {
                    return None;
                }
                color /= count.max(1.0);
                color /= color.max_element().max(1.0);
                let blossom = kind == Kind::Blossoms;
                if blossom {
                    color = color.lerp(Vec3::ONE, 0.4);
                }
                Some(Self {
                    min,
                    max,
                    color: (color * 0.85).to_array(),
                    blossom,
                })
            })
            .collect()
    }
    pub fn transformed(self, mut transform: impl FnMut(Vec3) -> Vec3) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for x in [self.min.x, self.max.x] {
            for y in [self.min.y, self.max.y] {
                for z in [self.min.z, self.max.z] {
                    let p = transform(Vec3::new(x, y, z));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
        Self { min, max, ..self }
    }
}
