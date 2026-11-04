use super::{Mesh, Renderer, culling::Frustum};
use crate::render_mesh::Vertex;
use glam::{Mat4, Vec3};
use std::{collections::BTreeMap, sync::Arc};

pub(super) struct Decal {
    _source: Arc<Vec<Vertex>>,
    mesh: Mesh,
    bounds: [Vec3; 2],
    pub(super) ground_blood: bool,
}

impl Renderer {
    pub fn set_decals<'a>(
        &mut self,
        pages: impl IntoIterator<Item = &'a Arc<Vec<Vertex>>>,
    ) -> usize {
        let mut uploaded = 0;
        let mut previous = std::mem::take(&mut self.decals);
        self.decals = pages
            .into_iter()
            .filter(|page| !page.is_empty())
            .map(|page| {
                let key = Arc::as_ptr(page) as usize;
                let decal = previous.remove(&key).unwrap_or_else(|| {
                    uploaded += 1;
                    let bounds = page.iter().fold(
                        [Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)],
                        |[min, max], v| {
                            let p = Vec3::from(v.position);
                            [min.min(p), max.max(p)]
                        },
                    );
                    Decal {
                        _source: page.clone(),
                        mesh: self.upload(page),
                        bounds,
                        ground_blood: page.iter().all(|v| {
                            use crate::render_mesh::*;
                            let required = SCENERY | STAIN | SOFT | ALPHA;
                            v.flags
                                & (required
                                    | SCREEN
                                    | DEBUG
                                    | UI
                                    | EMITTER
                                    | SKY
                                    | WATER
                                    | FOOTPRINT
                                    | WET_CONTACT
                                    | UNLIT)
                                == required
                                && (v.flags & FOLIAGE != 0 || v.flags & WEAPON_COAT == 0)
                        }),
                    }
                });
                (key, decal)
            })
            .collect::<BTreeMap<_, _>>();
        uploaded
    }

    pub(super) fn snow_bounds(&self) -> bool {
        self.graphics.weather.snow_drift > 0
            && !self.graphics.weather.mode.rain()
            && self.weather.snow > 0.0
            && self.weather.wet + self.weather.puddles == 0.0
    }

    pub(super) fn draw_decals(&self, pass: &mut wgpu::RenderPass<'_>, matrix: Mat4) {
        let frustum = Frustum::new(matrix);
        let mut current = false;
        let bounds = self.snow_bounds();
        pass.set_pipeline(&self.pipelines.effects[5]);
        for decal in self
            .decals
            .values()
            .filter(|d| frustum.contains(d.bounds, 2.0))
        {
            let specialised = decal.ground_blood;
            #[cfg(test)]
            let specialised = specialised && !self.skip_blood_shader;
            if current != specialised {
                pass.set_pipeline(if specialised {
                    if bounds {
                        &self.pipelines.snow_blood
                    } else {
                        &self.pipelines.ground_blood
                    }
                } else {
                    &self.pipelines.effects[5]
                });
                current = specialised;
            }
            decal.mesh.bind(pass);
            pass.draw(0..decal.mesh.count, 0..1);
        }
    }
}
