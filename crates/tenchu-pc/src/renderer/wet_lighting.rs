use super::{Mesh, Renderer, culling::Frustum, geometry, pipelines};
use crate::{render_mesh, settings::Graphics};
use glam::Mat4;
use std::ops::Range;

const FORMATS: [wgpu::TextureFormat; 3] = [
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::Rgba32Uint,
    wgpu::TextureFormat::Rgba16Float,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Material {
    Surface,
    Water,
    Forward,
}

pub(super) struct Batch {
    pub material: Material,
    pub vertices: Range<u32>,
}

struct Targets {
    size: [u32; 2],
    views: [wgpu::TextureView; 3],
    bindings: wgpu::BindGroup,
}

pub(super) struct WetLighting {
    layout: wgpu::BindGroupLayout,
    targets: Option<Targets>,
    surface: wgpu::RenderPipeline,
    forward: wgpu::RenderPipeline,
    lighting: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
}

impl WetLighting {
    pub fn new(renderer: &Renderer) -> Self {
        let device = &renderer.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Prepared wet surfaces"),
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
        let lighting_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Wet surface lighting"),
            bind_group_layouts: &[
                &renderer.pipelines.opaque.get_bind_group_layout(0),
                &renderer.shadows.layout,
                &renderer.materials.layout,
                &layout,
            ],
            push_constant_ranges: &[],
        });
        let shader = pipelines::Options::new(&renderer.graphics, renderer.samples)
            .wet_shader(device, renderer.gl);
        let pipeline = |vertex,
                        fragment,
                        layout: &wgpu::PipelineLayout,
                        formats: &[wgpu::TextureFormat],
                        surface| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fragment),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: if surface { &geometry::LAYOUTS } else { &[] },
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: surface,
                    depth_compare: if surface {
                        wgpu::CompareFunction::Less
                    } else {
                        wgpu::CompareFunction::Always
                    },
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &formats
                        .iter()
                        .map(|&format| {
                            Some(wgpu::ColorTargetState {
                                format,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            })
                        })
                        .collect::<Vec<_>>(),
                }),
                multiview: None,
                cache: None,
            })
        };
        Self {
            sky: pipeline("vertex", "deferred_sky", &renderer.layout, &FORMATS, true),
            surface: pipeline(
                "vertex",
                "deferred_material",
                &renderer.layout,
                &FORMATS,
                true,
            ),
            forward: pipeline(
                "vertex",
                "deferred_fallback",
                &renderer.layout,
                &FORMATS,
                true,
            ),
            lighting: pipeline(
                "deferred_vertex",
                "deferred_light",
                &lighting_layout,
                &[renderer.world_format()],
                false,
            ),
            layout,
            targets: None,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        if self
            .targets
            .as_ref()
            .is_some_and(|targets| targets.size == size)
        {
            return;
        }
        let views = FORMATS.map(|format| Renderer::attachment(device, size, format, 1));
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Prepared wet surfaces"),
            layout: &self.layout,
            entries: &std::array::from_fn::<_, 3, _>(|i| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(&views[i]),
            }),
        });
        self.targets = Some(Targets {
            size,
            views,
            bindings,
        });
    }

    pub fn surfaces(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        meshes: &[&Mesh],
        sky: Option<&Mesh>,
    ) {
        let targets = self.targets.as_ref().unwrap();
        let attachments = targets.views.each_ref().map(|view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Wet surface materials"),
            color_attachments: &attachments,
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
        pass.set_bind_group(0, &renderer.bindings, &[]);
        pass.set_bind_group(1, &renderer.shadows.bindings, &[]);
        pass.set_bind_group(2, &renderer.materials.bindings, &[]);
        if let Some(sky) = sky {
            pass.set_pipeline(&self.sky);
            sky.bind(&mut pass);
            sky.draw(&mut pass, 0..sky.count);
        }
        for mesh in meshes.iter().filter(|mesh| mesh.count > 0 && mesh.scenery) {
            mesh.bind(&mut pass);
            // Keep authoring order so coplanar backing faces only fill texture cutouts.
            for batch in &mesh.material_batches {
                pass.set_pipeline(match batch.material {
                    Material::Surface | Material::Water => &self.surface,
                    Material::Forward => &self.forward,
                });
                mesh.draw(&mut pass, batch.vertices.clone());
            }
        }
    }

    pub fn light(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.lighting);
        pass.set_bind_group(3, &self.targets.as_ref().unwrap().bindings, &[]);
        pass.draw(0..3, 0..1);
    }
}

pub(super) fn configured(graphics: &Graphics, samples: u32, gl: bool) -> bool {
    !gl && samples == 1
        && graphics.post.enabled
        && graphics.weather.mode != crate::settings::WeatherMode::Original
        && graphics.lighting.enabled
        && graphics.lighting.shadows
        && graphics.lighting.source_size > 0
}

pub(super) fn useful(renderer: &Renderer, matrix: Mat4, meshes: &[&Mesh]) -> bool {
    if renderer.wet_lighting.is_none()
        || renderer.weather.wet + renderer.weather.puddles + renderer.weather.snow <= 0.0
    {
        return false;
    }
    if meshes.iter().any(|mesh| mesh.count > 0 && !mesh.scenery) {
        return false;
    }
    let frustum = Frustum::new(matrix);
    let wind = f32::from(renderer.graphics.motion.wind) / 100.0;
    meshes.iter().any(|mesh| {
        mesh.scenery
            && mesh
                .material_batches
                .iter()
                .any(|batch| batch.material == Material::Surface)
            && frustum.any_visible(&mesh.shadow_chunks, wind)
    })
}

pub(super) fn update_batches(batches: &mut Vec<Batch>, flags: impl ExactSizeIterator<Item = u32>) {
    let count = flags.len() as u32;
    batches.clear();
    for (i, flags) in flags.enumerate().step_by(3) {
        let material = if flags & render_mesh::UNLIT != 0 {
            Material::Forward
        } else if flags & render_mesh::WATER != 0 {
            Material::Water
        } else {
            Material::Surface
        };
        let end = (i as u32 + 3).min(count);
        if let Some(last) = batches.last_mut().filter(|last| last.material == material) {
            last.vertices.end = end;
        } else {
            batches.push(Batch {
                material,
                vertices: i as u32..end,
            });
        }
    }
}

pub(super) fn camera_rays(matrix: Mat4) -> Option<[f32; 16]> {
    // Inverting in f64 avoids amplifying far-plane depth error into world-space lighting.
    let matrix = matrix.as_dmat4();
    let inverse = matrix.inverse();
    let homogeneous = inverse * glam::DVec4::new(0.0, 0.0, 1.0, 0.0);
    if !homogeneous.is_finite() || homogeneous.w.abs() < 1e-12 {
        return None;
    }
    let eye = homogeneous.truncate() / homogeneous.w;
    let ray = |x, y| {
        let p = inverse.project_point3(glam::DVec3::new(x, y, 0.0));
        (p - eye) / (matrix * p.extend(1.0)).w
    };
    let middle = ray(0.0, 0.0);
    let mut result = [0.0; 16];
    for (i, v) in [middle, ray(1.0, 0.0) - middle, ray(0.0, 1.0) - middle, eye]
        .iter()
        .enumerate()
    {
        result[i * 4..i * 4 + 3].copy_from_slice(&v.as_vec3().to_array());
    }
    result.iter().all(|v| v.is_finite()).then_some(result)
}
