use super::{BOUNDS_TILE, Rect};

const LAYERS: usize = crate::lighting::MAX_SHADOW_LIGHTS * 6;
const UNIFORM_BYTES: u64 = (LAYERS * size_of::<Region>()) as u64;

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Region {
    area: [u32; 4],
    face: u32,
    padding: [u32; 3],
}

struct Level {
    binding: wgpu::BindGroup,
    size: u32,
    groups: [u32; 2],
}

struct Compute {
    pipelines: [wgpu::ComputePipeline; 2],
    levels: Vec<Level>,
    params: wgpu::Buffer,
    regions: Vec<Region>,
    changed: Vec<(usize, Rect)>,
}

pub(super) struct Bounds {
    pub view: wgpu::TextureView,
    compute: Option<Compute>,
    dirty: Vec<Option<Rect>>,
}

impl Bounds {
    pub fn new(
        device: &wgpu::Device,
        source: &wgpu::TextureView,
        resolution: u32,
        layers: usize,
        enabled: bool,
    ) -> Self {
        let tile = BOUNDS_TILE;
        let size = if enabled {
            resolution.div_ceil(tile)
        } else {
            1
        };
        let mip_count = size.ilog2() + 1;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Coarse point shadow bounds"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: super::super::array_layers(device, layers as u32),
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | if enabled {
                    wgpu::TextureUsages::STORAGE_BINDING
                } else {
                    wgpu::TextureUsages::empty()
                },
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        if !enabled {
            return Self {
                view,
                compute: None,
                dirty: vec![None; layers],
            };
        }
        let levels: Vec<_> = (0..mip_count)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Changed shadow regions"),
            size: UNIFORM_BYTES * u64::from(mip_count),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let stages = std::array::from_fn::<_, 2, _>(|stage| {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow bounds reduction"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: if stage == 0 {
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
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rg32Float,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                        },
                        count: None,
                    },
                ],
            });
            let code = if stage == 0 {
                include_str!("bounds_base.wgsl")
            } else {
                include_str!("bounds_reduce.wgsl")
            };
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shadow bounds reduction"),
                source: wgpu::ShaderSource::Wgsl(
                    code.replace("LAYERS", &LAYERS.to_string())
                        .replace("BASE_TILE", &format!("{tile}u"))
                        .into(),
                ),
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Shadow bounds reduction"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[&layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
            (layout, pipeline)
        });
        let bindings = (0..mip_count)
            .map(|mip| {
                let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Shadow bounds level"),
                    layout: &stages[usize::from(mip != 0)].0,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &params,
                                offset: UNIFORM_BYTES * u64::from(mip),
                                size: std::num::NonZeroU64::new(UNIFORM_BYTES),
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(if mip == 0 {
                                source
                            } else {
                                &levels[mip as usize - 1]
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&levels[mip as usize]),
                        },
                    ],
                });
                Level {
                    binding,
                    size: size >> mip,
                    groups: [0; 2],
                }
            })
            .collect();
        Self {
            view,
            compute: Some(Compute {
                pipelines: stages.map(|(_, p)| p),
                levels: bindings,
                params,
                regions: vec![Region::default(); LAYERS * mip_count as usize],
                changed: Vec::with_capacity(layers),
            }),
            dirty: vec![Some(Rect::full(resolution)); layers],
        }
    }

    pub fn enabled(&self) -> bool {
        self.compute.is_some()
    }

    pub fn invalidate(&mut self, face: usize, region: Rect) {
        if self.compute.is_some() {
            self.dirty[face] = Some(self.dirty[face].map_or(region, |old| old.union(region)));
        }
    }

    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        let Some(compute) = &mut self.compute else {
            return;
        };
        compute.changed.clear();
        compute.changed.extend(
            self.dirty
                .iter()
                .enumerate()
                .filter_map(|(face, rect)| rect.map(|rect| (face, rect))),
        );
        if compute.changed.is_empty() {
            return;
        }
        for (mip, level) in compute.levels.iter_mut().enumerate() {
            let scale = BOUNDS_TILE << mip;
            let mut extent = [0u32; 2];
            for (i, &(face, region)) in compute.changed.iter().enumerate() {
                let low = region.min.map(|v| v / scale);
                let high = region.max.map(|v| v.div_ceil(scale).min(level.size));
                compute.regions[mip * LAYERS + i] = Region {
                    area: [low[0], low[1], high[0], high[1]],
                    face: face as u32,
                    ..Default::default()
                };
                extent[0] = extent[0].max(high[0] - low[0]);
                extent[1] = extent[1].max(high[1] - low[1]);
            }
            let divisor = if mip == 0 { 1 } else { 8 };
            level.groups = extent.map(|v| v.div_ceil(divisor));
        }
        queue.write_buffer(&compute.params, 0, bytemuck::cast_slice(&compute.regions));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Coarse shadow depth reduction"),
            timestamp_writes: None,
        });
        for (mip, level) in compute.levels.iter().enumerate() {
            pass.set_pipeline(&compute.pipelines[usize::from(mip != 0)]);
            pass.set_bind_group(0, &level.binding, &[]);
            let [x, y] = level.groups;
            pass.dispatch_workgroups(x, y, compute.changed.len() as u32);
        }
        self.dirty.fill(None);
    }
}
