use super::*;
use crate::render_mesh;
use tenchu_assets::model::{Corner, Face, Geometry, Texture};
use tenchu_core::math::Matrix;

fn rectangle(
    z: i16,
    color: u8,
    texture: Option<Texture>,
    blend: bool,
    reversed: bool,
    double_sided: bool,
) -> Vec<Vertex> {
    let mut corners = (0..4)
        .map(|vertex| Corner {
            vertex,
            normal: None,
            color: [color; 3],
            uv: [0; 2],
        })
        .collect::<Vec<_>>();
    if reversed {
        corners.swap(1, 2);
    }
    let face = Face {
        mode: 0x28,
        flags: 1,
        corners,
        texture,
        double_sided,
        translucent: blend,
    };
    let geometry = Geometry {
        vertices: vec![
            [-600, -600, z],
            [600, -600, z],
            [-600, 600, z],
            [600, 600, z],
        ],
        normals: vec![],
        faces: vec![face],
    };
    let mut vertices = Vec::new();
    render_mesh::append_face(
        &mut vertices,
        &geometry,
        &geometry.faces[0],
        Matrix::default(),
        true,
    );
    vertices
}

fn read_pixels(renderer: &Renderer, target: &wgpu::Texture) -> Vec<u8> {
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Renderer regression readback"),
        size: 256 * 64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(64),
            },
        },
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap();
        });
    renderer.device.poll(wgpu::Maintain::Wait);
    receive.recv().unwrap().unwrap();
    let pixels = buffer.slice(..).get_mapped_range();
    pixels.to_vec()
}

fn read_pixel(renderer: &Renderer, target: &wgpu::Texture) -> [u8; 4] {
    let pixels = read_pixels(renderer, target);
    pixels[24 * 256 + 24 * 4..24 * 256 + 25 * 4]
        .try_into()
        .unwrap()
}

fn fixture(vram: &Vram) -> (Renderer, wgpu::Texture) {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("No graphics adapter");
    let renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        [64; 2],
        vram,
    ))
    .unwrap();
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Renderer regression"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    (renderer, target)
}

