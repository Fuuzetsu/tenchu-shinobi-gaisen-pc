use super::culling::Rect;
use super::{EffectMesh, Mesh};
use crate::lighting::{Light, MAX_LIGHTS, MAX_SCENE_LIGHTS, MAX_SHADOW_LIGHTS};
use glam::{Mat4, Vec3};

pub const FACES: usize = MAX_SHADOW_LIGHTS * 6 + 1;
pub const NEAR: f32 = 20.0;
pub const FILTER_SAMPLES: usize = 32;
const LIGHT_WORDS: usize = MAX_SCENE_LIGHTS * 8;

pub struct Geometry<'a> {
    pub meshes: &'a [&'a Mesh],
    pub effects: Option<&'a EffectMesh>,
    pub materials: &'a wgpu::BindGroup,
}

#[derive(PartialEq)]
struct FaceState {
    matrix: Mat4,
    options: [f32; 12],
    meshes: Vec<(wgpu::Buffer, u64)>,
    static_meshes: Vec<(wgpu::Buffer, u64)>,
    damage: Option<Rect>,
}

pub struct Shadows {
    pub layout: wgpu::BindGroupLayout,
    pub bindings: wgpu::BindGroup,
    data: wgpu::Buffer,
    filter: [[f32; 4]; FILTER_SAMPLES],
    sampler: wgpu::Sampler,
    views: Vec<wgpu::TextureView>,
    faces: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    pipeline: wgpu::RenderPipeline,
    weather_pipeline: wgpu::RenderPipeline,
    clear_pipeline: wgpu::RenderPipeline,
    resolution: u32,
    sun: wgpu::TextureView,
    sun_resolution: u32,
    weather: wgpu::TextureView,
    weather_layers: [wgpu::TextureView; 2],
    weather_resolution: u32,
    selection: crate::lighting::shadows::Selection,
    grid: super::light_grid::Grid,
    readable: ReadableDepth,
    gl: bool,
    cached: [Option<FaceState>; FACES + 2],
    #[cfg(test)]
    pub(super) rendered_faces: usize,
    #[cfg(test)]
    pub(super) rendered_area: u64,
    #[cfg(test)]
    pub(super) draw_all_faces: bool,
}

