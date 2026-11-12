use super::post;

#[derive(Clone, Copy)]
pub struct Request {
    pub scene: u32,
    pub tick: u64,
    pub additive: Option<bool>,
}

pub(super) struct Feedback {
    pub scene: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    history: [wgpu::TextureView; 2],
    combine: wgpu::RenderPipeline,
    present: wgpu::RenderPipeline,
    sources: [wgpu::BindGroup; 2],
    results: [wgpu::BindGroup; 2],
    weights: wgpu::Buffer,
    pub size: [u32; 2],
    index: usize,
    previous: Option<(u32, u64)>,
    valid: bool,
}

impl Feedback {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, size: [u32; 2]) -> Self {
        let scene = post::target(device, size, format);
        let depth = post::target(device, size, wgpu::TextureFormat::Depth32Float);
        let history = std::array::from_fn(|_| post::target(device, size, format));
        let weights = post::buffer(device, 16);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Original frame feedback"),
            entries: &[
                post::texture_entry(0, false, false),
                post::texture_entry(1, false, false),
                post::sampler_entry(2),
                post::uniform_entry(3),
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let binding = |current, previous| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Frame feedback sources"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(current),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(previous),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: weights.as_entire_binding(),
                    },
                ],
            })
        };
        let sources = std::array::from_fn(|i| binding(&scene, &history[1 - i]));
        let results = std::array::from_fn(|i| binding(&history[i], &scene));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original frame feedback"),
            source: wgpu::ShaderSource::Wgsl(include_str!("feedback.wgsl").into()),
        });
        Self {
            scene,
            depth,
            history,
            sources,
            results,
            weights,
            size,
            combine: post::pipeline(device, &layout, &shader, "combine", format, false),
            present: post::pipeline(device, &layout, &shader, "present", format, false),
            index: 0,
            previous: None,
            valid: false,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, request: Request) {
        let identity = (request.scene, request.tick);
        if self.previous != Some(identity) {
            self.valid = self.previous.is_some_and(|(scene, tick)| {
                scene == request.scene
                    && request
                        .tick
                        .checked_sub(tick)
                        .is_some_and(|elapsed| (1..9).contains(&elapsed))
            });
            self.index ^= 1;
            self.previous = Some(identity);
        }
        let weights: [f32; 4] = match request.additive.filter(|_| self.valid) {
            Some(false) => [0.5, 0.5, 0.0, 0.0],
            Some(true) => [1.0, 0.75, 0.0, 0.0],
            None => [1.0, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.weights, 0, bytemuck::cast_slice(&weights));
    }

    pub fn target(&self) -> &wgpu::TextureView {
        &self.history[self.index]
    }

    pub fn combine(&self, encoder: &mut wgpu::CommandEncoder) {
        self.draw(
            encoder,
            self.target(),
            &self.combine,
            &self.sources[self.index],
        );
    }

    pub fn present(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.draw(encoder, target, &self.present, &self.results[self.index]);
    }

    fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        sources: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Original frame feedback"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, sources, &[]);
        pass.draw(0..3, 0..1);
    }
}
