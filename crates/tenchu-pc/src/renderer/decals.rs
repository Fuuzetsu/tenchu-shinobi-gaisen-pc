use super::{Mesh, Renderer, culling::Frustum};
use crate::render_mesh::Vertex;
use crate::visuals::surfaces::SourceRange;
use glam::{Mat4, Vec3};
use std::{collections::HashMap, sync::Arc};
use wgpu::util::DeviceExt;

pub(super) mod cache;
pub(super) mod composite;

pub(super) struct Decal {
    _source: Arc<Vec<Vertex>>,
    mesh: Mesh,
    bounds: [Vec3; 2],
    pub(super) ground_blood: bool,
    source_ids: Option<wgpu::Buffer>,
    source_bounds: Vec<[Vec3; 2]>,
    sources: Vec<SourceRange>,
    constant_opacity: Vec<bool>,
}

impl Renderer {
    pub fn set_decals<'a>(
        &mut self,
        pages: impl IntoIterator<Item = &'a Arc<Vec<Vertex>>>,
    ) -> usize {
        self.set_projected_decals(pages.into_iter().map(|page| (page, &[][..])))
    }

    pub(crate) fn set_projected_decals<'a>(
        &mut self,
        pages: impl IntoIterator<Item = (&'a Arc<Vec<Vertex>>, &'a [SourceRange])>,
    ) -> usize {
        let mut uploaded = 0;
        let mut previous: HashMap<_, _> = std::mem::take(&mut self.decals)
            .into_iter()
            .map(|decal| (Arc::as_ptr(&decal._source) as usize, decal))
            .collect();
        self.decals = pages
            .into_iter()
            .filter(|(page, _)| !page.is_empty())
            .map(|(page, sources)| {
                let key = Arc::as_ptr(page) as usize;
                let mut decal = previous.remove(&key).unwrap_or_else(|| {
                    uploaded += 1;
                    let bounds = bounds(page);
                    Decal {
                        _source: page.clone(),
                        source_ids: None,
                        source_bounds: Vec::new(),
                        sources: sources.to_vec(),
                        constant_opacity: sources
                            .iter()
                            .map(|source| {
                                page.get(source.vertices.clone()).is_some_and(|vertices| {
                                    vertices.len() >= 3
                                        && vertices.iter().all(|v| {
                                            (v.opacity - vertices[0].opacity).abs() < 0.00001
                                        })
                                })
                            })
                            .collect(),
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
                if self.composite_supported && decal.ground_blood && decal.source_ids.is_none() {
                    let mut ids = vec![0_u32; page.len()];
                    for source in sources {
                        let next = self.decal_sources.len() + 1;
                        if next >= (1 << 22) {
                            break;
                        }
                        let Some(target) = ids.get_mut(source.vertices.clone()) else {
                            continue;
                        };
                        if target.is_empty() {
                            continue;
                        }
                        let id = *self
                            .decal_sources
                            .entry(source.source.clone())
                            .or_insert(next as u32);
                        target.fill(id);
                        decal
                            .source_bounds
                            .push(bounds(&page[source.vertices.clone()]));
                    }
                    decal.source_ids = Some(self.device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Blood receiver identities"),
                            contents: bytemuck::cast_slice(&ids),
                            usage: wgpu::BufferUsages::VERTEX,
                        },
                    ));
                }
                decal
            })
            .collect();
        if self.decals.is_empty() {
            self.decal_sources.clear();
            if let Some(composite) = &mut self.composite {
                composite.targets = None;
            }
        }
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
        if let Some(cache) = &self.blood_cache {
            cache.draw(pass, self.snow_bounds());
        }
        let mut current = false;
        let bounds = self.snow_bounds();
        pass.set_pipeline(&self.pipelines.effects[5]);
        for (index, decal) in self
            .decals
            .iter()
            .enumerate()
            .filter(|(_, d)| frustum.contains(d.bounds, 2.0))
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
            if let Some(cache) = &self.blood_cache {
                for range in &cache.fallback[index] {
                    pass.draw(range.clone(), 0..1);
                }
            } else {
                pass.draw(0..decal.mesh.count, 0..1);
            }
        }
    }
}

fn bounds(vertices: &[Vertex]) -> [Vec3; 2] {
    vertices.iter().fold(
        [Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)],
        |[min, max], vertex| {
            let point = Vec3::from(vertex.position);
            [min.min(point), max.max(point)]
        },
    )
}