#[test]
#[ignore = "requires a graphics adapter; HDR bloom spreads beyond highlights, UI bypasses grading, and MSAA depth resolves correctly"]
fn hdr_finishing_preserves_ui_and_restores_original_rendering() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut vertices = rectangle(1000, 96, None, false, false, false);
    let mut highlight = rectangle(900, 255, None, false, false, false);
    for vertex in &mut highlight {
        vertex.position[0] *= 0.15;
        vertex.position[1] *= 0.15;
        vertex.color = [2.0; 3];
    }
    vertices.extend(highlight);
    let mesh = renderer.upload(&vertices);
    let mut hud = rectangle(1000, 255, None, false, false, false);
    for vertex in &mut hud {
        vertex.position = [
            vertex.position[0] / 30.0 - 90.0,
            vertex.position[1] / 30.0 - 90.0,
            0.01,
        ];
        vertex.color = [0.2, 0.6, 0.9];
        vertex.flags = render_mesh::SCREEN | render_mesh::UI;
    }
    let effects = renderer.upload_effects(render_effects::Geometry {
        batches: vec![render_effects::Batch {
            blend: 0,
            range: 0..hud.len() as u32,
        }],
        vertices: hud,
    });
    renderer
        .draw(&view, camera, &[&mesh], Some(&effects))
        .unwrap();
    let original = read_pixels(&renderer, &target);
    let pixel = |pixels: &[u8], x: usize, y: usize| {
        <[u8; 4]>::try_from(&pixels[y * 256 + x * 4..y * 256 + x * 4 + 4]).unwrap()
    };
    let mut graphics = crate::settings::Graphics::default();
    graphics.post.enabled = true;
    graphics.post.filmic = false;
    graphics.post.bloom = 0;
    graphics.post.occlusion = 0;
    graphics.post.fog_tint = 0;
    for samples in renderer.sample_counts.clone() {
        graphics.samples = samples;
        graphics.post.bloom = 0;
        renderer.configure(&graphics);
        renderer
            .draw(&view, camera, &[&mesh], Some(&effects))
            .unwrap();
        let neutral = read_pixels(&renderer, &target);
        assert_eq!(
            pixel(&neutral, 8, 8),
            pixel(&original, 8, 8),
            "HUD with {samples} samples"
        );
        assert!(pixel(&neutral, 44, 32)[0].abs_diff(96) <= 1);
        graphics.post.bloom = 100;
        renderer.configure(&graphics);
        renderer
            .draw(&view, camera, &[&mesh], Some(&effects))
            .unwrap();
        let bloom = read_pixels(&renderer, &target);
        assert!(
            pixel(&bloom, 44, 32)[0] > pixel(&neutral, 44, 32)[0] + 5,
            "highlight halo with {samples} samples"
        );
        assert_eq!(pixel(&bloom, 8, 8), pixel(&original, 8, 8));
    }
    graphics.post.saturation = 0;
    graphics.post.contrast = 120;
    graphics.post.filmic = true;
    renderer.configure(&graphics);
    renderer
        .draw(&view, camera, &[&mesh], Some(&effects))
        .unwrap();
    assert_eq!(
        pixel(&read_pixels(&renderer, &target), 8, 8),
        pixel(&original, 8, 8)
    );
    let mut distant = rectangle(25000, 96, None, false, false, false);
    for vertex in &mut distant {
        vertex.position[0] *= 10.3;
        vertex.position[1] *= 10.3;
    }
    let distant = renderer.upload(&distant);
    graphics.post.bloom = 0;
    graphics.post.fog_tint = 1;
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&distant], None).unwrap();
    let pixels = read_pixels(&renderer, &target);
    let background = pixel(&pixels, 0, 0);
    assert!(
        pixels
            .chunks_exact(4)
            .all(|p| p.iter().zip(background).all(|(a, b)| a.abs_diff(b) <= 1)),
        "fog and sky must meet without dark MSAA fringes"
    );
    renderer.configure(&Default::default());
    renderer
        .draw(&view, camera, &[&mesh], Some(&effects))
        .unwrap();
    assert_eq!(read_pixels(&renderer, &target), original);
}

#[test]
#[ignore = "requires a graphics adapter; contact shading darkens nearby corners and leaves isolated flat surfaces unchanged"]
fn screen_space_contact_shading_responds_to_geometry() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let camera = Mat4::perspective_rh(0.9, 1.0, 50.0, 250_000.0)
        * Mat4::look_at_rh(
            glam::Vec3::new(0.0, -1800.0, -3500.0),
            glam::Vec3::new(0.0, 0.0, 2000.0),
            glam::Vec3::NEG_Y,
        );
    let mut vertices = Vec::new();
    for points in [
        [
            [-5000.0, 0.0, 0.0],
            [5000.0, 0.0, 0.0],
            [-5000.0, 0.0, 7000.0],
            [5000.0, 0.0, 7000.0],
        ],
        [
            [-3000.0, -2500.0, 2500.0],
            [3000.0, -2500.0, 2500.0],
            [-3000.0, 0.0, 2500.0],
            [3000.0, 0.0, 2500.0],
        ],
    ] {
        let mut quad = rectangle(1000, 153, None, false, false, true);
        for (vertex, index) in quad.iter_mut().zip([0, 1, 2, 1, 3, 2]) {
            vertex.position = points[index];
            vertex.flags = render_mesh::DOUBLE_SIDED;
        }
        vertices.extend(quad);
    }
    let mesh = renderer.upload(&vertices);
    let mut graphics = crate::settings::Graphics::default();
    graphics.post.enabled = true;
    graphics.post.filmic = false;
    graphics.post.bloom = 0;
    graphics.post.occlusion = 0;
    graphics.post.fog_tint = 0;
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let flat = read_pixels(&renderer, &target);
    graphics.post.occlusion = 100;
    graphics.post.occlusion_radius = 15;
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let shaded = read_pixels(&renderer, &target);
    let pairs: Vec<_> = flat.chunks_exact(4).zip(shaded.chunks_exact(4)).collect();
    assert!(
        pairs.iter().filter(|(a, b)| a[0] > b[0] + 3).count() > 20,
        "corners acquire contact shading"
    );
    assert!(
        pairs.iter().filter(|(a, b)| a[0] == 153 && a == b).count() > 200,
        "flat regions remain unshaded"
    );
    assert!(pairs.iter().all(|(a, b)| b[0] <= a[0]));
}

