use crate::{render_effects, render_mesh::Vertex};
use glam::Mat4;
use tenchu_assets::{Result, texture::Vram};
use tenchu_core::projection::{DEPTH_CUE, DISTANCE};
use wgpu::util::DeviceExt;

mod materials;
mod pipelines;
mod post;
mod shadows;
mod sun;

#[cfg(test)]
mod tests;

pub struct Mesh {
    buffer: wgpu::Buffer,
    count: u32,
}

pub struct EffectMesh {
    mesh: Mesh,
    batches: Vec<render_effects::Batch>,
    ranges: [Vec<std::ops::Range<u32>>; 2],
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    effects: [wgpu::RenderPipeline; 5],
    overlay_effects: [wgpu::RenderPipeline; 5],
    post: Option<post::Post>,
    size: [u32; 2],
    bindings: wgpu::BindGroup,
    camera: wgpu::Buffer,
    depth: wgpu::TextureView,
    texture: wgpu::Texture,
    format: wgpu::TextureFormat,
    layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    pub sample_counts: Vec<u32>,
    samples: u32,
    multisample: Option<wgpu::TextureView>,
    graphics: crate::settings::Graphics,
    shadows: shadows::Shadows,
    materials: materials::Materials,
}

impl Renderer {
    pub async fn new(
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        size: [u32; 2],
        vram: &Vram,
    ) -> Result<Self> {
        let features =
            adapter.features() & wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        let format_flags = |format: wgpu::TextureFormat| {
            if features.is_empty() {
                format.guaranteed_format_features(features).flags
            } else {
                adapter.get_texture_format_features(format).flags
            }
        };
        let color = format_flags(format);
        let hdr = format_flags(post::FORMAT);
        let depth = format_flags(wgpu::TextureFormat::Depth32Float);
        let sample_counts = [1, 2, 4, 8, 16]
            .into_iter()
            .filter(|&n| {
                color.sample_count_supported(n)
                    && hdr.sample_count_supported(n)
                    && depth.sample_count_supported(n)
                    && (n == 1
                        || (color & hdr)
                            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE))
            })
            .collect();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Tenchu renderer"),
                    required_features: features,
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original paletted textures"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "const MAX_LIGHTS: u32 = {}u; const SHADOW_FACES: u32 = {}u; const MAX_MIPS: u32 = {}u; const SHADOW_NEAR: f32 = {};\n{}\n{}\n{}\n{}",
                    crate::lighting::MAX_LIGHTS,
                    shadows::FACES,
                    materials::MAX_MIPS,
                    shadows::NEAR,
                    include_str!("scene.wgsl"),
                    include_str!("renderer/palette.wgsl"),
                    include_str!("renderer/lighting.wgsl"),
                    include_str!("renderer/materials.wgsl")
                )
                .into(),
            ),
        });
        let binding_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera"),
            contents: bytemuck::bytes_of(&[0.0f32; 32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture words"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let materials = materials::Materials::new(&device, &queue, vram);
        let shadows = shadows::Shadows::new(&device, &texture, &materials.layout);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scene and effects"),
            bind_group_layouts: &[&binding_layout, &shadows.layout, &materials.layout],
            push_constant_ranges: &[],
        });
        let (pipeline, effects) = pipelines::create(&device, format, &layout, &shader, 1);
        let (_, overlay_effects) = pipelines::create(&device, format, &layout, &shader, 1);
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene textures and camera"),
            layout: &binding_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
            ],
        });
        let depth = Self::attachment(&device, size, wgpu::TextureFormat::Depth32Float, 1);
        let mut renderer = Self {
            device,
            queue,
            pipeline,
            effects,
            overlay_effects,
            post: None,
            size,
            bindings,
            camera,
            depth,
            texture,
            format,
            layout,
            shader,
            sample_counts,
            samples: 1,
            multisample: None,
            graphics: crate::settings::Graphics::default(),
            shadows,
            materials,
        };
        renderer.update_vram(vram);
        Ok(renderer)
    }

    fn attachment(
        device: &wgpu::Device,
        [width, height]: [u32; 2],
        format: wgpu::TextureFormat,
        samples: u32,
    ) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.size = size;
        self.depth = Self::attachment(
            &self.device,
            size,
            wgpu::TextureFormat::Depth32Float,
            self.samples,
        );
        self.multisample = (self.samples > 1)
            .then(|| Self::attachment(&self.device, size, self.world_format(), self.samples));
        self.post =
            self.graphics.post.enabled.then(|| {
                post::Post::new(&self.device, self.format, size, self.samples, &self.depth)
            });
    }

    fn world_format(&self) -> wgpu::TextureFormat {
        if self.graphics.post.enabled {
            post::FORMAT
        } else {
            self.format
        }
    }

    pub fn set_asset_pack(&mut self, pack: &crate::visuals::pack::Pack) {
        self.materials.set_pack(pack.textures.clone());
    }

    pub fn set_emitters(&mut self, emitters: &crate::visuals::lights::Emitters) {
        self.materials.set_emissions(emitters.emissions.clone());
    }

    pub fn bind_models(&mut self, models: &mut crate::visuals::models::Models) {
        self.materials.bind_models(models);
    }

    pub fn reset_vram(&mut self, vram: &Vram) {
        self.materials.reset(vram);
        self.update_vram(vram);
    }

    pub fn configure(&mut self, graphics: &crate::settings::Graphics) {
        self.materials.configure(graphics);
        let samples = if self.sample_counts.contains(&graphics.samples) {
            graphics.samples
        } else {
            1
        };
        let rebuild =
            samples != self.samples || graphics.post.enabled != self.graphics.post.enabled;
        self.graphics = graphics.clone();
        if rebuild {
            (self.pipeline, self.effects) = pipelines::create(
                &self.device,
                self.world_format(),
                &self.layout,
                &self.shader,
                samples,
            );
            self.samples = samples;
            self.resize(self.size);
        }
    }

    pub fn update_vram(&mut self, vram: &Vram) {
        self.materials.update(vram);
        self.queue.write_texture(
            self.texture.as_image_copy(),
            bytemuck::cast_slice(&vram.words),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(2048),
                rows_per_image: Some(512),
            },
            self.texture.size(),
        );
    }

    pub fn upload(&mut self, vertices: &[Vertex]) -> Mesh {
        let mut vertices = vertices.to_vec();
        self.materials.annotate(&mut vertices);
        Mesh {
            buffer: self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Geometry"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            count: vertices.len() as u32,
        }
    }

    pub fn upload_effects(&mut self, geometry: render_effects::Geometry) -> EffectMesh {
        let mut ranges: [Vec<std::ops::Range<u32>>; 2] = Default::default();
        for (i, triangle) in geometry.vertices.chunks_exact(3).enumerate() {
            let overlay =
                triangle[0].flags & (crate::render_mesh::UI | crate::render_mesh::DEBUG) != 0;
            let ranges = &mut ranges[usize::from(overlay)];
            let start = i as u32 * 3;
            if let Some(last) = ranges.last_mut().filter(|last| last.end == start) {
                last.end += 3;
            } else {
                ranges.push(start..start + 3);
            }
        }
        EffectMesh {
            mesh: self.upload(&geometry.vertices),
            batches: geometry.batches,
            ranges,
        }
    }

    #[cfg(test)]
    pub fn draw(
        &mut self,
        target: &wgpu::TextureView,
        matrix: Mat4,
        meshes: &[&Mesh],
        effects: Option<&EffectMesh>,
    ) -> Result<()> {
        self.draw_lit(target, matrix, meshes, effects, &[], meshes)
    }

    pub fn draw_lit(
        &mut self,
        target: &wgpu::TextureView,
        matrix: Mat4,
        meshes: &[&Mesh],
        effects: Option<&EffectMesh>,
        lights: &[crate::lighting::Light],
        casters: &[&Mesh],
    ) -> Result<()> {
        self.materials.flush(&self.device, &self.queue)?;
        let mut camera = matrix.to_cols_array().to_vec();
        camera.extend([
            self.size[0] as f32 / self.size[1] as f32,
            f32::from(self.graphics.anisotropy),
            u8::from(self.post.is_some()) as f32,
            f32::from(self.graphics.magnification),
        ]);
        camera.extend([
            if self.graphics.fog {
                DEPTH_CUE[0] as f32 * f32::from(DISTANCE) / 256.0
                    * f32::from(self.graphics.fog_distance)
                    / 100.0
            } else {
                0.0
            },
            if self.graphics.fog {
                DEPTH_CUE[1] as f32 / 16_777_216.0
            } else {
                0.0
            },
            0.0,
            0.0,
        ]);
        camera.extend([
            if self.post.is_some() {
                1.0
            } else {
                f32::from(self.graphics.exposure) / 100.0
            },
            u8::from(self.graphics.textures) as f32,
            u8::from(self.graphics.shading) as f32,
            if self.graphics.mipmaps {
                2.0
            } else {
                u8::from(self.graphics.bilinear) as f32
            },
        ]);
        camera.extend(self.graphics.post.fog_color());
        camera.push(if self.post.is_some() {
            f32::from(self.graphics.post.haze) / 100.0
        } else {
            0.0
        });
        self.queue
            .write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.shadows
            .configure(&self.device, &self.graphics.lighting, lights.len());
        self.shadows.draw(
            &mut encoder,
            &self.queue,
            &self.graphics.lighting,
            matrix,
            lights,
            casters,
            effects,
            &self.materials.bindings,
        );
        if let Some(post) = &self.post {
            post.prepare(&self.queue, matrix, &self.graphics);
        }
        {
            let world_target = self.post.as_ref().map_or(target, |post| &post.scene);
            let clear = self.graphics.post.fog_color().map(f64::from);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.multisample.as_ref().unwrap_or(world_target),
                    resolve_target: self.multisample.as_ref().map(|_| world_target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0],
                            g: clear[1],
                            b: clear[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bindings, &[]);
            pass.set_bind_group(1, &self.shadows.bindings, &[]);
            pass.set_bind_group(2, &self.materials.bindings, &[]);
            for mesh in meshes {
                if mesh.count == 0 {
                    continue;
                }
                pass.set_vertex_buffer(0, mesh.buffer.slice(..));
                pass.draw(0..mesh.count, 0..1);
            }
            if let Some(effects) = effects.filter(|e| e.mesh.count != 0) {
                draw_effects(
                    &mut pass,
                    &self.effects,
                    effects,
                    self.post.as_ref().map(|_| false),
                );
            }
        }
        if let Some(post) = &self.post {
            post.draw(&mut encoder, target, self.graphics.post.bloom > 0);
            if let Some(effects) = effects.filter(|e| !e.ranges[1].is_empty()) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Unprocessed HUD and inspection overlays"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &post.depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_bind_group(0, &self.bindings, &[]);
                pass.set_bind_group(1, &self.shadows.bindings, &[]);
                pass.set_bind_group(2, &self.materials.bindings, &[]);
                draw_effects(&mut pass, &self.overlay_effects, effects, Some(true));
            }
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }
}

