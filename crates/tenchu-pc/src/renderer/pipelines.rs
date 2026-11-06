use super::{materials, shadows};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Options {
    filtering: u8,
    magnification: u16,
    lighting: bool,
    relief: u16,
    hdr: bool,
    weather: bool,
    rain: bool,
    water: bool,
    coarse_shadows: bool,
    adaptive_shadows: bool,
    flame_motion: bool,
}

impl Options {
    pub(super) fn new(graphics: &crate::settings::Graphics, samples: u32) -> Self {
        Self {
            filtering: if graphics.mipmaps {
                2
            } else {
                u8::from(graphics.bilinear)
            },
            magnification: graphics.magnification,
            lighting: graphics.lighting.enabled || graphics.sky.enabled,
            relief: graphics.surface_detail,
            hdr: graphics.post.enabled,
            weather: graphics.weather.mode != crate::settings::WeatherMode::Original,
            rain: graphics.weather.mode.rain(),
            water: graphics.water.enabled,
            coarse_shadows: super::wet_lighting::configured(graphics, samples, false),
            adaptive_shadows: graphics.rendering.adaptive_shadows,
            flame_motion: graphics.lighting.enabled && graphics.lighting.flame_motion > 0,
        }
    }
}

pub(super) struct Pipelines {
    pub opaque: wgpu::RenderPipeline,
    pub scenery: wgpu::RenderPipeline,
    pub effects: [wgpu::RenderPipeline; 6],
    pub visibility: wgpu::RenderPipeline,
    pub ground_blood: wgpu::RenderPipeline,
    pub snow_blood: wgpu::RenderPipeline,
}

pub(super) fn create(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    samples: u32,
) -> Pipelines {
    let make_pipeline = |blend: Option<wgpu::BlendState>,
                         fragment: &str,
                         bias: i32,
                         depth_write_enabled: bool,
                         depth_compare: wgpu::CompareFunction| {
        let targets = [Some(wgpu::ColorTargetState {
            format,
            blend,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(fragment),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &super::geometry::LAYOUTS,
            },
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled,
                depth_compare,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: bias,
                    ..Default::default()
                },
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(fragment),
                compilation_options: Default::default(),
                targets: if fragment == "depth_only" {
                    &[]
                } else {
                    &targets
                },
            }),
            multiview: None,
            cache: None,
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
    // Leave one depth unit for the color pass's strict test, preserving coplanar face order.
    Pipelines {
        opaque: pipeline,
        scenery: make_pipeline(None, "scenery", 0, true, wgpu::CompareFunction::Less),
        effects,
        visibility: make_pipeline(None, "depth_only", 1, true, wgpu::CompareFunction::Less),
        snow_blood: make_pipeline(
            Some(wgpu::BlendState::ALPHA_BLENDING),
            "snow_blood",
            -2,
            false,
            wgpu::CompareFunction::LessEqual,
        ),
        ground_blood: make_pipeline(
            Some(wgpu::BlendState::ALPHA_BLENDING),
            "ground_blood",
            -2,
            false,
            wgpu::CompareFunction::LessEqual,
        ),
    }
}

impl Options {
    pub(super) fn shader(self, device: &wgpu::Device, gl: bool) -> wgpu::ShaderModule {
        // Distinct modules also avoid wgpu 24 GL's cache ignoring pipeline overrides.
        self.module(device, gl, include_str!("blood_composite.wgsl"))
    }

    pub(super) fn blood_cache_shader(self, device: &wgpu::Device, gl: bool) -> wgpu::ShaderModule {
        self.module(device, gl, include_str!("decals/cache_shading.wgsl"))
    }

    pub(super) fn wet_shader(
        self,
        device: &wgpu::Device,
        gl: bool,
        quarter_shadows: bool,
    ) -> wgpu::ShaderModule {
        let source = format!(
            "const SCREEN_SHADOWS: bool = {}; const SCREEN_SHADOW_STEP: i32 = {};\n{}\n{}",
            quarter_shadows,
            super::wet_lighting::SHADOW_STRIDE,
            include_str!("wet_lighting.wgsl"),
            include_str!("wet_lighting/shadows.wgsl")
        );
        self.module(device, gl, &source)
    }

    fn module(self, device: &wgpu::Device, gl: bool, pass: &str) -> wgpu::ShaderModule {
        let common = format!(
            "const FLAME_MOTION: bool = {}; const ADAPTIVE_SHADOWS: bool = {}; const FILTER_MATERIALS: bool = {}; const LIGHTING_ENABLED: bool = {}; const SURFACE_DETAIL: bool = {}; const PARALLAX_ENABLED: bool = {}; const HDR_ENABLED: bool = {}; const WEATHER_ENABLED: bool = {}; const RAIN_ENABLED: bool = {}; const WATER_ENABLED: bool = {};\nconst MAX_SCENE_LIGHTS: u32 = {}u; const MAX_LIGHTS: u32 = {}u; const SHADOW_FACES: u32 = {}u; const SHADOW_FILTER_SAMPLES: u32 = {}u; const SHADOW_FILTER_TAPS: u32 = {}u; const SHADOW_FILTER_GROUP: u32 = {}u; const MAX_MIPS: u32 = {}u; const SHADOW_NEAR: f32 = {}; const RELIEF_GRADIENT_RANGE: f32 = {}; const MATERIAL_MAGNIFICATION: f32 = {}.0; const MATERIAL_FILTERING: f32 = {}.0;\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            self.flame_motion,
            self.adaptive_shadows,
            self.filtering != 0 || self.magnification != 0,
            self.lighting,
            self.relief != 0,
            self.relief == 2,
            self.hdr,
            self.weather,
            self.rain,
            self.water,
            crate::lighting::MAX_SCENE_LIGHTS,
            crate::lighting::MAX_LIGHTS,
            shadows::FACES,
            shadows::FILTER_SAMPLES,
            if self.adaptive_shadows {
                16
            } else {
                shadows::FILTER_SAMPLES
            },
            shadows::FILTER_GROUP_SAMPLES,
            materials::MAX_MIPS,
            shadows::NEAR,
            crate::visuals::relief::GRADIENT_RANGE,
            self.magnification,
            self.filtering,
            concat!(
                include_str!("blood_mask.wgsl"),
                "\n",
                include_str!("../scene.wgsl")
            ),
            include_str!("palette.wgsl"),
            include_str!("lighting.wgsl"),
            include_str!("weather.wgsl"),
            materials::shader(device),
            include_str!("relief.wgsl"),
            include_str!("water.wgsl"),
            include_str!("weapons.wgsl"),
            include_str!("foliage.wgsl"),
            if gl {
                include_str!("depth_gl.wgsl")
            } else {
                include_str!("depth.wgsl")
            }
        );
        let bounds = include_str!("shadows/bounds_query.wgsl");
        let bounds_tile = shadows::BOUNDS_TILE;
        let bounds_enabled = !gl && self.coarse_shadows;
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Scene materials and lighting"),
            source: wgpu::ShaderSource::Wgsl(format!("const COARSE_SHADOWS: bool = {bounds_enabled}; const SHADOW_BOUND_TILE: u32 = {bounds_tile}u;\n{common}\n{pass}\n{bounds}").into()),
        })
    }
}
