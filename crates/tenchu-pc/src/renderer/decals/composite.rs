use super::{Frustum, Mat4, Renderer};
use crate::renderer::{culling::Rect, geometry};

pub(in crate::renderer) struct Composite {
    layout: wgpu::BindGroupLayout,
    coverage: wgpu::RenderPipeline,
    shaded: [wgpu::RenderPipeline; 2],
    pub targets: Option<Targets>,
}

pub(in crate::renderer) struct Targets {
    opacity: wgpu::TextureView,
    identity: wgpu::TextureView,
    surface: wgpu::TextureView,
    bindings: wgpu::BindGroup,
}

impl Composite {
    pub fn new(renderer: &Renderer) -> Self {
        let device = &renderer.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blood composition"),
            entries: &std::array::from_fn::<_, 3, _>(|binding| wgpu::BindGroupLayoutEntry {
                binding: binding as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: if binding == 1 {
                        wgpu::TextureSampleType::Uint
                    } else {
                        wgpu::TextureSampleType::Float { filterable: false }
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }),
        });
        let full = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blood composition pipeline"),
            bind_group_layouts: &[
                &renderer.pipelines.opaque.get_bind_group_layout(0),
                &renderer.shadows.layout,
                &renderer.materials.layout,
                &layout,
            ],
            push_constant_ranges: &[],
        });
        let buffers = [
            geometry::LAYOUTS[0].clone(),
            geometry::LAYOUTS[1].clone(),
            wgpu::VertexBufferLayout {
                array_stride: 4,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![9 => Uint32],
            },
        ];
        let pipeline = |entry: &str,
                        targets: &[Option<wgpu::ColorTargetState>],
                        layout: &wgpu::PipelineLayout| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &renderer.shader,
                    entry_point: Some("blood_vertex"),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: -2,
                        ..Default::default()
                    },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &renderer.shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets,
                }),
                multiview: None,
                cache: None,
            })
        };
        let alpha = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let min = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Min,
        };
        let coverage = pipeline(
            "blood_coverage",
            &[
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R16Float,
                    blend: Some(wgpu::BlendState {
                        color: alpha,
                        alpha,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R32Uint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState {
                        color: min,
                        alpha: min,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
            &renderer.layout,
        );
        let shaded = ["blood_composite", "snow_blood_composite"].map(|entry| {
            pipeline(
                entry,
                &[Some(wgpu::ColorTargetState {
                    format: renderer.world_format(),
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                &full,
            )
        });
        Self {
            layout,
            coverage,
            shaded,
            targets: None,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        if self.targets.is_some() {
            return;
        }
        let opacity = Renderer::attachment(device, size, wgpu::TextureFormat::R16Float, 1);
        let identity = Renderer::attachment(device, size, wgpu::TextureFormat::R32Uint, 1);
        let surface = Renderer::attachment(device, size, wgpu::TextureFormat::Rgba16Float, 1);
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blood composition textures"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&opacity),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&identity),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&surface),
                },
            ],
        });
        self.targets = Some(Targets {
            opacity,
            identity,
            surface,
            bindings,
        });
    }

    fn geometry(renderer: &Renderer, pass: &mut wgpu::RenderPass<'_>, matrix: Mat4) {
        let frustum = Frustum::new(matrix);
        let mut first = 1;
        for (index, decal) in renderer
            .decals
            .iter()
            .enumerate()
            .filter(|(_, d)| frustum.contains(d.bounds, 2.0))
        {
            decal.mesh.bind(pass);
            pass.set_vertex_buffer(2, decal.source_ids.as_ref().unwrap().slice(..));
            if let Some(cache) = &renderer.blood_cache {
                for range in &cache.fallback[index] {
                    pass.draw(range.clone(), first..first + 1);
                }
            } else {
                pass.draw(0..decal.mesh.count, first..first + 1);
            }
            first += decal.mesh.count / 3;
        }
    }

    pub fn coverage(&self, renderer: &Renderer, encoder: &mut wgpu::CommandEncoder, matrix: Mat4) {
        let targets = self.targets.as_ref().unwrap();
        let attachment = |view, clear| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Blood coverage"),
            color_attachments: &[
                attachment(&targets.opacity, wgpu::Color::TRANSPARENT),
                attachment(&targets.identity, wgpu::Color::TRANSPARENT),
                attachment(
                    &targets.surface,
                    wgpu::Color {
                        r: 2047.0,
                        g: 2047.0,
                        b: 2047.0,
                        a: 2047.0,
                    },
                ),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &renderer.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.coverage);
        pass.set_bind_group(0, &renderer.bindings, &[]);
        pass.set_bind_group(1, &renderer.shadows.bindings, &[]);
        pass.set_bind_group(2, &renderer.materials.bindings, &[]);
        Self::geometry(renderer, &mut pass, matrix);
    }

    pub fn shade(&self, renderer: &Renderer, pass: &mut wgpu::RenderPass<'_>, matrix: Mat4) {
        pass.set_pipeline(&self.shaded[usize::from(renderer.snow_bounds())]);
        pass.set_bind_group(3, &self.targets.as_ref().unwrap().bindings, &[]);
        Self::geometry(renderer, pass, matrix);
    }
}

pub(in crate::renderer) fn useful(renderer: &Renderer, matrix: Mat4) -> bool {
    if renderer.composite.is_none() {
        return false;
    }
    #[cfg(test)]
    if let Some(force) = renderer.force_blood_composite {
        return force;
    }
    let frustum = Frustum::new(matrix);
    let mut area = 0_u64;
    let mut count = 0;
    let mut union: Option<Rect> = None;
    for (index, decal) in renderer
        .decals
        .iter()
        .enumerate()
        .filter(|(_, d)| frustum.contains(d.bounds, 2.0))
    {
        if renderer
            .blood_cache
            .as_ref()
            .is_some_and(|cache| cache.fallback[index].is_empty())
        {
            continue;
        }
        if !decal.ground_blood {
            return false;
        }
        for &bounds in &decal.source_bounds {
            let rect = Rect::projected(bounds, matrix, 256, 0.0);
            area += u64::from(rect.max[0] - rect.min[0]) * u64::from(rect.max[1] - rect.min[1]);
            union = Some(union.map_or(rect, |old| old.union(rect)));
            count += 1;
        }
    }
    // Avoid full-resolution coverage targets for a few stains or distant, sparse marks.
    count >= 128
        && union.is_some_and(|rect| {
            let occupied =
                u64::from(rect.max[0] - rect.min[0]) * u64::from(rect.max[1] - rect.min[1]);
            area > occupied + 256 * 256
        })
}