#[test]
#[ignore = "requires a graphics adapter; replacements retain higher-resolution detail, use their own cutouts and revert without disturbing original animation"]
fn replacement_textures_reload_cutouts_and_revert() {
    use crate::visuals::{catalog, pack::Pack, pixels::Image};
    use std::{collections::BTreeMap, sync::Arc};
    let mut vram = Vram::default();
    vram.images.push(tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [1, 1],
    });
    vram.words[0] = 0x7fff;
    let id = catalog::image_id([1, 1], &[255; 4]);
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut vertices = rectangle(
        1000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        false,
        false,
        false,
    );
    for vertex in &mut vertices {
        vertex.uv = [vertex.position[0] / 1200.0, vertex.position[1] / 1200.0];
    }
    let mesh = renderer.upload(&vertices);
    let mut image = Image {
        size: [4, 4],
        rgba: (0..16)
            .flat_map(|n| {
                if n % 2 == 0 {
                    [255, 0, 0, 255]
                } else {
                    [0, 255, 0, 255]
                }
            })
            .collect(),
    };
    let pack = |image: Image| Pack {
        textures: BTreeMap::from([(id.clone(), Arc::new(image))]),
        ..Default::default()
    };
    renderer.set_asset_pack(&pack(image.clone()));
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let colors = read_pixels(&renderer, &target);
    assert!(colors.chunks_exact(4).any(|p| p == [255, 0, 0, 255]));
    assert!(colors.chunks_exact(4).any(|p| p == [0, 255, 0, 255]));
    image.rgba.fill(0);
    renderer.set_asset_pack(&pack(image.clone()));
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[0; 3]);
    vram.words[0] = 31;
    renderer.update_vram(&vram);
    let mut graphics = crate::settings::Graphics::default();
    graphics.asset_pack = false;
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert!(
        read_pixels(&renderer, &target)
            .chunks_exact(4)
            .any(|p| p == [255, 0, 0, 255])
    );
    vram.words[0] = 0;
    renderer.update_vram(&vram);
    image.rgba = [0, 0, 255, 255].repeat(16);
    renderer.set_asset_pack(&pack(image));
    graphics.asset_pack = true;
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[0, 0, 255]);
}

#[test]
#[ignore = "requires a graphics adapter; minification averages texels, anisotropy preserves detail across the short axis, and VRAM animation stays live"]
fn decoded_mips_filter_without_blurring_the_short_axis() {
    let mut vram = Vram::default();
    vram.images.push(tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [256, 16],
    });
    for y in 0..16 {
        for x in 0..256 {
            vram.words[y * 1024 + x] = if x % 2 == 0 { 0x7fff } else { 0x8000 };
        }
    }
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut vertices = rectangle(
        2000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        false,
        false,
        false,
    );
    for vertex in &mut vertices {
        vertex.uv = [
            (vertex.position[0] + vertex.position[1] + 1200.0) / 2400.0 * 255.0,
            0.0,
        ];
    }
    let mesh = renderer.upload(&vertices);
    let mut options = crate::settings::Graphics::default();
    options.set_filtering(2);
    renderer.configure(&options);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let averaged = read_pixel(&renderer, &target);
    assert!(
        averaged[..3].iter().all(|&c| c.abs_diff(128) <= 2),
        "{averaged:?}"
    );
    for y in 0..16 {
        for x in 0..256 {
            vram.words[y * 1024 + x] = if y % 2 == 0 { 0x7fff } else { 0x8000 };
        }
    }
    renderer.update_vram(&vram);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let blurred = read_pixel(&renderer, &target);
    assert!(blurred[0].abs_diff(128) <= 2, "{blurred:?}");
    options.set_filtering(6);
    renderer.configure(&options);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let detailed = read_pixel(&renderer, &target);
    assert!(detailed[0] > 240, "{detailed:?}");
    for word in &mut vram.words[..16 * 1024] {
        *word = 31;
    }
    renderer.update_vram(&vram);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[255, 0, 0]);
}

