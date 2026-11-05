use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; specialised scenery must preserve cutouts, water, emission, wind and weather"]
fn scenery_shader_preserves_materials_and_surface_flags() {
    use crate::render_mesh::*;
    let mut vram = Vram::default();
    vram.images.push(tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [16, 16],
    });
    for y in 0..16 {
        for x in 0..16 {
            vram.words[y * 1024 + x] = [0, 0x7fff, 0x03e0, 0x801f][(x / 4 + y / 4) % 4];
        }
    }
    let (mut renderer, target) = fixture(&vram);
    renderer
        .materials
        .set_emissions(std::collections::BTreeMap::from([(
            crate::visuals::catalog::Region {
                page: 256,
                palette: 0,
                rect: [0, 0, 16, 16],
            },
            [2.0, 0.8, 0.3, 0.65],
        )]));
    let mut vertices = Vec::new();
    for (i, extra) in [
        0,
        TRANSLUCENT,
        EMITTER,
        FOLIAGE | FLUTTER | (180 << WIND_WEIGHT_SHIFT),
        WATER,
        WATER | WATERFALL,
        UNLIT,
        SNOW | WEAPON_METAL | WEAPON_COAT,
    ]
    .into_iter()
    .enumerate()
    {
        let center = Vec3::new(
            (i % 4) as f32 * 900.0 - 1350.0,
            0.0,
            (i / 4) as f32 * 900.0 - 450.0,
        );
        let points = [
            Vec3::new(-400.0, 0.0, -400.0),
            Vec3::new(400.0, 0.0, -400.0),
            Vec3::new(-400.0, 0.0, 400.0),
            Vec3::new(400.0, 0.0, 400.0),
        ];
        vertices.extend([0, 1, 2, 1, 3, 2].map(|j| Vertex {
            position: (center + points[j]).to_array(),
            normal: Vec3::NEG_Y.to_array(),
            color: [0.6, 0.8, 0.7],
            uv: [
                (points[j].x + 400.0) * 15.0 / 800.0,
                (points[j].z + 400.0) * 15.0 / 800.0,
            ],
            page: 256,
            flags: TEXTURED | SCENERY | DOUBLE_SIDED | FOG | extra,
            opacity: 1.0,
            ..Vertex::zeroed()
        }));
    }
    let mut terrain = renderer.upload(&vertices);
    assert!(terrain.scenery);
    let actor = renderer.upload(&rectangle(1000, 90, None, false, false, false));
    assert!(!actor.scenery);
    let camera = Mat4::orthographic_rh(-2200.0, 2200.0, -2200.0, 2200.0, 1.0, 20000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -5000.0, -1500.0), Vec3::ZERO, Vec3::NEG_Y);
    let view = target.create_view(&Default::default());
    let light = crate::lighting::Light {
        position: Vec3::new(-1500.0, -1800.0, 2000.0),
        color: [1.0, 0.4, 0.2],
        radius: 12000.0,
        flame: false,
        emission_radius: 0.0,
    };
    let capture = |renderer: &mut Renderer, terrain: &Mesh| {
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[terrain], &[&actor]),
                None,
                &[light],
                &[terrain, &actor],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.water.enabled = true;
    graphics.motion.wind = 80;
    graphics.weapon_reflections = 100;
    graphics.surface_detail = 100;
    for samples in renderer.sample_counts.clone() {
        graphics.samples = samples;
        for (mode, filtering, hdr) in [
            (crate::settings::WeatherMode::Original, 0, false),
            (crate::settings::WeatherMode::Original, 2, true),
            (crate::settings::WeatherMode::Snow, 1, false),
            (crate::settings::WeatherMode::Rain, 2, true),
        ] {
            graphics.weather.mode = mode;
            graphics.set_filtering(filtering);
            graphics.post.enabled = hdr;
            renderer.configure(&graphics);
            renderer.weather = crate::visuals::weather::SurfaceWeather {
                snow: 0.8,
                wet: 0.7,
                puddles: 0.5,
                time: 10.0,
                flash: 0.3,
            };
            for time in [0.0, 1.5] {
                renderer.visual_time = time;
                terrain.scenery = false;
                let expected = capture(&mut renderer, &terrain);
                assert!(expected.chunks_exact(4).any(|p| p[..3] != [0; 3]));
                terrain.scenery = true;
                assert_eq!(
                    capture(&mut renderer, &terrain),
                    expected,
                    "samples {samples}, weather {mode:?}, time {time}"
                );
            }
        }
    }
}
