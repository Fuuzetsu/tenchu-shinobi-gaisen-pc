use super::{Mesh, Renderer, culling::Frustum};
use crate::render_mesh::Vertex;
use glam::{Mat4, Vec3};
use std::{collections::BTreeMap, sync::Arc};

pub(super) struct Decal {
    _source: Arc<Vec<Vertex>>,
    mesh: Mesh,
    bounds: [Vec3; 2],
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
                    }
                });
                (key, decal)
            })
            .collect::<BTreeMap<_, _>>();
        uploaded
    }

    pub(super) fn draw_decals(&self, pass: &mut wgpu::RenderPass<'_>, matrix: Mat4) {
        let frustum = Frustum::new(matrix);
        pass.set_pipeline(&self.effects[5]);
        for decal in self
            .decals
            .values()
            .filter(|d| frustum.contains(d.bounds, 2.0))
        {
            pass.set_vertex_buffer(0, decal.mesh.buffer.slice(..));
            pass.draw(0..decal.mesh.count, 0..1);
        }
    }
}
