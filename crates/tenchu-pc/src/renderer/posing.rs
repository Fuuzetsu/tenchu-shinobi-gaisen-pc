use super::geometry;
use crate::visuals::posing::{Frame, Rest, Transform, WORKGROUP_SIZE};
use std::{cell::Cell, collections::HashMap, sync::Arc};

pub(super) struct Posing {
    pipeline: wgpu::ComputePipeline,
    generation: Arc<()>,
    sources: HashMap<usize, u32>,
    rest: Vec<Rest>,
    transforms: Vec<Transform>,
    jobs: Vec<[u32; 4]>,
    buffers: [wgpu::Buffer; 3],
    binding: Option<([wgpu::Buffer; 4], u64, wgpu::BindGroup)>,
    dirty: Cell<bool>,
}

impl Posing {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Character posing"),
            source: wgpu::ShaderSource::Wgsl(include_str!("posing.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Character posing"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            pipeline,
            generation: Arc::default(),
            sources: HashMap::new(),
            rest: Vec::new(),
            transforms: Vec::new(),
            jobs: Vec::new(),
            buffers: ["Rest poses", "Part transforms", "Pose jobs"].map(|label| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: 0,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            }),
            binding: None,
            dirty: Cell::new(false),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Frame,
        output: &wgpu::Buffer,
    ) {
        let possible_growth: usize = frame
            .parts
            .iter()
            .filter(|part| {
                !self
                    .sources
                    .contains_key(&(Arc::as_ptr(&part.source) as usize))
            })
            .map(|part| part.source.vertices.len())
            .sum();
        if !Arc::ptr_eq(&self.generation, &frame.generation)
            || ((self.rest.len() + possible_growth) * size_of::<Rest>()) as u64 > frame.max_bytes
        {
            self.sources.clear();
            self.rest.clear();
            self.generation = frame.generation.clone();
        }
        let previous = self.rest.len();
        self.transforms.clear();
        self.jobs.clear();
        for part in &frame.parts {
            let key = Arc::as_ptr(&part.source) as usize;
            let start = *self.sources.entry(key).or_insert_with(|| {
                let start = self.rest.len() as u32;
                self.rest.extend_from_slice(&part.source.vertices);
                start
            });
            let transform = self.transforms.len() as u32;
            self.transforms.push(part.transform);
            for offset in (0..part.source.vertices.len()).step_by(WORKGROUP_SIZE) {
                self.jobs.push([
                    start + offset as u32,
                    (part.start + offset) as u32,
                    (part.source.vertices.len() - offset).min(WORKGROUP_SIZE) as u32,
                    transform,
                ]);
            }
        }
        if previous != self.rest.len() {
            geometry::update_buffer(
                device,
                queue,
                &mut self.buffers[0],
                bytemuck::cast_slice(&self.rest),
                wgpu::BufferUsages::STORAGE,
            );
        }
        geometry::update_buffer(
            device,
            queue,
            &mut self.buffers[1],
            bytemuck::cast_slice(&self.transforms),
            wgpu::BufferUsages::STORAGE,
        );
        geometry::update_buffer(
            device,
            queue,
            &mut self.buffers[2],
            bytemuck::cast_slice(&self.jobs),
            wgpu::BufferUsages::STORAGE,
        );
        let buffers = [
            self.buffers[0].clone(),
            self.buffers[1].clone(),
            self.buffers[2].clone(),
            output.clone(),
        ];
        if self
            .binding
            .as_ref()
            .is_none_or(|(bound, size, _)| *bound != buffers || *size != frame.max_bytes)
        {
            let entries = std::array::from_fn::<_, 4, _>(|i| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffers[i],
                    offset: 0,
                    size: wgpu::BufferSize::new(buffers[i].size().min(frame.max_bytes)),
                }),
            });
            let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Character pose buffers"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &entries,
            });
            self.binding = Some((buffers, frame.max_bytes, binding));
        }
        self.dirty.set(true);
    }

    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder) {
        if !self.dirty.replace(false) {
            return;
        }
        let Some((_, _, binding)) = &self.binding else {
            return;
        };
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Character posing"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, binding, &[]);
        pass.dispatch_workgroups(self.jobs.len() as u32, 1, 1);
    }
}
