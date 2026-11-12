use super::{appearance::skin, models::rotation, motion};
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
    joints: [u32; 4],
    weights: [f32; 4],
}

pub(crate) struct Source {
    pub vertices: Vec<Rest>,
    bounds: Bounds,
    joints: Vec<(usize, Bounds)>,
}

impl Source {
    pub fn new(vertices: &[Vertex], rules: &[motion::BoundRule]) -> Self {
        Self {
            bounds: Bounds::collect(vertices.iter()),
            joints: Vec::new(),
            vertices: vertices
                .iter()
                .map(|v| Rest {
                    pose: Pose::from(v),
                    rules: rules.iter().take(4).enumerate().fold(0, |mask, (i, rule)| {
                        mask | (u32::from(rule.affects(v.material)) << i)
                    }),
                    padding: 0,
                    joints: [0; 4],
                    weights: [0.0; 4],
                })
                .collect(),
        }
    }

    pub fn skinned(
        vertices: &[Vertex],
        rules: &[motion::BoundRule],
        weights: &[skin::Influence],
        root: glam::Vec3,
    ) -> Self {
        let mut source = Self::new(vertices, rules);
        let mut bounds = [Bounds::default(); 64];
        for ((rest, vertex), weights) in source.vertices.iter_mut().zip(vertices).zip(weights) {
            rest.pose.position = (glam::Vec3::from(vertex.position) + root).to_array();
            rest.joints = weights.joints.map(u32::from);
            rest.weights = weights.weights;
            let vertex = Vertex {
                position: rest.pose.position,
                ..*vertex
            };
            let bound = Bounds::collect(std::iter::once(&vertex));
            for (joint, weight) in weights.joints.into_iter().zip(weights.weights) {
                if weight != 0.0 {
                    bounds[usize::from(joint)].merge(bound);
                }
            }
        }
        source.joints = bounds
            .into_iter()
            .enumerate()
            .filter(|(_, b)| b.shadow.is_some() || b.scenery.is_some())
            .collect();
        source
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
    pub fn skin(transform: skin::Transform) -> Self {
        Self {
            columns: std::array::from_fn(|i| {
                let column = if i < 3 {
                    glam::Vec3::from(transform.point.matrix3.col(i))
                } else {
                    transform.normal.col(i - 3)
                };
                column
                    .extend(if i < 3 {
                        transform.point.translation[i]
                    } else {
                        0.0
                    })
                    .to_array()
            }),
            rules: [[[0.0; 4]; 2]; 4],
            control: [0; 4],
        }
    }

    pub fn skin_at(&mut self, start: u32, count: u32) {
        self.control[2] = start;
        self.control[3] = count;
    }

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
    pub skin: Vec<skin::Transform>,
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
        if self.skin.is_empty() {
            self.source
                .bounds
                .transformed(rotation, translation, displacement)
        } else {
            let mut bounds = Bounds::default();
            for &(joint, source) in &self.source.joints {
                let transform = self.skin[joint].point;
                bounds.merge(source.transformed(
                    transform.matrix3,
                    transform.translation,
                    displacement,
                ));
            }
            bounds
        }
    }
}

pub(crate) struct Frame {
    pub generation: Arc<()>,
    pub parts: Vec<Part>,
    pub max_bytes: u64,
    max_jobs: u32,
    source_bytes: u64,
    jobs: u32,
    transforms: usize,
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
            transforms: 0,
        }
    }

    pub fn clear(&mut self, generation: Option<&Arc<()>>) {
        if let Some(generation) = generation {
            self.generation = generation.clone();
        }
        self.parts.clear();
        self.source_bytes = 0;
        self.jobs = 0;
        self.transforms = 0;
    }

    pub fn push(
        &mut self,
        source: Arc<Source>,
        matrix: Matrix,
        deformation: Option<&motion::Deformation<'_>>,
        start: usize,
        skin: Option<&[skin::Transform]>,
    ) -> bool {
        let count = source.vertices.len();
        let source_bytes = self.source_bytes + (count * size_of::<Rest>()) as u64;
        let jobs = self.jobs + count.div_ceil(WORKGROUP_SIZE) as u32;
        let transforms = self.transforms + 1 + skin.map_or(0, |s| s.len());
        if source_bytes > self.max_bytes
            || ((start + count) * size_of::<Pose>()) as u64 > self.max_bytes
            || (transforms * size_of::<Transform>()) as u64 > self.max_bytes
            || jobs > self.max_jobs
            || u64::from(jobs) * 16 > self.max_bytes
        {
            return false;
        }
        self.parts.push(Part {
            source,
            transform: Transform::new(matrix, deformation),
            start,
            skin: skin.map_or_else(Vec::new, <[skin::Transform]>::to_vec),
        });
        self.source_bytes = source_bytes;
        self.jobs = jobs;
        self.transforms = transforms;
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
