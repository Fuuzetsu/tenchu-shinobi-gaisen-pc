use crate::render_mesh::Vertex;

pub fn create(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    samples: u32,
) -> (wgpu::RenderPipeline, [wgpu::RenderPipeline; 5]) {
    let make_pipeline = |blend: Option<wgpu::BlendState>,
                         fragment: &str,
                         bias: i32,
                         depth_write_enabled: bool| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fragment), layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader, entry_point: Some("vertex"), compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32, 4 => Uint32, 5 => Uint32],
            }],
        },
        primitive: wgpu::PrimitiveState { front_face: wgpu::FrontFace::Cw, cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float, depth_write_enabled,
            depth_compare: wgpu::CompareFunction::LessEqual, stencil: Default::default(),
            bias: wgpu::DepthBiasState { constant: bias, ..Default::default() },
        }),
        multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: shader, entry_point: Some(fragment), compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview: None, cache: None,
        })
    };
    let pipeline = make_pipeline(None, "opaque", 0, true);
    let effects = std::array::from_fn(|mode| {
        if mode == 0 {
            return make_pipeline(None, "opaque", -2, true);
        }
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: if matches!(mode, 1 | 4) {
                    wgpu::BlendFactor::Constant
                } else {
                    wgpu::BlendFactor::One
                },
                dst_factor: if mode == 1 {
                    wgpu::BlendFactor::Constant
                } else {
                    wgpu::BlendFactor::One
                },
                operation: if mode == 3 {
                    wgpu::BlendOperation::ReverseSubtract
                } else {
                    wgpu::BlendOperation::Add
                },
            },
            alpha: wgpu::BlendComponent::REPLACE,
        };
        make_pipeline(Some(blend), "translucent", -2, false)
    });
    (pipeline, effects)
}
