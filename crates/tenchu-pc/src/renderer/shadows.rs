use super::{EffectMesh, Mesh};
use crate::{
    lighting::{Light, MAX_LIGHTS, MAX_SHADOW_LIGHTS},
    render_mesh::Vertex,
};
use glam::{Mat4, Vec3};

pub const FACES: usize = MAX_SHADOW_LIGHTS * 6 + 1;
pub const NEAR: f32 = 20.0;
const LIGHT_WORDS: usize = MAX_LIGHTS * 8;

pub struct Geometry<'a> {
    pub meshes: &'a [&'a Mesh],
    pub effects: Option<&'a EffectMesh>,
    pub materials: &'a wgpu::BindGroup,
}

pub struct Shadows {
    pub layout: wgpu::BindGroupLayout,
    pub bindings: wgpu::BindGroup,
    data: wgpu::Buffer,
    sampler: wgpu::Sampler,
    views: Vec<wgpu::TextureView>,
    faces: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    pipeline: wgpu::RenderPipeline,
    resolution: u32,
    sun: wgpu::TextureView,
    sun_resolution: u32,
    weather: wgpu::TextureView,
    weather_layers: [wgpu::TextureView; 2],
    weather_resolution: u32,
}

impl Shadows {
    pub fn new(
        device: &wgpu::Device,
        vram: &wgpu::Texture,
        materials: &wgpu::BindGroupLayout,
    ) -> Self {
        let uniform = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Point lights and shadow maps"),
            entries: &[
                uniform,
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let data = buffer(device, (LIGHT_WORDS + FACES * 16 + 100) * 4);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow comparison filter"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sun = super::sun::target(device, 1);
        let (weather, weather_layers) = weather_targets(device, 1);
        let (bindings, views) = maps(device, &layout, &data, &sampler, 1, 1, [&sun, &weather]);
        let face_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow face"),
            entries: &[
                uniform,
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let faces = (0..FACES + 2)
            .map(|_| {
                let data = buffer(device, 80);
                let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Shadow face and original texture"),
                    layout: &face_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: data.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                &vram.create_view(&Default::default()),
                            ),
                        },
                    ],
                });
                (data, binding)
            })
            .collect();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Paletted cutout shadows"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "const MAX_MIPS: u32 = {}u;\n{}\n{}\n{}\n{}",
                    super::materials::MAX_MIPS,
                    include_str!("shadow.wgsl"),
                    include_str!("palette.wgsl"),
                    super::materials::shader(device).replace("@group(2)", "@group(1)"),
                    include_str!("foliage.wgsl")
                )
                .into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow face"),
            bind_group_layouts: &[&face_layout, materials],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Point light shadow caster"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vertex"), compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32, 4 => Uint32, 5 => Uint32, 6 => Uint32],
                }],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true, depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fragment"),
                compilation_options: Default::default(), targets: &[],
            }),
            multiview: None, cache: None,
        });
        Self {
            layout,
            bindings,
            data,
            sampler,
            views,
            faces,
            pipeline,
            resolution: 1,
            sun,
            sun_resolution: 1,
            weather,
            weather_layers,
            weather_resolution: 1,
        }
    }

    pub fn configure(&mut self, device: &wgpu::Device, frame: &super::sun::Frame, count: usize) {
        let settings = &frame.settings;
        let count = count
            .min(usize::from(settings.shadow_count))
            .min(MAX_SHADOW_LIGHTS);
        let active = settings.enabled && settings.shadows && count > 0;
        let layers = if active { count * 6 } else { 1 };
        let resolution = if active { settings.resolution } else { 1 };
        let sun_resolution =
            if settings.enabled && settings.shadows && settings.directional.intensity > 0 {
                settings.directional.resolution
            } else {
                1
            };
        let sun_changed = self.sun_resolution != sun_resolution;
        if sun_changed {
            self.sun = super::sun::target(device, sun_resolution);
            self.sun_resolution = sun_resolution;
        }
        let weather_changed = self.weather_resolution != frame.weather_resolution;
        if weather_changed {
            self.weather_resolution = frame.weather_resolution;
            (self.weather, self.weather_layers) = weather_targets(device, self.weather_resolution);
        }
        if self.resolution != resolution
            || self.views.len() != layers
            || sun_changed
            || weather_changed
        {
            (self.bindings, self.views) = maps(
                device,
                &self.layout,
                &self.data,
                &self.sampler,
                resolution,
                layers,
                [&self.sun, &self.weather],
            );
            self.resolution = resolution;
        }
    }

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        frame: &super::sun::Frame,
        camera: Mat4,
        lights: &[Light],
        geometry: &Geometry,
    ) {
        let settings = &frame.settings;
        let count = if settings.enabled {
            lights.len().min(MAX_LIGHTS)
        } else {
            0
        };
        let mut data = vec![0.0f32; LIGHT_WORDS + FACES * 16];
        for (i, light) in lights.iter().take(count).enumerate() {
            data[i * 8..i * 8 + 4].copy_from_slice(&light.position.extend(light.radius).to_array());
            data[i * 8 + 4..i * 8 + 7].copy_from_slice(&light.color);
            if !settings.shadows || i >= usize::from(settings.shadow_count).min(MAX_SHADOW_LIGHTS) {
                continue;
            }
            data[i * 8 + 7] = (i + 1) as f32;
            for (face, matrix) in matrices(light).iter().enumerate() {
                let index = i * 6 + face;
                data[LIGHT_WORDS + index * 16..LIGHT_WORDS + (index + 1) * 16]
                    .copy_from_slice(&matrix.to_cols_array());
                self.draw_face(
                    encoder,
                    queue,
                    index,
                    *matrix,
                    [
                        u8::from(settings.lamps) as f32,
                        frame.visual_time,
                        frame.wind,
                        0.0,
                    ],
                    geometry,
                );
            }
        }
        let sun_matrix = super::sun::matrix(&settings.directional, camera, frame.direction);
        let index = FACES - 1;
        data[LIGHT_WORDS + index * 16..LIGHT_WORDS + (index + 1) * 16]
            .copy_from_slice(&sun_matrix.to_cols_array());
        let sun_shadows =
            settings.enabled && settings.shadows && settings.directional.intensity > 0;
        if sun_shadows {
            self.draw_face(
                encoder,
                queue,
                index,
                sun_matrix,
                [
                    u8::from(settings.lamps) as f32,
                    frame.visual_time,
                    frame.wind,
                    0.0,
                ],
                geometry,
            );
        }
        data.extend([
            frame.ambient,
            f32::from(settings.intensity) / 100.0,
            count as f32,
            if !settings.enabled {
                0.0
            } else if settings.shadows {
                2.0
            } else {
                1.0
            },
        ]);
        data.extend([
            u8::from(settings.lamps) as f32,
            f32::from(settings.softness),
            frame.tint,
            0.0,
        ]);
        data.extend(sun_matrix.inverse().to_cols_array());
        data.extend(frame.direction.extend(frame.strength).to_array());
        data.extend(frame.color.to_array());
        data.push(u8::from(sun_shadows) as f32);
        let weather_matrices = weather_matrices(
            camera,
            self.weather_resolution,
            frame.precipitation,
            geometry.meshes,
        );
        for matrix in weather_matrices {
            data.extend(matrix.to_cols_array());
        }
        for matrix in weather_matrices {
            data.extend(matrix.inverse().to_cols_array());
        }
        data.extend([
            frame.weather.wet,
            frame.weather.snow,
            frame.weather.time,
            0.0,
        ]);
        if self.weather_resolution > 1 {
            for (layer, matrix) in weather_matrices.into_iter().enumerate() {
                self.draw_face(
                    encoder,
                    queue,
                    FACES + layer,
                    matrix,
                    [0.0, frame.visual_time, frame.wind, 1.0],
                    geometry,
                );
            }
        }
        queue.write_buffer(&self.data, 0, bytemuck::cast_slice(&data));
    }

    fn draw_face(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        index: usize,
        matrix: Mat4,
        options: [f32; 4],
        geometry: &Geometry,
    ) {
        let target = if index >= FACES {
            &self.weather_layers[index - FACES]
        } else if index == FACES - 1 {
            &self.sun
        } else {
            &self.views[index]
        };
        let (buffer, bindings) = &self.faces[index];
        let mut face = matrix.to_cols_array().to_vec();
        face.extend(options);
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&face));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shadow face"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bindings, &[]);
        pass.set_bind_group(1, geometry.materials, &[]);
        for mesh in geometry
            .meshes
            .iter()
            .copied()
            .chain(geometry.effects.map(|e| &e.mesh))
        {
            if mesh.count != 0 {
                pass.set_vertex_buffer(0, mesh.buffer.slice(..));
                pass.draw(0..mesh.count, 0..1);
            }
        }
    }
}