#[test]
#[ignore = "requires a graphics adapter; checks moving point-light shadows, alpha cutouts, second-light layers and unchanged defaults"]
fn point_lights_occlude_and_move_without_shadowing_texture_holes() {
    use crate::lighting::Light;
    use glam::Vec3;
    let mut vram = Vram::default();
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let matrix = Mat4::orthographic_rh(-1000.0, 1000.0, -1000.0, 1000.0, 1.0, 5000.0)
        * Mat4::look_to_rh(Vec3::new(0.0, 0.0, -2000.0), Vec3::Z, Vec3::NEG_Y);
    let quad = |center: Vec3, size: f32, flags: u32| {
        let points = [
            center + Vec3::new(-size, -size, 0.0),
            center + Vec3::new(size, -size, 0.0),
            center + Vec3::new(-size, size, 0.0),
            center + Vec3::new(size, size, 0.0),
        ];
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            material: 0,
            normal: [0.0; 3],
            position: points[i].to_array(),
            color: [0.5; 3],
            uv: [0.0; 2],
            page: 256,
            palette: 0,
            flags: flags | render_mesh::DOUBLE_SIDED,
        })
    };
    let receiver = renderer.upload(&quad(Vec3::ZERO, 1000.0, 0));
    let sample = Vec3::new(-234.375, -234.375, 0.0);
    let light = Light {
        position: Vec3::new(1000.0, -800.0, -1000.0),
        color: [1.0; 3],
        radius: 5000.0,
    };
    let blocker_position = (sample + light.position) * 0.5;
    let blocker = renderer.upload(&quad(blocker_position, 140.0, 0));
    let transparent = renderer.upload(&quad(blocker_position, 140.0, render_mesh::TEXTURED));
    let mixed = renderer.upload(&quad(
        blocker_position,
        140.0,
        render_mesh::TEXTURED | render_mesh::TRANSLUCENT,
    ));
    let draw = |renderer: &mut Renderer, lights: &[Light], blockers: &[&Mesh]| {
        let mut casters = vec![&receiver];
        casters.extend_from_slice(blockers);
        renderer
            .draw_lit(&view, matrix, &[&receiver], None, lights, &casters)
            .unwrap();
        read_pixel(renderer, &target)
    };
    let original = draw(&mut renderer, &[light], &[&blocker]);
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 30;
    renderer.configure(&graphics);
    let lit = draw(&mut renderer, &[light], &[]);
    let shadowed = draw(&mut renderer, &[light], &[&blocker]);
    assert!(lit[0] > shadowed[0] + 25, "{lit:?} versus {shadowed:?}");
    assert!((i32::from(shadowed[0]) - 38).abs() <= 2, "{shadowed:?}");
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), lit);
    vram.words[0] = 0x7fff;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    assert_eq!(draw(&mut renderer, &[light], &[&mixed]), shadowed);
    vram.words[0] |= 0x8000;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&mixed]), lit);

    for (alpha, expected) in [(255, shadowed), (128, lit), (0, lit)] {
        let pack = crate::visuals::pack::Pack {
            textures: std::collections::BTreeMap::from([(
                crate::visuals::catalog::image_id([1, 1], &[0; 4]),
                std::sync::Arc::new(crate::visuals::pixels::Image {
                    size: [4, 4],
                    rgba: [255, 255, 255, alpha].repeat(16),
                }),
            )]),
            ..Default::default()
        };
        renderer.set_asset_pack(&pack);
        assert_eq!(draw(&mut renderer, &[light], &[&mixed]), expected);
    }
    renderer.set_asset_pack(&crate::visuals::pack::Pack::default());
    let region = crate::visuals::catalog::Region {
        page: 256,
        palette: 0,
        rect: [0, 0, 1, 1],
    };
    renderer
        .materials
        .set_emissions(std::collections::BTreeMap::from([(
            region,
            [2.0, 2.0, 2.0, 0.65],
        )]));
    vram.words[0] = 0x7fff;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), lit);
    graphics.lighting.lamps = false;
    renderer.configure(&graphics);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    graphics.lighting.lamps = true;
    renderer.configure(&graphics);
    vram.words[0] = 31;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    renderer.materials.set_emissions(Default::default());

    let moved = Light {
        position: Vec3::new(-1000.0, -800.0, -1000.0),
        ..light
    };
    assert!(draw(&mut renderer, &[moved], &[&blocker])[0] > shadowed[0] + 25);
    let inactive = Light {
        color: [0.0; 3],
        ..moved
    };
    assert_eq!(
        draw(&mut renderer, &[inactive, light], &[&blocker]),
        shadowed
    );
    assert_eq!(draw(&mut renderer, &[inactive, light], &[]), lit);
    for resolution in [256, 1024, 2048] {
        graphics.lighting.resolution = resolution;
        graphics.samples = *renderer.sample_counts.last().unwrap();
        renderer.configure(&graphics);
        assert_eq!(draw(&mut renderer, &[light], &[&blocker]), shadowed);
    }
    graphics.lighting.resolution = 256;
    renderer.configure(&graphics);
    let mut full_budget = vec![inactive; crate::lighting::MAX_LIGHTS];
    *full_budget.last_mut().unwrap() = light;
    assert_eq!(draw(&mut renderer, &full_budget, &[&blocker]), lit);
    assert_eq!(draw(&mut renderer, &full_budget, &[]), lit);
    full_budget.swap(0, crate::lighting::MAX_LIGHTS - 1);
    assert_eq!(draw(&mut renderer, &full_budget, &[&blocker]), shadowed);
    graphics.lighting.shadow_count = 0;
    renderer.configure(&graphics);
    assert_eq!(draw(&mut renderer, &full_budget, &[&blocker]), lit);
    graphics.lighting.shadow_count = 2;
    renderer.configure(&graphics);
    draw(&mut renderer, &[light], &[&blocker]);
    let hard = read_pixels(&renderer, &target);
    graphics.lighting.softness = 3;
    renderer.configure(&graphics);
    let filtered = draw(&mut renderer, &[light], &[&blocker]);
    assert!(filtered[0] < lit[0] - 20);
    assert_ne!(read_pixels(&renderer, &target), hard);
    graphics.lighting.softness = 0;
    renderer.configure(&graphics);
    draw(&mut renderer, &[], &[]);
    assert_eq!(draw(&mut renderer, &[light], &[&blocker]), shadowed);
    graphics.lighting.shadows = false;
    renderer.configure(&graphics);
    assert_eq!(draw(&mut renderer, &[light], &[&blocker]), lit);
    renderer.configure(&crate::settings::Graphics::default());
    assert_eq!(draw(&mut renderer, &[light], &[&blocker]), original);
}

