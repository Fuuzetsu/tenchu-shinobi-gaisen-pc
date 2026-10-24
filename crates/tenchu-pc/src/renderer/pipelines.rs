use super::{materials, shadows};
use crate::render_mesh::Vertex;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Options {
    filtering: bool,
    lighting: bool,
    relief: bool,
    hdr: bool,
    weather: bool,
}

impl From<&crate::settings::Graphics> for Options {
    fn from(graphics: &crate::settings::Graphics) -> Self {
        Self {
            filtering: graphics.bilinear || graphics.mipmaps || graphics.magnification != 0,
            lighting: graphics.lighting.enabled || graphics.sky.enabled,
            relief: graphics.surface_detail != 0,
            hdr: graphics.post.enabled,
            weather: graphics.weather.mode != crate::settings::WeatherMode::Original,
        }
    }
}

pub(super) fn create(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    samples: u32,
) -> (wgpu::RenderPipeline, [wgpu::RenderPipeline; 6]) {
    let make_pipeline = |blend: Option<wgpu::BlendState>,
                         fragment: &str,
                         bias: i32,
                         depth_write_enabled: bool,
                         depth_compare: wgpu::CompareFunction| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fragment), layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader, entry_point: Some("vertex"), compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32, 4 => Uint32, 5 => Uint32, 6 => Uint32, 7 => Float32x3, 8 => Float32],
            }],
        },
        primitive: wgpu::PrimitiveState { front_face: wgpu::FrontFace::Cw, cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float, depth_write_enabled,
            depth_compare, stencil: Default::default(),
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
    // Earlier TMD faces cover later coplanar authoring/backing faces in the original OT.
    let pipeline = make_pipeline(None, "opaque", 0, true, wgpu::CompareFunction::Less);
    let effects = std::array::from_fn(|mode| {
        if mode == 0 {
            return make_pipeline(None, "opaque", -2, true, wgpu::CompareFunction::LessEqual);
        }
        if mode == 5 {
            return make_pipeline(
                Some(wgpu::BlendState::ALPHA_BLENDING),
                "alpha",
                -2,
                false,
                wgpu::CompareFunction::LessEqual,
            );
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
        make_pipeline(
            Some(blend),
            "translucent",
            -2,
            false,
            wgpu::CompareFunction::LessEqual,
        )
    });
    (pipeline, effects)
}

impl Options {
    pub(super) fn shader(self, device: &wgpu::Device) -> wgpu::ShaderModule {
        // Distinct modules also avoid wgpu 24 GL's cache ignoring pipeline overrides.
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Original paletted textures"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "const FILTER_MATERIALS: bool = {}; const LIGHTING_ENABLED: bool = {}; const SURFACE_DETAIL: bool = {}; const HDR_ENABLED: bool = {}; const WEATHER_ENABLED: bool = {};\nconst MAX_LIGHTS: u32 = {}u; const SHADOW_FACES: u32 = {}u; const MAX_MIPS: u32 = {}u; const SHADOW_NEAR: f32 = {}; const RELIEF_GRADIENT_RANGE: f32 = {};\n{}\n{}\n{}\n{}\n{}",
                    self.filtering, self.lighting, self.relief, self.hdr, self.weather,
                    crate::lighting::MAX_LIGHTS,
                    shadows::FACES,
                    materials::MAX_MIPS,
                    shadows::NEAR,
                    crate::visuals::relief::GRADIENT_RANGE,
                    include_str!("../scene.wgsl"),
                    include_str!("palette.wgsl"),
                    include_str!("lighting.wgsl"),
                    materials::shader(device),
                    include_str!("relief.wgsl")
                )
                .into(),
            ),
        })
    }
}
