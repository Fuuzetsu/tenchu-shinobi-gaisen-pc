use crate::settings::Graphics;
use glam::Mat4;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct Post {
    pub scene: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    levels: Vec<wgpu::TextureView>,
    bloom_bindings: Vec<wgpu::BindGroup>,
    extract: wgpu::RenderPipeline,
    downsample: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    data: wgpu::Buffer,
    bloom_data: wgpu::Buffer,
    size: [u32; 2],
}

impl Post {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: [u32; 2],
        samples: u32,
        depth: &wgpu::TextureView,
    ) -> Self {
        let scene = target(device, size, FORMAT);
        let overlay_depth = target(device, size, wgpu::TextureFormat::Depth32Float);
        let levels: Vec<_> = (1..=4)
            .map(|level| target(device, size.map(|n| (n >> level).max(1)), FORMAT))
            .collect();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Post-processing interpolation"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let data = buffer(device, 32 * 4);
        let bloom_data = buffer(device, 16);
        let bloom_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bloom source"),
            entries: &[
                texture_entry(0, false, false),
                sampler_entry(1),
                uniform_entry(2),
            ],
        });
        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bloom pyramid"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });
        let bloom_bindings = std::iter::once(&scene)
            .chain(levels.iter().take(3))
            .map(|source| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Bloom level"),
                    layout: &bloom_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(source),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: bloom_data.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();
        let extract = pipeline(
            device,
            &bloom_layout,
            &bloom_shader,
            "extract",
            FORMAT,
            false,
        );
        let downsample = pipeline(
            device,
            &bloom_layout,
            &bloom_shader,
            "downsample",
            FORMAT,
            false,
        );
        let mut entries: Vec<_> = (0..5).map(|i| texture_entry(i, false, false)).collect();
        entries.extend([
            sampler_entry(5),
            texture_entry(6, true, samples > 1),
            uniform_entry(7),
        ]);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene finishing"),
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HDR finishing and depth resolve"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "const SAMPLE_COUNT: i32 = {samples};\n{}",
                    include_str!("post.wgsl").replace(
                        "DEPTH_TYPE",
                        if samples > 1 {
                            "texture_depth_multisampled_2d"
                        } else {
                            "texture_depth_2d"
                        }
                    )
                )
                .into(),
            ),
        });
        let composite = pipeline(device, &layout, &shader, "fragment", format, true);
        let mut entries: Vec<_> = std::iter::once(&scene)
            .chain(&levels)
            .enumerate()
            .map(|(i, view)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        entries.extend([
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(depth),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: data.as_entire_binding(),
            },
        ]);
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene finishing"),
            layout: &layout,
            entries: &entries,
        });
        Self {
            scene,
            depth: overlay_depth,
            levels,
            bloom_bindings,
            extract,
            downsample,
            composite,
            bindings,
            data,
            bloom_data,
            size,
        }
    }

    pub fn prepare(&self, queue: &wgpu::Queue, camera: Mat4, graphics: &Graphics) {
        let settings = &graphics.post;
        let mut data = camera.inverse().to_cols_array().to_vec();
        data.extend([
            f32::from(graphics.exposure) / 100.0,
            f32::from(settings.contrast) / 100.0,
            f32::from(settings.saturation) / 100.0,
            f32::from(settings.temperature) / 100.0,
            f32::from(settings.bloom) / 100.0,
            f32::from(settings.occlusion) / 100.0,
            f32::from(settings.occlusion_radius) * 100.0,
            u8::from(settings.filmic) as f32,
            f32::from(settings.gamma) / 100.0,
            f32::from(settings.vignette) / 100.0,
            f32::from(settings.sharpen) / 100.0,
            0.0,
        ]);
        data.extend([self.size[0] as f32, self.size[1] as f32, 0.0, 0.0]);
        queue.write_buffer(&self.data, 0, bytemuck::cast_slice(&data));
        queue.write_buffer(
            &self.bloom_data,
            0,
            bytemuck::cast_slice(&[f32::from(settings.bloom_threshold) / 100.0, 0.0, 0.0, 0.0]),
        );
    }

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        bloom: bool,
    ) {
        if bloom {
            for (i, view) in self.levels.iter().enumerate() {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Bloom downsample"),
                    color_attachments: &[Some(attachment(view))],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(if i == 0 {
                    &self.extract
                } else {
                    &self.downsample
                });
                pass.set_bind_group(0, &self.bloom_bindings[i], &[]);
                pass.draw(0..3, 0..1);
            }
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Tone, atmosphere and contact shading"),
            color_attachments: &[Some(attachment(target))],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.composite);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn attachment(view: &wgpu::TextureView) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
    }
}

fn target(device: &wgpu::Device, size: [u32; 2], format: wgpu::TextureFormat) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Post-processing target"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Post-processing parameters"),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn texture_entry(binding: u32, depth: bool, multisampled: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: if depth {
                wgpu::TextureSampleType::Depth
            } else {
                wgpu::TextureSampleType::Float { filterable: true }
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    entry: &str,
    format: wgpu::TextureFormat,
    depth: bool,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Post-processing pass"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: depth.then_some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}