fn buffer(device: &wgpu::Device, size: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Lighting uniform"),
        size: size as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn maps(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    data: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    resolution: u32,
    layers: usize,
    sky_maps: [&wgpu::TextureView; 2],
) -> (wgpu::BindGroup, Vec<wgpu::TextureView>) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Point shadow depth array"),
        size: wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: super::array_layers(device, layers as u32),
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Lighting textures"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: data.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture.create_view(
                    &wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2Array),
                        ..Default::default()
                    },
                )),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(sky_maps[0]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(sky_maps[1]),
            },
        ],
    });
    let views = (0..layers)
        .map(|face| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Shadow face"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: face as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        })
        .collect();
    (bindings, views)
}

fn weather_targets(
    device: &wgpu::Device,
    resolution: u32,
) -> (wgpu::TextureView, [wgpu::TextureView; 2]) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Near and stage-wide precipitation shelter"),
        size: wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: 2,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let layers = std::array::from_fn(|index| {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: index as u32,
            array_layer_count: Some(1),
            ..Default::default()
        })
    });
    (
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        }),
        layers,
    )
}

fn weather_matrices(camera: Mat4, resolution: u32, direction: Vec3, meshes: &[&Mesh]) -> [Mat4; 2] {
    let inverse = camera.inverse();
    let near = inverse.project_point3(Vec3::ZERO);
    let forward = (inverse.project_point3(Vec3::Z) - near).normalize_or_zero();
    let view = Mat4::look_to_rh(Vec3::ZERO, direction, Vec3::Z);
    let focus = view.transform_point3(near + forward * 6000.0);
    let mut min = focus - Vec3::splat(20000.0);
    let mut max = focus + Vec3::splat(20000.0);
    for [a, b] in meshes.iter().filter_map(|mesh| mesh.bounds) {
        for x in [a.x, b.x] {
            for y in [a.y, b.y] {
                for z in [a.z, b.z] {
                    let p = view.transform_point3(Vec3::new(x, y, z));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
    }
    let far_center = (min + max) * 0.5;
    let far_radius = ((max.x - min.x).max(max.y - min.y) * 0.5 + 2000.0).max(20000.0);
    [(focus, 20000.0), (far_center, far_radius)].map(|(mut center, radius)| {
        let texel = radius * 2.0 / resolution as f32;
        center.x = (center.x / texel).floor() * texel;
        center.y = (center.y / texel).floor() * texel;
        Mat4::orthographic_rh(
            center.x - radius,
            center.x + radius,
            center.y - radius,
            center.y + radius,
            -max.z - 1000.0,
            -min.z + 1000.0,
        ) * view
    })
}

fn matrices(light: &Light) -> [Mat4; 6] {
    [
        (Vec3::X, Vec3::NEG_Y),
        (Vec3::NEG_X, Vec3::NEG_Y),
        (Vec3::Y, Vec3::Z),
        (Vec3::NEG_Y, Vec3::NEG_Z),
        (Vec3::Z, Vec3::NEG_Y),
        (Vec3::NEG_Z, Vec3::NEG_Y),
    ]
    .map(|(direction, up)| {
        Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, NEAR, light.radius)
            * Mat4::look_to_rh(light.position, direction, up)
    })
}
