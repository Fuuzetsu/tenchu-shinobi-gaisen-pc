use super::{models::rotation, motion};
use crate::render_mesh::{Bounds, Pose, Vertex};
use bytemuck::{Pod, Zeroable};
use glam::{Mat3A, Vec3A};
use std::sync::Arc;
use tenchu_core::math::Matrix;

pub(crate) const WORKGROUP_SIZE: usize = 128;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Rest {
    pose: Pose,
    rules: u32,
    padding: u32,
}

pub(crate) struct Source {
    pub vertices: Vec<Rest>,
    bounds: Bounds,
}

impl Source {
    pub fn new(vertices: &[Vertex], rules: &[motion::BoundRule]) -> Self {
        Self {
            bounds: Bounds::collect(vertices.iter()),
            vertices: vertices
                .iter()
                .map(|v| Rest {
                    pose: Pose::from(v),
                    rules: rules.iter().take(4).enumerate().fold(0, |mask, (i, rule)| {
                        mask | (u32::from(rule.affects(v.material)) << i)
                    }),
                    padding: 0,
                })
                .collect(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Transform {
    columns: [[f32; 4]; 6],
    rules: [[[f32; 4]; 2]; 4],
    control: [u32; 4],
}

impl Transform {
    fn new(matrix: Matrix, deformation: Option<&motion::Deformation<'_>>) -> Self {
        let rotation = rotation(matrix);
        let normal = rotation.inverse().transpose();
        let translation = matrix.translation.to_array();
        let mut out = Self {
            columns: std::array::from_fn(|i| {
                let column = if i < 3 {
                    rotation.col(i)
                } else {
                    normal.col(i - 3)
                };
                column
                    .extend(if i < 3 { translation[i] as f32 } else { 0.0 })
                    .to_array()
            }),
            rules: [[[0.0; 4]; 2]; 4],
            control: [0; 4],
        };
        if let Some(deformation) = deformation {
            for (i, (bound, offset)) in deformation
                .rules
                .iter()
                .zip(&deformation.pose.offsets)
                .enumerate()
            {
                let rule = &bound.rule;
                out.rules[i] = [
                    [rule.axis[0], rule.axis[1], rule.axis[2], rule.start],
                    [offset.x, offset.y, offset.z, rule.end],
                ];
                out.control[0] |= u32::from(rule.absolute) << i;
                out.control[1] += 1;
            }
        }
        out
    }
}

pub(crate) struct Part {
    pub source: Arc<Source>,
    pub transform: Transform,
    pub start: usize,
}

impl Part {
    pub fn end(&self) -> usize {
        self.start + self.source.vertices.len()
    }

    fn bounds(&self) -> Bounds {
        let c = &self.transform.columns;
        let rotation = Mat3A::from_cols(
            Vec3A::new(c[0][0], c[0][1], c[0][2]),
            Vec3A::new(c[1][0], c[1][1], c[1][2]),
            Vec3A::new(c[2][0], c[2][1], c[2][2]),
        );
        let translation = Vec3A::new(c[0][3], c[1][3], c[2][3]);
        let mut displacement = [Vec3A::ZERO; 2];
        for rule in &self.transform.rules[..self.transform.control[1] as usize] {
            let delta = Vec3A::new(rule[1][0], rule[1][1], rule[1][2]);
            displacement[0] += delta.min(Vec3A::ZERO);
            displacement[1] += delta.max(Vec3A::ZERO);
        }
        self.source
            .bounds
            .transformed(rotation, translation, displacement)
    }
}

pub(crate) struct Frame {
    pub generation: Arc<()>,
    pub parts: Vec<Part>,
    pub max_bytes: u64,
    max_jobs: u32,
    source_bytes: u64,
    jobs: u32,
}

impl Frame {
    pub fn new(max_bytes: u64, max_jobs: u32) -> Self {
        Self {
            generation: Arc::default(),
            parts: Vec::new(),
            max_bytes,
            max_jobs,
            source_bytes: 0,
            jobs: 0,
        }
    }

    pub fn clear(&mut self, generation: Option<&Arc<()>>) {
        if let Some(generation) = generation {
            self.generation = generation.clone();
        }
        self.parts.clear();
        self.source_bytes = 0;
        self.jobs = 0;
    }

    pub fn push(
        &mut self,
        source: Arc<Source>,
        matrix: Matrix,
        deformation: Option<&motion::Deformation<'_>>,
        start: usize,
    ) -> bool {
        let count = source.vertices.len();
        let source_bytes = self.source_bytes + (count * size_of::<Rest>()) as u64;
        let jobs = self.jobs + count.div_ceil(WORKGROUP_SIZE) as u32;
        if source_bytes > self.max_bytes
            || ((start + count) * size_of::<Pose>()) as u64 > self.max_bytes
            || ((self.parts.len() + 1) * size_of::<Transform>()) as u64 > self.max_bytes
            || jobs > self.max_jobs
            || u64::from(jobs) * 16 > self.max_bytes
        {
            return false;
        }
        self.parts.push(Part {
            source,
            transform: Transform::new(matrix, deformation),
            start,
        });
        self.source_bytes = source_bytes;
        self.jobs = jobs;
        true
    }

    pub fn bounds(&self, vertices: &[Vertex], range: std::ops::Range<usize>) -> Bounds {
        let first = self.parts.partition_point(|part| part.end() <= range.start);
        let mut start = range.start;
        let mut bounds = Bounds::default();
        for part in self.parts[first..]
            .iter()
            .take_while(|p| p.start < range.end)
        {
            let end = part.start.max(range.start);
            bounds.merge(Bounds::collect(vertices[start..end].iter()));
            bounds.merge(part.bounds());
            start = part.end().min(range.end);
        }
        bounds.merge(Bounds::collect(vertices[start..range.end].iter()));
        bounds
    }
}