#[test]
#[ignore = "requires TENCHU_DISC and a graphics adapter; actual animated models and weapon meshes must cast distinct shadows"]
fn live_character_and_weapon_meshes_cast_shadows() {
    use crate::{lighting::Light, session::Options};
    use glam::Vec3;
    use tenchu_assets::game::GameAssets;
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    for character in 0..2 {
        let mut game = Options {
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        for tick in 0..45 {
            game.step(if tick == 35 {
                tenchu_core::input::SQUARE
            } else {
                0
            })
            .unwrap();
        }
        let before = crate::session::tests::state(&game);
        let (mut renderer, target) = fixture(&game.mission.vram);
        let mut graphics = crate::settings::Graphics::default();
        graphics.lighting.enabled = true;
        graphics.lighting.ambient = 25;
        graphics.lighting.intensity = 250;
        graphics.lighting.resolution = 1024;
        renderer.configure(&graphics);
        let origin = Vec3::from(game.player().position.map(|v| v as f32));
        let points = [
            Vec3::new(-4000.0, 20.0, -4000.0),
            Vec3::new(4000.0, 20.0, -4000.0),
            Vec3::new(-4000.0, 20.0, 4000.0),
            Vec3::new(4000.0, 20.0, 4000.0),
        ];
        let receiver = renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
            material: 0,
            normal: [0.0; 3],
            position: (origin + points[i]).to_array(),
            color: [0.5; 3],
            uv: [0.0; 2],
            page: 0,
            palette: 0,
            flags: render_mesh::DOUBLE_SIDED,
        }));
        let matrix = Mat4::orthographic_rh(-4000.0, 4000.0, -4000.0, 4000.0, 1.0, 20000.0)
            * Mat4::look_at_rh(origin - Vec3::Y * 9000.0, origin, Vec3::Z);
        let lights = [Light {
            position: origin + Vec3::new(2500.0, -4000.0, 1500.0),
            radius: 15000.0,
            color: [1.0; 3],
        }];
        let view = target.create_view(&Default::default());
        let draw = |renderer: &mut Renderer, mesh: Option<&Mesh>| {
            let casters: Vec<_> = mesh.into_iter().collect();
            renderer
                .draw_lit(&view, matrix, &[&receiver], None, &lights, &casters)
                .unwrap();
            read_pixels(renderer, &target)
        };
        let lit = draw(&mut renderer, None);
        let armed = renderer.upload(&render_mesh::shadow_models(&game, None));
        let armed_pixels = draw(&mut renderer, Some(&armed));
        assert_eq!(crate::session::tests::state(&game), before);
        assert!(
            lit.chunks_exact(4)
                .zip(armed_pixels.chunks_exact(4))
                .filter(|(a, b)| a[0] > b[0].saturating_add(15))
                .count()
                > 20,
            "Character {character} casts no substantial silhouette"
        );
        let weapons = std::mem::take(&mut game.roster.get_mut(game.player_id).weapons);
        assert!(weapons.iter().any(Option::is_some));
        let unarmed = renderer.upload(&render_mesh::shadow_models(&game, None));
        let unarmed_pixels = draw(&mut renderer, Some(&unarmed));
        assert!(
            unarmed_pixels
                .chunks_exact(4)
                .zip(armed_pixels.chunks_exact(4))
                .any(|(a, b)| a[0] > b[0].saturating_add(10)),
            "Character {character}'s weapon contributes no shadow"
        );
    }
}

