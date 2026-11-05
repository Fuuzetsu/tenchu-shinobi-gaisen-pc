use super::{Mat4, buffer, texture_entry, uniform_entry};
use crate::{lighting::Light, renderer::culling::Frustum};
use glam::Vec3;

const MAX_PLUMES: usize = 32;

pub(super) struct Heat {
    pub target: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    data: wgpu::Buffer,
    vertices: wgpu::Buffer,
    count: u32,
}

impl Heat {
    pub fn new(
        device: &wgpu::Device,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        depth_sampler: &wgpu::Sampler,
        samples: u32,
        gl: bool,
    ) -> Self {
        let data = buffer(device, 24 * 4);
        let mut uniform = uniform_entry(0);
        uniform.visibility = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Heat plume depth"),
            entries: &[
                uniform,
                texture_entry(1, true, samples > 1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Heat plume depth"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: data.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(depth_sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flame heat plumes"),
            source: wgpu::ShaderSource::Wgsl(
                super::depth_shader(include_str!("heat.wgsl"), samples, gl).into(),
            ),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Flame heat plumes"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Heat plume depth"),
                    bind_group_layouts: &[&layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
                }],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: super::FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visible flame heat plumes"),
            size: (MAX_PLUMES * 32) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            target: target.clone(),
            pipeline,
            bindings,
            data,
            vertices,
            count: 0,
        }
    }

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        camera: Mat4,
        lights: &[Light],
        time: f32,
        strength: u16,
        size: [u32; 2],
    ) -> bool {
        let frustum = Frustum::new(camera);
        let mut plumes: Vec<_> = lights
            .iter()
            .filter(|l| l.flame)
            .filter_map(|light| {
                let width = (light.emission_radius * 0.7).clamp(300.0, 800.0);
                let height = width * 4.0;
                let center = light.position - Vec3::Y * height * 0.5;
                let extent = Vec3::new(width, height * 0.5, width);
                let clip = camera * center.extend(1.0);
                if clip.w <= 0.0 || !frustum.contains([center - extent, center + extent], 0.0) {
                    return None;
                }
                Some((
                    clip.w,
                    [
                        light.position.x,
                        light.position.y,
                        light.position.z,
                        width,
                        height,
                        crate::lighting::flames::phase(light.position),
                        0.0,
                        0.0,
                    ],
                ))
            })
            .collect();
        plumes.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        plumes.truncate(MAX_PLUMES);
        let instances: Vec<_> = plumes.into_iter().map(|(_, instance)| instance).collect();
        self.count = instances.len() as u32;
        if self.count == 0 {
            return false;
        }
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&instances));
        let inverse = camera.inverse();
        let right = inverse.x_axis.truncate().normalize_or_zero();
        let mut data = camera.to_cols_array().to_vec();
        data.extend(right.extend(0.0).to_array());
        data.extend([
            time,
            f32::from(strength) / 100.0,
            size[0] as f32,
            size[1] as f32,
        ]);
        queue.write_buffer(&self.data, 0, bytemuck::cast_slice(&data));
        true
    }

    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.count == 0 {
            return;
        }
        let mut attachment = super::attachment(&self.target);
        attachment.ops.load = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Quarter-resolution flame heat"),
            color_attachments: &[Some(attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..6, 0..self.count);
    }
}
