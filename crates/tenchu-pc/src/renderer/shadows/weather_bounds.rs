use super::Rect;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Transform {
    height: [f32; 4],
    layer: u32,
    padding: [u32; 3],
}

struct Level {
    bindings: wgpu::BindGroup,
    target: wgpu::TextureView,
    size: u32,
}

pub(super) struct Bounds {
    pub view: wgpu::TextureView,
    pipelines: [wgpu::RenderPipeline; 2],
    faces: Vec<Vec<Level>>,
    dirty: Vec<Option<Rect>>,
    transforms: Vec<wgpu::Buffer>,
}

impl Bounds {
    pub fn new(
        device: &wgpu::Device,
        resolution: u32,
        source: &wgpu::TextureView,
        gl: bool,
    ) -> Self {
        let format = wgpu::TextureFormat::Rg32Float;
        let size = resolution.div_ceil(4).max(1);
        let levels = size.ilog2() + 1;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Weather height bounds"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 2,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let setups = std::array::from_fn::<_, 2, _>(|stage| {
            let depth = stage == 0 && !gl;
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Weather bounds source"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: if depth {
                                wgpu::TextureSampleType::Depth
                            } else {
                                wgpu::TextureSampleType::Float { filterable: false }
                            },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Conservative weather bounds"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("weather_bounds.wgsl")
                        .replace(
                            "SOURCE",
                            if depth {
                                "@group(0) @binding(0) var source: texture_depth_2d_array;"
                            } else {
                                "@group(0) @binding(0) var source: texture_2d_array<f32>;"
                            },
                        )
                        .replace("STRIDE", if stage == 0 { "4" } else { "2" })
                        .replace(
                            "SAMPLE",
                            if depth {
                                "height_bounds(point, textureLoad(source, point, transform.layer, 0), size)"
                            } else if stage == 0 {
                                "height_bounds(point, textureLoad(source, point, transform.layer, 0).r, size)"
                            } else {
                                "textureLoad(source, point, transform.layer, 0).rg"
                            },
                        )
                        .into(),
                ),
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Weather height reduction"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[&layout],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });
            (layout, pipeline)
        });
        let transforms: Vec<_> = (0..2)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Weather height transform"),
                    size: std::mem::size_of::<Transform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let inputs: Vec<_> = (0..levels)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let faces = (0..2)
            .map(|face| {
                let mut passes: Vec<Level> = Vec::new();
                for mip in 0..levels {
                    let target = texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: face as u32,
                        array_layer_count: Some(1),
                        base_mip_level: mip,
                        mip_level_count: Some(1),
                        ..Default::default()
                    });
                    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Weather height reduction"),
                        layout: &setups[usize::from(mip != 0)].0,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(if mip == 0 {
                                    source
                                } else {
                                    &inputs[mip as usize - 1]
                                }),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: transforms[face].as_entire_binding(),
                            },
                        ],
                    });
                    passes.push(Level {
                        bindings,
                        target,
                        size: size >> mip,
                    });
                }
                passes
            })
            .collect();
        Self {
            transforms,
            view,
            pipelines: setups.map(|(_, pipeline)| pipeline),
            faces,
            dirty: vec![Some(Rect::full(resolution)); 2],
        }
    }

    pub fn invalidate(&mut self, face: usize, region: Rect) {
        self.dirty[face] = Some(self.dirty[face].map_or(region, |old| old.union(region)));
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        face: usize,
        height: [f32; 4],
    ) {
        let Some(region) = self.dirty[face].take() else {
            return;
        };
        queue.write_buffer(
            &self.transforms[face],
            0,
            bytemuck::bytes_of(&Transform {
                height,
                layer: face as u32,
                padding: [0; 3],
            }),
        );
        for (mip, level) in self.faces[face].iter().enumerate() {
            let scale = 4 << mip;
            let min = region.min.map(|v| (v / scale).min(level.size - 1));
            let max = region.max.map(|v| v.div_ceil(scale).min(level.size));
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Weather height bounds"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &level.target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipelines[usize::from(mip != 0)]);
            pass.set_bind_group(0, &level.bindings, &[]);
            pass.set_scissor_rect(min[0], min[1], max[0] - min[0], max[1] - min[1]);
            pass.draw(0..3, 0..1);
        }
    }
}
