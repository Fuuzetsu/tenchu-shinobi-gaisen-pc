use super::{
    draw::{Frame, GpuVertex},
    raster::Blend,
};
use std::{collections::BTreeMap, sync::Arc};
use wgpu::util::DeviceExt;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

struct Image {
    bind: wgpu::BindGroup,
    filtered: bool,
}

struct Renderer {
    layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::RenderPipeline; 6],
    samplers: [wgpu::Sampler; 2],
    images: BTreeMap<String, Image>,
    vertices: Option<wgpu::Buffer>,
    frame: Option<Arc<Frame>>,
    room: wgpu::Buffer,
}

pub fn register(renderer: &mut egui_wgpu::Renderer, format: wgpu::TextureFormat) {
    renderer.callback_resources.insert(TargetFormat(format));
}

struct TargetFormat(wgpu::TextureFormat);

fn create_renderer(device: &wgpu::Device, format: wgpu::TextureFormat) -> Renderer {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Frontend images"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
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
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Frontend images"),
        bind_group_layouts: &[&layout],
        push_constant_ranges: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Frontend images"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}",
                include_str!("gpu.wgsl"),
                include_str!("room.wgsl")
            )
            .into(),
        ),
    });
    let pipelines = std::array::from_fn(|i| {
        use wgpu::{BlendComponent, BlendFactor as F, BlendOperation as O};
        let component = match i {
            2 => BlendComponent {
                src_factor: F::Constant,
                dst_factor: F::Constant,
                operation: O::Add,
            },
            3 => BlendComponent {
                src_factor: F::One,
                dst_factor: F::One,
                operation: O::Add,
            },
            4 => BlendComponent {
                src_factor: F::One,
                dst_factor: F::One,
                operation: O::ReverseSubtract,
            },
            5 => BlendComponent {
                src_factor: F::Constant,
                dst_factor: F::One,
                operation: O::Add,
            },
            _ => BlendComponent::REPLACE,
        };
        let constants = [("srgb_target".into(), f64::from(format.is_srgb()))].into();
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Frontend sprite blend"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x3, 3 => Float32x4, 4 => Float32] }], },
            fragment: Some(wgpu::FragmentState { module: &shader,
                entry_point: Some(match i { 0 => "fs_all", 1 => "fs_opaque", _ => "fs_semi" }),
                compilation_options: wgpu::PipelineCompilationOptions { constants: &constants, ..Default::default() },
                targets: &[Some(wgpu::ColorTargetState { format,
                    blend: Some(wgpu::BlendState { color: component, alpha: BlendComponent::REPLACE }),
                    write_mask: wgpu::ColorWrites::ALL })], }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview: None, cache: None,
        })
    });
    let samplers = [wgpu::FilterMode::Nearest, wgpu::FilterMode::Linear].map(|filter| {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Frontend texture sampling"),
            mag_filter: filter,
            min_filter: filter,
            ..Default::default()
        })
    });
    Renderer {
        layout,
        pipelines,
        samplers,
        images: BTreeMap::new(),
        vertices: None,
        frame: None,
        room: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shop window lighting"),
            contents: bytemuck::bytes_of(&super::room::Lighting::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }),
    }
}

pub enum Paint {
    Frontend(Arc<Frame>),
    Models(Arc<Frame>),
}

#[derive(Default)]
struct Renderers(BTreeMap<u8, Renderer>);

impl Paint {
    fn frame(&self) -> (u8, &Arc<Frame>) {
        match self {
            Self::Frontend(frame) => (0, frame),
            Self::Models(frame) => (1, frame),
        }
    }
}

impl egui_wgpu::CallbackTrait for Paint {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let (slot, frame) = self.frame();
        if resources.get::<Renderers>().is_none() {
            resources.insert(Renderers::default());
        }
        if !resources.get::<Renderers>().unwrap().0.contains_key(&slot) {
            let format = resources
                .get::<TargetFormat>()
                .expect("Frontend format registered")
                .0;
            resources
                .get_mut::<Renderers>()
                .unwrap()
                .0
                .insert(slot, create_renderer(device, format));
        }
        let renderer = resources
            .get_mut::<Renderers>()
            .unwrap()
            .0
            .get_mut(&slot)
            .unwrap();
        if renderer
            .frame
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, frame))
        {
            return Vec::new();
        }
        renderer
            .images
            .retain(|id, _| frame.images.contains_key(id));
        queue.write_buffer(&renderer.room, 0, bytemuck::bytes_of(&frame.room));
        for (id, image) in &frame.images {
            let filtered = frame.filtered.contains(id);
            if renderer
                .images
                .get(id)
                .is_some_and(|old| old.filtered == filtered)
            {
                continue;
            }
            let [width, height] = image.size;
            let texture = device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("Frontend artwork"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &image.rgba,
            );
            let view = texture.create_view(&Default::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Frontend artwork"),
                layout: &renderer.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &renderer.samplers[usize::from(filtered)],
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: renderer.room.as_entire_binding(),
                    },
                ],
            });
            renderer.images.insert(id.clone(), Image { bind, filtered });
        }
        renderer.vertices = (!frame.vertices.is_empty()).then(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frontend triangles"),
                contents: bytemuck::cast_slice(&frame.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        renderer.frame = Some(frame.clone());
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let (slot, frame) = self.frame();
        let renderer = &resources.get::<Renderers>().unwrap().0[&slot];
        let Some(vertices) = &renderer.vertices else {
            return;
        };
        pass.set_vertex_buffer(0, vertices.slice(..));
        for draw in &frame.draws {
            pass.set_bind_group(0, &renderer.images[&draw.image].bind, &[]);
            if matches!(draw.blend, Blend::Opaque) {
                pass.set_pipeline(&renderer.pipelines[0]);
                pass.draw(draw.vertices.clone(), 0..1);
                continue;
            }
            pass.set_pipeline(&renderer.pipelines[1]);
            pass.draw(draw.vertices.clone(), 0..1);
            let pipeline = match draw.blend {
                Blend::Opaque => unreachable!(),
                Blend::Average => 2,
                Blend::Add => 3,
                Blend::Subtract => 4,
                Blend::Quarter => 5,
            };
            let factor = if pipeline == 5 { 0.25 } else { 0.5 };
            pass.set_blend_constant(wgpu::Color {
                r: factor,
                g: factor,
                b: factor,
                a: 1.0,
            });
            pass.set_pipeline(&renderer.pipelines[pipeline]);
            pass.draw(draw.vertices.clone(), 0..1);
        }
    }
}