fn draw_effects(
    pass: &mut wgpu::RenderPass<'_>,
    pipelines: &[wgpu::RenderPipeline; 5],
    effects: &EffectMesh,
    overlay: Option<bool>,
) {
    pass.set_vertex_buffer(0, effects.mesh.buffer.slice(..));
    let ranges = overlay.map(|overlay| &effects.ranges[usize::from(overlay)]);
    pass.set_pipeline(&pipelines[0]);
    if let Some(ranges) = ranges {
        for range in ranges {
            pass.draw(range.clone(), 0..1);
        }
    } else {
        pass.draw(0..effects.mesh.count, 0..1);
    }
    for batch in &effects.batches {
        if batch.blend == 0 {
            continue;
        }
        let constant = if batch.blend == 4 { 0.25 } else { 0.5 };
        pass.set_blend_constant(wgpu::Color {
            r: constant,
            g: constant,
            b: constant,
            a: 1.0,
        });
        pass.set_pipeline(&pipelines[usize::from(batch.blend)]);
        if let Some(ranges) = ranges {
            for range in ranges {
                let start = range.start.max(batch.range.start);
                let end = range.end.min(batch.range.end);
                if start < end {
                    pass.draw(start..end, 0..1);
                }
            }
        } else {
            pass.draw(batch.range.clone(), 0..1);
        }
    }
}

pub fn game_camera(view: tenchu_core::math::Matrix, aspect: f32) -> Mat4 {
    Mat4::perspective_rh(
        2.0 * (120.0f32 / f32::from(DISTANCE)).atan(),
        aspect,
        50.0,
        250_000.0,
    ) * view_matrix(view)
}

pub fn view_matrix(view: tenchu_core::math::Matrix) -> Mat4 {
    let mut columns = [[0.0; 4]; 4];
    for (column, values) in columns.iter_mut().take(3).enumerate() {
        for (row, value) in values.iter_mut().take(3).enumerate() {
            *value = f32::from(view.rotation[row][column]) / 4096.0;
        }
    }
    columns[3] = [
        view.translation[0] as f32,
        view.translation[1] as f32,
        view.translation[2] as f32,
        1.0,
    ];
    Mat4::from_scale(glam::Vec3::new(1.0, -1.0, -1.0)) * Mat4::from_cols_array_2d(&columns)
}