impl Shadows {
    pub fn new(
        device: &wgpu::Device,
        vram: &wgpu::Texture,
        materials: &wgpu::BindGroupLayout,
        gl: bool,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
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
        let data = buffer(
            device,
            (LIGHT_WORDS + FACES * 16 + 128 + FILTER_SAMPLES * 4) * 4,
        );
        let grid = super::light_grid::Grid::new(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow comparison filter"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sun = super::sun::target(device, 1);
        let (weather, weather_layers) = weather_targets(device, 1);
        let (bindings, views, readable) = maps(
            device,
            &layout,
            &data,
            &sampler,
            1,
            1,
            MapTargets {
                views: [&sun, &weather],
                grid: &grid.view,
                sizes: [1, 1],
                gl,
            },
        );
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
                let data = buffer(device, 112);
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
        let pipeline = |label, bias| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &super::geometry::LAYOUTS,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias,
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if gl { "readable_depth" } else { "fragment" }),
                    compilation_options: Default::default(),
                    targets: if gl {
                        &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::R32Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::RED,
                        })]
                    } else {
                        &[]
                    },
                }),
                multiview: None,
                cache: None,
            })
        };
        let weather_pipeline = pipeline("Precipitation shelter", wgpu::DepthBiasState::default());
        let clear_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Clear changed shadow region"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("clear_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: gl.then_some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("clear_depth"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::RED,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let pipeline = pipeline(
            "Point light shadow caster",
            wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        );
        Self {
            layout,
            bindings,
            data,
            filter: std::array::from_fn(|i| {
                let angle = i as f32 * 2.3999632;
                [
                    angle.cos(),
                    angle.sin(),
                    ((i as f32 + 0.5) / FILTER_SAMPLES as f32).sqrt(),
                    0.0,
                ]
            }),
            sampler,
            views,
            faces,
            pipeline,
            weather_pipeline,
            clear_pipeline,
            resolution: 1,
            sun,
            sun_resolution: 1,
            weather,
            weather_layers,
            weather_resolution: 1,
            selection: Default::default(),
            grid,
            readable,
            gl,
            cached: std::array::from_fn(|_| None),
            #[cfg(test)]
            rendered_faces: 0,
            #[cfg(test)]
            rendered_area: 0,
            #[cfg(test)]
            draw_all_faces: false,
        }
    }

    pub fn invalidate(&mut self) {
        self.cached.fill_with(|| None);
    }

    pub fn configure(&mut self, device: &wgpu::Device, frame: &super::sun::Frame, count: usize) {
        let settings = &frame.settings;
        let count = count
            .min(usize::from(settings.shadow_count))
            .min(MAX_SHADOW_LIGHTS);
        let active = settings.enabled && settings.count > 0 && settings.shadows && count > 0;
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
            (self.bindings, self.views, self.readable) = maps(
                device,
                &self.layout,
                &self.data,
                &self.sampler,
                resolution,
                layers,
                MapTargets {
                    views: [&self.sun, &self.weather],
                    grid: &self.grid.view,
                    sizes: [self.sun_resolution, self.weather_resolution],
                    gl: self.gl,
                },
            );
            self.resolution = resolution;
            self.invalidate();
        }
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        frame: &super::sun::Frame,
        camera: Mat4,
        lights: &[Light],
        geometry: &Geometry,
    ) {
        #[cfg(test)]
        {
            self.rendered_faces = 0;
            self.rendered_area = 0;
            if self.draw_all_faces {
                self.invalidate();
            }
        }
        let settings = &frame.settings;
        let count = if settings.enabled && settings.count > 0 {
            lights.len().min(MAX_SCENE_LIGHTS)
        } else {
            0
        };
        let selected = self.selection.update(
            &lights[..count],
            if settings.shadows {
                usize::from(settings.shadow_count)
                    .min(MAX_SHADOW_LIGHTS)
                    .min(count)
            } else {
                0
            },
            frame.elapsed,
        );
        self.grid
            .update(queue, &lights[..count], usize::from(settings.count));
        let mut data = vec![0.0f32; LIGHT_WORDS + FACES * 16];
        let receivers = super::culling::Volume::new(camera);
        for i in 0..self.grid.order.len() {
            let source = self.grid.order[i];
            let light = &lights[source];
            data[i * 8..i * 8 + 4].copy_from_slice(&light.position.extend(light.radius).to_array());
            data[i * 8 + 4..i * 8 + 7].copy_from_slice(&light.color);
            let Some((slot, strength)) = selected[source] else {
                continue;
            };
            data[i * 8 + 7] = (slot + 1) as f32 + strength * 0.5;
            for (face, matrix) in matrices(light).iter().enumerate() {
                let index = slot * 6 + face;
                data[LIGHT_WORDS + index * 16..LIGHT_WORDS + (index + 1) * 16]
                    .copy_from_slice(&matrix.to_cols_array());
                if !self.face_visible(&receivers, *matrix) {
                    continue;
                }
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
                        light.position.x,
                        light.position.y,
                        light.position.z,
                        f32::from(settings.source_size),
                        light.emission_radius,
                        0.0,
                        0.0,
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
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                ],
                geometry,
            );
        }
        data.extend([
            frame.ambient,
            f32::from(settings.intensity) / 100.0,
            usize::from(settings.count).min(MAX_LIGHTS) as f32,
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
        let weather_inverse = weather_matrices.map(|matrix| matrix.inverse());
        for inverse in weather_inverse {
            data.extend(inverse.to_cols_array());
        }
        data.extend([
            frame.weather.wet,
            frame.weather.snow,
            frame.weather.time,
            frame.weather.puddles,
        ]);
        data.extend(frame.rain);
        data.extend([
            f32::from(settings.source_size),
            frame.day,
            frame.weather.flash,
            0.0,
        ]);
        data.extend(self.grid.uniform());
        data.extend(self.filter.iter().flatten().copied());
        data.extend(
            weather_inverse[0]
                .z_axis
                .truncate()
                .normalize()
                .extend(0.0)
                .to_array(),
        );
        for inverse in weather_inverse {
            data.extend([
                inverse.x_axis.truncate().length() * 2.0,
                inverse.y_axis.truncate().length() * 2.0,
                0.0,
                0.0,
            ]);
        }
        if self.weather_resolution > 1 {
            for (layer, matrix) in weather_matrices.into_iter().enumerate() {
                self.draw_face(
                    encoder,
                    queue,
                    FACES + layer,
                    matrix,
                    [
                        0.0,
                        frame.visual_time,
                        frame.wind,
                        1.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    geometry,
                );
            }
        }
        queue.write_buffer(&self.data, 0, bytemuck::cast_slice(&data));
    }

    fn face_visible(&self, receivers: &super::culling::Volume, matrix: Mat4) -> bool {
        #[cfg(test)]
        if self.draw_all_faces {
            return true;
        }
        // Only cull receiver volumes; off-camera geometry can still cast visible shadows.
        receivers.intersects(&super::culling::Volume::new(matrix))
    }

    fn draw_face(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        index: usize,
        matrix: Mat4,
        options: [f32; 12],
        geometry: &Geometry,
    ) {
        let frustum = super::culling::Frustum::new(matrix);
        let size = if index >= FACES {
            self.weather_resolution
        } else if index == FACES - 1 {
            self.sun_resolution
        } else {
            self.resolution
        };
        let mut foliage = false;
        let mut damage: Option<Rect> = None;
        let mut draws = Vec::new();
        for mesh in geometry
            .meshes
            .iter()
            .copied()
            .chain(geometry.effects.map(|e| &e.mesh))
        {
            if mesh.count == 0 {
                continue;
            }
            let (ranges, animated) = if mesh.shadow_chunks.is_empty() {
                damage = Some(Rect::full(size));
                (std::iter::once(0..mesh.count).collect(), true)
            } else {
                frustum.ranges(&mesh.shadow_chunks, options[2])
            };
            if !ranges.is_empty() {
                foliage |= animated;
                if let Some(rect) =
                    frustum.damage(&mesh.shadow_chunks, options[2], mesh.dynamic, matrix, size)
                {
                    damage = Some(damage.map_or(rect, |old| old.union(rect)));
                }
                draws.push((mesh, ranges));
            }
        }
        let mut state = FaceState {
            matrix,
            options,
            meshes: draws
                .iter()
                .map(|(mesh, _)| (mesh.vertices.pose.clone(), mesh.revision))
                .collect(),
            static_meshes: draws
                .iter()
                .filter(|(mesh, _)| !mesh.dynamic)
                .map(|(mesh, _)| (mesh.vertices.pose.clone(), mesh.revision))
                .collect(),
            damage,
        };
        // Time affects depth only where wind actually moves a caster.
        if !foliage || options[2] == 0.0 {
            state.options[1..3].fill(0.0);
        }
        if self.cached[index].as_ref() == Some(&state) {
            return;
        }
        // Clear both footprints so moving casters uncover the fixed geometry below.
        let mut region = Rect::full(size);
        if let Some(previous) = self.cached[index].as_ref() {
            let mut a = previous.options;
            let mut b = state.options;
            a[1] = 0.0;
            b[1] = 0.0;
            if previous.matrix == state.matrix
                && a == b
                && previous.static_meshes == state.static_meshes
            {
                let mut damage = previous.damage;
                if let Some(rect) = state.damage {
                    damage = Some(damage.map_or(rect, |old| old.union(rect)));
                }
                if let Some(damage) = damage {
                    region = damage;
                }
            }
        }
        self.cached[index] = Some(state);
        #[cfg(test)]
        {
            self.rendered_faces += 1;
            self.rendered_area +=
                u64::from(region.max[0] - region.min[0]) * u64::from(region.max[1] - region.min[1]);
        }
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
        let colors = [Some(wgpu::RenderPassColorAttachment {
            view: self.readable.face(index),
            resolve_target: None,
            ops: wgpu::Operations {
                load: if region == Rect::full(size) {
                    wgpu::LoadOp::Clear(wgpu::Color::WHITE)
                } else {
                    wgpu::LoadOp::Load
                },
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shadow face"),
            color_attachments: if self.gl { &colors } else { &[] },
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target,
                depth_ops: Some(wgpu::Operations {
                    load: if region == Rect::full(size) {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_bind_group(0, bindings, &[]);
        pass.set_bind_group(1, geometry.materials, &[]);
        pass.set_scissor_rect(
            region.min[0],
            region.min[1],
            region.max[0] - region.min[0],
            region.max[1] - region.min[1],
        );
        if region != Rect::full(size) {
            pass.set_pipeline(&self.clear_pipeline);
            pass.draw(0..3, 0..1);
        }
        pass.set_pipeline(if index >= FACES {
            &self.weather_pipeline
        } else {
            &self.pipeline
        });
        for (mesh, ranges) in draws {
            mesh.bind(&mut pass);
            for range in ranges {
                mesh.draw(&mut pass, range);
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

struct MapTargets<'a> {
    views: [&'a wgpu::TextureView; 2],
    grid: &'a wgpu::TextureView,
    sizes: [u32; 2],
    gl: bool,
}
struct ReadableDepth {
    point: (wgpu::TextureView, Vec<wgpu::TextureView>),
    weather: (wgpu::TextureView, Vec<wgpu::TextureView>),
    sun: wgpu::TextureView,
}
impl ReadableDepth {
    fn new(device: &wgpu::Device, sizes: [u32; 3], layers: usize) -> Self {
        let array = |size, count| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Readable shadow depth"),
                size: wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: super::array_layers(device, count),
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let all = texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
            let faces = (0..count)
                .map(|layer| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: layer,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect();
            (all, faces)
        };
        Self {
            point: array(sizes[0], layers as u32),
            sun: array(sizes[1], 1).1.pop().unwrap(),
            weather: array(sizes[2], 2),
        }
    }
    fn face(&self, index: usize) -> &wgpu::TextureView {
        if index >= FACES {
            &self.weather.1[index - FACES]
        } else if index == FACES - 1 {
            &self.sun
        } else {
            &self.point.1[index]
        }
    }
}

fn maps(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    data: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    resolution: u32,
    layers: usize,
    sky: MapTargets<'_>,
) -> (wgpu::BindGroup, Vec<wgpu::TextureView>, ReadableDepth) {
    let readable = ReadableDepth::new(
        device,
        if sky.gl {
            [resolution, sky.sizes[0], sky.sizes[1]]
        } else {
            [1; 3]
        },
        layers,
    );
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
                resource: wgpu::BindingResource::TextureView(sky.views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(sky.views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&readable.point.0),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&readable.weather.0),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(sky.grid),
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
    (bindings, views, readable)
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
