use crate::render_mesh::Vertex;
use glam::Mat4;
use tenchu_assets::{Result, texture::Vram};
use wgpu::util::DeviceExt;

pub struct Mesh {
    buffer: wgpu::Buffer,
    count: u32,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    camera: wgpu::Buffer,
    depth: wgpu::TextureView,
}

impl Renderer {
    pub async fn new(
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        size: [u32; 2],
        vram: &Vram,
    ) -> Result<Self> {
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Tenchu renderer"),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original paletted textures"),
            source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Scene"), layout: None,
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vertex"), compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32, 4 => Uint32, 5 => Uint32],
                }],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual, stencil: Default::default(), bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fragment"), compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview: None, cache: None,
        });
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera"),
            contents: bytemuck::cast_slice(&Mat4::IDENTITY.to_cols_array()),
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
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&vram.words),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(2048),
                rows_per_image: Some(512),
            },
            texture.size(),
        );
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene textures and camera"),
            layout: &pipeline.get_bind_group_layout(0),
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
        let depth = Self::depth(&device, size);
        Ok(Self {
            device,
            queue,
            pipeline,
            bindings,
            camera,
            depth,
        })
    }

    fn depth(device: &wgpu::Device, [width, height]: [u32; 2]) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.depth = Self::depth(&self.device, size);
    }

    pub fn upload(&self, vertices: &[Vertex]) -> Mesh {
        Mesh {
            buffer: self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Geometry"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            count: vertices.len() as u32,
        }
    }

    pub fn draw(&self, target: &wgpu::TextureView, matrix: Mat4, meshes: &[&Mesh]) {
        self.queue.write_buffer(
            &self.camera,
            0,
            bytemuck::cast_slice(&matrix.to_cols_array()),
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.008,
                            g: 0.012,
                            b: 0.02,
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
            for mesh in meshes {
                pass.set_vertex_buffer(0, mesh.buffer.slice(..));
                pass.draw(0..mesh.count, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
    }
}

pub fn camera(eye: [f32; 3], target: [f32; 3], aspect: f32) -> Mat4 {
    let eye = glam::Vec3::from(eye);
    let target = glam::Vec3::from(target);
    let up = if (target - eye).normalize_or_zero().y.abs() > 0.9999 {
        glam::Vec3::Z
    } else {
        glam::Vec3::NEG_Y
    };
    Mat4::perspective_rh(2.0 * (120.0f32 / 300.0).atan(), aspect, 50.0, 250_000.0)
        * Mat4::look_at_rh(eye, target, up)
}

pub fn game_camera(view: tenchu_core::math::Matrix, aspect: f32) -> Mat4 {
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
    Mat4::perspective_rh(2.0 * (120.0f32 / 300.0).atan(), aspect, 50.0, 250_000.0)
        * Mat4::from_scale(glam::Vec3::new(1.0, -1.0, -1.0))
        * Mat4::from_cols_array_2d(&columns)
}
