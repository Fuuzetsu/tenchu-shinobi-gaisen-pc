use super::super::{Mesh, culling::Frustum};
use super::Rect;
use glam::Mat4;
use std::ops::Range;

#[derive(Default)]
pub(super) struct Culling {
    matrix: Mat4,
    wind: f32,
    size: u32,
    pub meshes: Vec<Culled>,
}

pub(super) struct Culled {
    buffer: wgpu::Buffer,
    revision: u64,
    dynamic: bool,
    pub ranges: Vec<Range<u32>>,
    pub foliage: bool,
    pub damage: Option<Rect>,
}

impl Culling {
    pub fn update<'a>(
        &mut self,
        matrix: Mat4,
        wind: f32,
        size: u32,
        meshes: impl Iterator<Item = &'a Mesh>,
    ) {
        let reusable = self.matrix == matrix && self.wind == wind && self.size == size;
        self.matrix = matrix;
        self.wind = wind;
        self.size = size;
        let frustum = Frustum::new(matrix);
        let mut count = 0;
        for (index, mesh) in meshes.enumerate() {
            count = index + 1;
            if reusable
                && self.meshes.get(index).is_some_and(|old| {
                    old.buffer == mesh.vertices.pose
                        && old.revision == mesh.revision
                        && old.dynamic == mesh.dynamic
                })
            {
                continue;
            }
            let (ranges, foliage, damage) = if mesh.count == 0 {
                (Vec::new(), false, None)
            } else if mesh.shadow_chunks.is_empty() {
                (
                    std::iter::once(0..mesh.count).collect(),
                    true,
                    Some(Rect::full(size)),
                )
            } else {
                let (ranges, foliage) = frustum.ranges(&mesh.shadow_chunks, wind);
                let damage = if ranges.is_empty() {
                    None
                } else {
                    frustum.damage(&mesh.shadow_chunks, wind, mesh.dynamic, matrix, size)
                };
                (ranges, foliage, damage)
            };
            let culled = Culled {
                buffer: mesh.vertices.pose.clone(),
                revision: mesh.revision,
                dynamic: mesh.dynamic,
                ranges,
                foliage,
                damage,
            };
            if let Some(previous) = self.meshes.get_mut(index) {
                *previous = culled;
            } else {
                self.meshes.push(culled);
            }
        }
        self.meshes.truncate(count);
    }
}