#[test]
#[ignore = "requires a graphics adapter; directional shadows work without point lights, stay stable under small camera motion, and preserve original defaults"]
fn directional_shadows_cover_scenery_without_point_sources() {
    use glam::Vec3;
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let quad = |y: f32, half: f32| {
        let points = [
            Vec3::new(-half, y, -half),
            Vec3::new(half, y, -half),
            Vec3::new(-half, y, half),
            Vec3::new(half, y, half),
        ];
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: points[i].to_array(),
            color: [0.5; 3],
            uv: [0.0; 2],
            page: 0,
            palette: 0,
            flags: render_mesh::DOUBLE_SIDED,
            material: 0,
            normal: [0.0; 3],
        })
    };
    let ground = renderer.upload(&quad(0.0, 1000.0));
    let roof = renderer.upload(&quad(-500.0, 400.0));
    let camera = |x: f32| {
        Mat4::orthographic_rh(-1000.0, 1000.0, -1000.0, 1000.0, 1.0, 5000.0)
            * Mat4::look_at_rh(Vec3::new(x, -3000.0, 0.0), Vec3::new(x, 0.0, 0.0), Vec3::Z)
    };
    let draw = |renderer: &mut Renderer, x| {
        renderer
            .draw_lit(&view, camera(x), &[&ground], None, &[], &[&ground, &roof])
            .unwrap();
        read_pixel(renderer, &target)
    };
    let original = draw(&mut renderer, 123.0);
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 20;
    graphics.lighting.shadow_count = 0;
    graphics.lighting.directional = crate::settings::Directional {
        intensity: 100,
        color: 0,
        azimuth: 0,
        elevation: 85,
        radius: 10,
        resolution: 2048,
    };
    renderer.configure(&graphics);
    let shadow = draw(&mut renderer, 123.0);
    assert_eq!(draw(&mut renderer, 123.01), shadow);
    graphics.lighting.shadows = false;
    renderer.configure(&graphics);
    let lit = draw(&mut renderer, 123.0);
    assert!(lit[0] > shadow[0] + 80, "{lit:?} versus {shadow:?}");
    graphics.lighting.directional.color = 1;
    renderer.configure(&graphics);
    let cool = draw(&mut renderer, 123.0);
    assert!(cool[2] > cool[0] + 25);
    graphics.lighting.directional.intensity = 0;
    renderer.configure(&graphics);
    assert_eq!(draw(&mut renderer, 123.0), shadow);
    renderer.configure(&crate::settings::Graphics::default());
    assert_eq!(draw(&mut renderer, 123.0), original);
}

