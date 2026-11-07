#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniform {
    pub rays: [f32; 16],
    pub matrices: [[f32; 16]; 2],
    pub spans: [[f32; 4]; 2],
    pub eye: [f32; 4],
}

pub(super) struct Volume {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pipeline: Option<wgpu::ComputePipeline>,
    uniform: wgpu::Buffer,
    bindings: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    size: [u32; 3],
}

impl Volume {
    pub fn new(device: &wgpu::Device, gl: bool) -> Self {
        let size = if gl { [1; 3] } else { [128, 72, 64] };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Rain haze volume"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: size[2],
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | if gl {
                    wgpu::TextureUsages::empty()
                } else {
                    wgpu::TextureUsages::STORAGE_BINDING
                },
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Rain haze interpolation"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rain haze rays"),
            size: size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pipeline = (!gl).then(|| {
            #[cfg(target_os = "android")]
            crate::android::graphics_stage("Compiling rain haze compute pipeline");
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rain haze volume"),
                source: wgpu::ShaderSource::Wgsl(
                    concat!(
                        include_str!("../rain_ray.wgsl"),
                        "\n",
                        include_str!("rain_haze.wgsl")
                    )
                    .into(),
                ),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Rain haze volume"),
                layout: None,
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
        });
        Self {
            view,
            sampler,
            pipeline,
            uniform,
            bindings: None,
            size,
        }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        weather: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        data: &Uniform,
    ) {
        let Some(pipeline) = &self.pipeline else {
            return;
        };
        if self
            .bindings
            .as_ref()
            .is_none_or(|(source, _)| source != weather)
        {
            let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Rain haze preparation"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(weather),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.view),
                    },
                ],
            });
            self.bindings = Some((weather.clone(), bindings));
        }
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(data));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Rain haze volume"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bindings.as_ref().unwrap().1, &[]);
        pass.dispatch_workgroups(
            self.size[0].div_ceil(8),
            self.size[1].div_ceil(8),
            self.size[2],
        );
    }
}