#[test]
#[ignore = "requires a graphics adapter; renders source winding, texture transparency and all four blend modes offscreen"]
fn winding_and_original_texture_blends_render_correctly() {
    let mut vram = Vram::default();
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    for (reversed, double_sided, expected) in [
        (false, false, [255; 3]),
        (true, false, [0; 3]),
        (true, true, [255; 3]),
    ] {
        let mesh = renderer.upload(&rectangle(1000, 255, None, false, reversed, double_sided));
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        assert_eq!(
            &read_pixel(&renderer, &target)[..3],
            &expected,
            "reversed {reversed}, double sided {double_sided}"
        );
    }
    let background = renderer.upload(&rectangle(1500, 102, None, false, false, false));
    let foreground = 15 | (15 << 5) | (15 << 10);
    for (blend, pixel, expected) in [
        (0, foreground | 0x8000, 123),
        (1, foreground, 123),
        (1, foreground | 0x8000, 113),
        (2, foreground | 0x8000, 225),
        (3, foreground | 0x8000, 0),
        (4, foreground | 0x8000, 133),
        (1, 0, 102),
        (1, 0x8000, 51),
    ] {
        vram.words[0] = pixel;
        renderer.update_vram(&vram);
        let vertices = rectangle(
            1000,
            128,
            Some(Texture {
                page: 256,
                palette: 0,
            }),
            blend != 0,
            false,
            false,
        );
        let count = vertices.len() as u32;
        let effects = renderer.upload_effects(render_effects::Geometry {
            vertices,
            batches: vec![render_effects::Batch {
                blend,
                range: 0..count,
            }],
        });
        renderer
            .draw(&view, camera, &[&background], Some(&effects))
            .unwrap();
        let color = read_pixel(&renderer, &target);
        assert!(
            color[..3]
                .iter()
                .all(|&v| i32::from(v).abs_diff(expected) <= 1),
            "blend {blend}, pixel {pixel:04x}: {color:?}, expected {expected}"
        );
    }
}

#[test]
#[ignore = "requires a graphics adapter; checks camera-forward fog, texture/blend handling and unfogged screen graphics"]
fn distance_fog_preserves_near_colors_and_screen_graphics() {
    let mut vram = Vram::default();
    vram.words[0] = 15 | (15 << 5) | (15 << 10) | 0x8000;
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let plane = |z: f32, texture: Option<Texture>, translucent| {
        let mut vertices = rectangle(
            1000,
            if texture.is_some() { 128 } else { 255 },
            texture,
            translucent,
            false,
            false,
        );
        for vertex in &mut vertices {
            vertex.position = vertex.position.map(|value| value * z / 1000.0);
        }
        vertices
    };
    for (z, expected) in [
        (1000.0, 255),
        (12000.0, 255),
        (15000.0, 151),
        (18000.0, 43),
        (20000.0, 0),
    ] {
        let vertices = plane(z, None, false);
        let mesh = renderer.upload(&vertices);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        let color = read_pixel(&renderer, &target);
        assert!(
            color[..3]
                .iter()
                .all(|&c| i32::from(c).abs_diff(expected) <= 1),
            "depth {z}: {color:?}"
        );

        let rotated = Matrix {
            rotation: [[0, 0, -4096], [0, 4096, 0], [4096, 0, 0]],
            translation: [5000, 0, 2000],
        };
        let vertices: Vec<_> = vertices
            .into_iter()
            .map(|mut vertex| {
                let [x, y, z] = vertex.position;
                vertex.position = [z - 2000.0, y, 5000.0 - x];
                vertex
            })
            .collect();
        let mesh = renderer.upload(&vertices);
        renderer
            .draw(&view, game_camera(rotated, 1.0), &[&mesh], None)
            .unwrap();
        assert_eq!(
            read_pixel(&renderer, &target),
            color,
            "fog must follow the camera, not world Z or radial distance"
        );
    }
    let vertices = plane(
        15000.0,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        true,
    );
    let effects = renderer.upload_effects(render_effects::Geometry {
        batches: vec![render_effects::Batch {
            blend: 2,
            range: 0..vertices.len() as u32,
        }],
        vertices,
    });
    renderer.draw(&view, camera, &[], Some(&effects)).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[73; 3]);

    let mut vertices = plane(20000.0, None, false);
    for vertex in &mut vertices {
        vertex.flags &= !render_mesh::FOG;
    }
    let mesh = renderer.upload(&vertices);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[255; 3]);

    for vertex in &mut vertices {
        vertex.flags |= render_mesh::SCREEN | render_mesh::FOG;
        vertex.position = [vertex.position[0] / 200.0, vertex.position[1] / 200.0, 0.5];
    }
    let mesh = renderer.upload(&vertices);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[255; 3]);
}

#[test]
#[ignore = "requires a graphics adapter; validates every supported MSAA mode, decoded bilinear sampling and unchanged UI"]
fn graphics_options_filter_palettes_and_resolve_supported_sample_counts() {
    let mut vram = Vram::default();
    vram.words[0] = 31;
    vram.words[1] = 31 << 10;
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut vertices = rectangle(
        1000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        false,
        false,
        false,
    );
    for vertex in &mut vertices {
        vertex.uv = [0.5, 0.0];
    }
    let mesh = renderer.upload(&vertices);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[255, 0, 0]);
    let mut options = crate::settings::Graphics {
        bilinear: true,
        ..Default::default()
    };
    for samples in renderer.sample_counts.clone() {
        options.samples = samples;
        renderer.configure(&options);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        let pixel = read_pixel(&renderer, &target);
        assert!(
            pixel[0].abs_diff(128) <= 1 && pixel[1] == 0 && pixel[2].abs_diff(128) <= 1,
            "{samples}x: {pixel:?}"
        );
    }
    options.textures = false;
    renderer.configure(&options);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[255; 3]);
    for vertex in &mut vertices {
        vertex.flags |= render_mesh::SCREEN;
        vertex.position = [vertex.position[0] / 5.0, vertex.position[1] / 5.0, 0.5];
    }
    let ui = renderer.upload(&vertices);
    options.exposure = 25;
    renderer.configure(&options);
    renderer.draw(&view, camera, &[&ui], None).unwrap();
    assert_eq!(
        &read_pixel(&renderer, &target)[..3],
        &[255, 0, 0],
        "UI keeps nearest textures and original brightness"
    );
    options = crate::settings::Graphics::default();
    renderer.configure(&options);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(
        &read_pixel(&renderer, &target)[..3],
        &[255, 0, 0],
        "restoring defaults restores the original image"
    );
}
