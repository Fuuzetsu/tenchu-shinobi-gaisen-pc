use super::*;
use crate::render_mesh;
use tenchu_assets::model::{Corner, Face, Geometry, Texture};
use tenchu_core::math::Matrix;

mod contact;
mod models;
mod motion;
mod scrolling;
mod sky;
mod sprites;
mod stains;
mod water;
mod weather;
mod weather_wind;

#[test]
#[ignore = "requires GPU timestamps; validates asynchronous scene timing without changing rendered pixels"]
fn detailed_timestamps_preserve_pixels_and_partition_scene_time() {
    let (mut renderer, target) = fixture(&Vram::default());
    if renderer.timings.is_none() {
        return;
    }
    let detailed = renderer
        .device
        .features()
        .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let terrain = renderer.upload(&rectangle(1600, 64, None, false, false, false));
    let actor = renderer.upload(&rectangle(1000, 128, None, false, false, false));
    let mut options = crate::settings::Graphics::default();
    let mut serial = 0;
    for samples in renderer.sample_counts.clone() {
        options.samples = samples;
        for post in [false, true] {
            options.post.enabled = post;
            renderer.configure(&options);
            renderer.profiling = false;
            renderer
                .draw(&view, camera, &[&terrain, &actor], None)
                .unwrap();
            let reference = read_pixels(&renderer, &target);
            renderer.profiling = true;
            for _ in 0..8 {
                renderer
                    .draw_lit(
                        &view,
                        camera,
                        SceneMeshes::new(&[&terrain], &[&actor]),
                        None,
                        &[],
                        &[],
                    )
                    .unwrap();
                assert_eq!(read_pixels(&renderer, &target), reference);
            }
            let sample = renderer.timings.as_ref().unwrap().latest.unwrap();
            assert!(sample.serial > serial);
            serial = sample.serial;
            assert!(
                sample
                    .milliseconds
                    .iter()
                    .all(|ms| ms.is_finite() && *ms >= 0.0)
            );
            assert_eq!(sample.scene.is_some(), detailed);
            if let Some(scene) = sample.scene {
                assert!(scene.iter().all(|ms| ms.is_finite() && *ms >= 0.0));
                assert!((scene.iter().sum::<f32>() - sample.milliseconds[1]).abs() < 0.001);
            }
        }
    }
}

#[test]
#[ignore = "requires a graphics adapter; reused geometry must not retain stale triangles, materials or bounds"]
fn streamed_geometry_matches_fresh_uploads_after_growth_movement_and_empty_frames() {
    let mut vram = Vram::default();
    vram.words[0] = 0x7c00;
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let textured = rectangle(
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
    let mut moved = rectangle(1200, 180, None, false, false, false);
    for v in &mut moved {
        v.position[0] += 600.0;
        v.flags |= render_mesh::SCENERY;
    }
    let mut streamed = renderer.upload(&[]);
    for mut vertices in [
        textured.clone(),
        moved.repeat(600),
        moved,
        Vec::new(),
        textured,
    ] {
        let fresh = renderer.upload(&vertices);
        renderer.draw(&view, camera, &[&fresh], None).unwrap();
        let expected = read_pixels(&renderer, &target);
        renderer.update_mesh(&mut streamed, &mut vertices);
        renderer.draw(&view, camera, &[&streamed], None).unwrap();
        assert_eq!(read_pixels(&renderer, &target), expected);
        assert_eq!(streamed.bounds, fresh.bounds);
    }
}

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

pub(super) fn read_pixel(renderer: &Renderer, target: &wgpu::Texture) -> [u8; 4] {
    let pixels = read_pixels(renderer, target);
    pixels[24 * 256 + 24 * 4..24 * 256 + 25 * 4]
        .try_into()
        .unwrap()
}

pub(super) fn fixture(vram: &Vram) -> (Renderer, wgpu::Texture) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
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
    for samples in renderer.sample_counts.clone() {
        graphics.samples = samples;
        graphics.post.occlusion = 0;
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
        graphics.post.occlusion = 0;
        renderer.configure(&graphics);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        assert_eq!(read_pixels(&renderer, &target), flat);
    }
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
    let mut graphics = crate::settings::Graphics {
        asset_pack: false,
        ..Default::default()
    };
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
    for magnification in 1..=3 {
        options.magnification = magnification;
        renderer.configure(&options);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        assert_eq!(
            read_pixel(&renderer, &target),
            averaged,
            "close-up modes must not change minification"
        );
    }
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
#[ignore = "requires a graphics adapter; close-up variants preserve texel interiors without cubic ringing or changes to default rendering"]
fn closeup_filters_preserve_detail_and_bound_interpolation() {
    let mut vram = Vram::default();
    vram.images.push(tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [4, 1],
    });
    vram.words[..4].copy_from_slice(&[0x8000, 0x8000, 0x7fff, 0x7fff]);
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
        vertex.uv = [(vertex.position[0] + 600.0) / 1200.0 * 3.0, 0.0];
    }
    let mesh = renderer.upload(&vertices);
    let mut graphics = crate::settings::Graphics::default();
    graphics.set_filtering(5);
    let mut images = Vec::new();
    for mode in 0..4 {
        graphics.magnification = mode;
        renderer.configure(&graphics);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        images.push(read_pixels(&renderer, &target));
    }
    let transitions = |pixels: &[u8]| {
        pixels[32 * 256..33 * 256]
            .chunks_exact(4)
            .filter(|p| p[0] > 5 && p[0] < 250)
            .count()
    };
    assert!(transitions(&images[0]) > 8);
    assert!(transitions(&images[1]) <= 2);
    let blending = |pixels: &[u8]| {
        pixels[32 * 256..33 * 256]
            .chunks_exact(4)
            .map(|p| u32::from(p[0].min(255 - p[0])))
            .sum::<u32>()
    };
    assert!(blending(&images[2]) < blending(&images[0]));
    assert_eq!(transitions(&images[3]), 0);
    graphics.set_filtering(0);
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let original = read_pixels(&renderer, &target);
    graphics.magnification = 0;
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert!(read_pixels(&renderer, &target) == original);
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
            opacity: 1.0,
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
            .draw_lit(
                &view,
                matrix,
                crate::renderer::SceneMeshes::new(&[&receiver], &[]),
                None,
                lights,
                &casters,
            )
            .unwrap();
        read_pixel(renderer, &target)
    };
    let original = draw(&mut renderer, &[light], &[&blocker]);
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.source_size = 0;
    graphics.lighting.ambient = 30;
    renderer.configure(&graphics);
    let lit = draw(&mut renderer, &[light], &[]);
    let cosmetic = renderer.upload(&quad(blocker_position, 140.0, render_mesh::ALPHA));
    assert_eq!(draw(&mut renderer, &[light], &[&cosmetic]), lit);
    let shadowed = draw(&mut renderer, &[light], &[&blocker]);
    assert!(lit[0] > shadowed[0] + 25, "{lit:?} versus {shadowed:?}");
    let ambient = (0.5 * 0.3f32.powf(1.0 / 2.2) * 255.0).round() as i32;
    assert!(
        (i32::from(shadowed[0]) - ambient).abs() <= 2,
        "{shadowed:?}"
    );
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), lit);
    vram.words[0] = 0x7fff;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    assert_eq!(draw(&mut renderer, &[light], &[&mixed]), shadowed);
    vram.words[0] |= 0x8000;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&mixed]), lit);

    for (alpha, expected) in [(255, shadowed), (128, lit), (0, lit)] {
        let mut rgba = vec![255; 64];
        // The sampled texel alone changes; opaque neighbors must not hide this hole.
        rgba[(2 * 4 + 2) * 4 + 3] = alpha;
        let pack = crate::visuals::pack::Pack {
            textures: std::collections::BTreeMap::from([(
                crate::visuals::catalog::image_id([1, 1], &[0; 4]),
                std::sync::Arc::new(crate::visuals::pixels::Image { size: [4, 4], rgba }),
            )]),
            ..Default::default()
        };
        renderer.set_asset_pack(&pack);
        assert_eq!(draw(&mut renderer, &[light], &[&mixed]), expected);
        assert_eq!(
            draw(&mut renderer, &[light], &[&transparent]),
            if alpha == 0 { lit } else { shadowed },
        );
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
    // Reusing a lantern's texture does not make ordinary scenery luminous or transparent.
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    let transparent = renderer.upload(&quad(
        blocker_position,
        140.0,
        render_mesh::TEXTURED | render_mesh::EMITTER,
    ));
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), lit);
    graphics.lighting.lamps = false;
    renderer.configure(&graphics);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    graphics.lighting.lamps = true;
    renderer.configure(&graphics);
    vram.words[0] = 0x4210;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    vram.words[0] = 31;
    renderer.update_vram(&vram);
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), shadowed);
    renderer.set_asset_pack(&crate::visuals::pack::Pack {
        textures: std::collections::BTreeMap::from([(
            crate::visuals::catalog::image_id([1, 1], &[0; 4]),
            std::sync::Arc::new(crate::visuals::pixels::Image {
                size: [4, 4],
                rgba: vec![255; 64],
            }),
        )]),
        ..Default::default()
    });
    assert_eq!(draw(&mut renderer, &[light], &[&transparent]), lit);
    renderer.set_asset_pack(&crate::visuals::pack::Pack::default());
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
        let origin = Vec3::from(game.player().position.map(|v| v as f32).to_array());
        let points = [
            Vec3::new(-4000.0, 20.0, -4000.0),
            Vec3::new(4000.0, 20.0, -4000.0),
            Vec3::new(-4000.0, 20.0, 4000.0),
            Vec3::new(4000.0, 20.0, 4000.0),
        ];
        let receiver = renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
            material: 0,
            normal: [0.0; 3],
            opacity: 1.0,
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
        let draw = |renderer: &mut Renderer, casters: &[&Mesh]| {
            renderer
                .draw_lit(
                    &view,
                    matrix,
                    crate::renderer::SceneMeshes::new(&[&receiver], &[]),
                    None,
                    &lights,
                    casters,
                )
                .unwrap();
            read_pixels(renderer, &target)
        };
        let lit = draw(&mut renderer, &[]);
        let mut vertices = render_mesh::shadow_models(&game, None);
        let armed = renderer.upload_shadow(&vertices);
        let armed_pixels = draw(&mut renderer, &[&armed]);
        let mut streamed = renderer.upload(&[]);
        renderer.update_mesh(&mut streamed, &mut vertices);
        assert_eq!(draw(&mut renderer, &[&streamed]), armed_pixels);
        let actors = renderer.upload(&render_mesh::characters_presented(&game, None, true, None));
        render_mesh::update_shadow_models(&mut vertices, &game, None, None, true);
        renderer.update_mesh(&mut streamed, &mut vertices);
        assert_eq!(draw(&mut renderer, &[&streamed, &actors]), armed_pixels);
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
        render_mesh::update_shadow_models(&mut vertices, &game, None, None, false);
        let unarmed = renderer.upload_shadow(&vertices);
        let unarmed_pixels = draw(&mut renderer, &[&unarmed]);
        renderer.update_mesh(&mut streamed, &mut vertices);
        assert_eq!(draw(&mut renderer, &[&streamed]), unarmed_pixels);
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
            opacity: 1.0,
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
            .draw_lit(
                &view,
                camera(x),
                crate::renderer::SceneMeshes::new(&[&ground], &[]),
                None,
                &[],
                &[&ground, &roof],
            )
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
    assert!(lit[0] > shadow[0] + 20, "{lit:?} versus {shadow:?}");
    graphics.lighting.directional.color = 1;
    renderer.configure(&graphics);
    let cool = draw(&mut renderer, 123.0);
    assert!(cool[2] > cool[0] + 5);
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
#[ignore = "requires a graphics adapter; coincident authoring faces must not cover earlier textured scenery"]
fn coplanar_backing_preserves_textures_and_fills_only_cutouts() {
    let mut vram = Vram::default();
    vram.words[0] = 31 << 5;
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
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
    for _ in 0..3 {
        vertices.extend(rectangle(1000, 200, None, false, false, false));
    }
    let mesh = renderer.upload(&vertices);
    let camera = game_camera(Matrix::default(), 1.0);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(read_pixel(&renderer, &target), [0, 255, 0, 255]);

    vram.words[0] = 0;
    renderer.update_vram(&vram);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(read_pixel(&renderer, &target), [200, 200, 200, 255]);
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
            translation: [5000, 0, 2000].into(),
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

#[test]
#[ignore = "requires a graphics adapter; local lights fill moon shadows, back-facing caps stay dark, and highlights retain detail"]
fn lanterns_fill_moon_shadows_without_lighting_backfaces_or_clipping() {
    use crate::lighting::Light;
    use glam::Vec3;
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let matrix = Mat4::orthographic_rh(-1000.0, 1000.0, -1000.0, 1000.0, 1.0, 5000.0)
        * Mat4::look_to_rh(Vec3::new(0.0, 0.0, -2000.0), Vec3::Z, Vec3::NEG_Y);
    let quad = |center: Vec3, size: f32, color: f32| {
        let points = [
            Vec3::new(-size, -size, 0.0),
            Vec3::new(size, -size, 0.0),
            Vec3::new(-size, size, 0.0),
            Vec3::new(size, size, 0.0),
        ];
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: (center + points[i]).to_array(),
            color: [color; 3],
            normal: [0.0; 3],
            opacity: 1.0,
            uv: [0.0; 2],
            page: 0,
            palette: 0,
            material: 0,
            flags: render_mesh::DOUBLE_SIDED,
        })
    };
    let ground = renderer.upload(&quad(Vec3::ZERO, 1000.0, 0.5));
    let bright = renderer.upload(&quad(Vec3::ZERO, 1000.0, 1.0));
    let sample = Vec3::new(-234.375, -234.375, 0.0);
    let lamp = Light {
        position: Vec3::new(-800.0, 300.0, -900.0),
        color: [1.0; 3],
        radius: 5000.0,
    };
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 5;
    graphics.lighting.intensity = 220;
    graphics.lighting.directional.intensity = 50;
    graphics.lighting.directional.color = 0;
    graphics.lighting.directional.azimuth = 180;
    graphics.lighting.directional.elevation = 45;
    graphics.lighting.directional.radius = 10;
    graphics.lighting.directional.resolution = 2048;
    let roof = renderer.upload(&quad(
        sample + super::sun::direction(&graphics.lighting.directional) * 500.0,
        100.0,
        0.5,
    ));
    let lamp_blocker = renderer.upload(&quad((sample + lamp.position) * 0.5, 100.0, 0.5));
    let draw = |renderer: &mut Renderer, receiver: &Mesh, lights: &[Light], blockers: &[&Mesh]| {
        renderer
            .draw_lit(
                &view,
                matrix,
                crate::renderer::SceneMeshes::new(&[receiver], &[]),
                None,
                lights,
                blockers,
            )
            .unwrap();
        f32::from(read_pixel(renderer, &target)[0])
    };
    for hdr in [false, true] {
        graphics.post.enabled = hdr;
        renderer.configure(&graphics);
        let moon = draw(&mut renderer, &ground, &[], &[]);
        let moon_shadow = draw(&mut renderer, &ground, &[], &[&roof]);
        let local = draw(&mut renderer, &ground, &[lamp], &[]);
        let filled = draw(&mut renderer, &ground, &[lamp], &[&roof]);
        let blocked = draw(&mut renderer, &ground, &[lamp], &[&lamp_blocker]);
        assert!(
            moon > moon_shadow + 5.0,
            "moon {moon} / shadow {moon_shadow}"
        );
        assert!(
            (local - filled) / local < (moon - moon_shadow) / moon * 0.3,
            "nearby lamp fills moon shadow: {local} / {filled} vs {moon} / {moon_shadow}"
        );
        assert!(
            local - blocked > (local - filled) * 3.0,
            "local occlusion dominates: {local} / {blocked} / {filled}"
        );
    }
    graphics.post.enabled = false;
    graphics.lighting.directional.intensity = 0;
    graphics.lighting.shadows = false;
    graphics.lighting.intensity = 500;
    renderer.configure(&graphics);
    let ambient = draw(&mut renderer, &ground, &[], &[]);
    let behind = Light {
        position: sample + Vec3::Z * 100.0,
        ..lamp
    };
    assert_eq!(
        draw(&mut renderer, &ground, &[behind], &[]),
        ambient,
        "cap facing away from an internal lamp receives no direct light"
    );
    let front = Light {
        position: sample - Vec3::Z * 100.0,
        ..lamp
    };
    let gray = draw(&mut renderer, &ground, &[front], &[]);
    let white = draw(&mut renderer, &bright, &[front], &[]);
    assert!(
        white < 255.0 && white > gray + 10.0,
        "highlight contrast: gray {gray}, white {white}"
    );
}

#[test]
#[ignore = "requires a graphics adapter; cubic filtering of foliage cutouts must not invent bright pixels"]
fn cubic_cutout_colors_stay_within_the_source_range() {
    let mut vram = Vram::default();
    vram.images.push(tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [4, 1],
    });
    vram.words[..4].copy_from_slice(&[0x8000, 15 << 5, 0, 0]);
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
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
        vertex.uv = [(vertex.position[0] + 600.0) / 1200.0 * 3.0, 0.0];
    }
    let mesh = renderer.upload(&vertices);
    let mut options = crate::settings::Graphics::default();
    options.set_filtering(6);
    options.magnification = 2;
    options.post.enabled = true;
    options.post.filmic = false;
    renderer.configure(&options);
    renderer
        .draw(&view, game_camera(Matrix::default(), 1.0), &[&mesh], None)
        .unwrap();
    let pixels = read_pixels(&renderer, &target);
    let green = pixels.chunks_exact(4).map(|p| p[1]).max().unwrap();
    assert!(
        green <= 124,
        "cubic alpha-edge overshoot: {green}, source maximum 123"
    );
}

#[test]
#[ignore = "requires a graphics adapter; relief must change lighting/parallax and revert exactly without affecting actors, cutouts or distant faces"]
fn surface_relief_is_optional_filtered_and_scoped_to_scenery() {
    let mut vram = Vram::default();
    vram.images.push(tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [32, 32],
    });
    for y in 0..32 {
        for x in 0..32 {
            let c = [9, 12, 18, 25, 29, 25, 18, 12][x % 8] - if y % 8 == 0 { 5 } else { 0 };
            vram.words[y * 1024 + x] =
                c | (c << 5) | (c << 10) | if x % 2 == 0 { 0x8000 } else { 0 };
        }
    }
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
        vertex.uv = [
            (vertex.position[0] / 1200.0 + 0.5) * 31.0,
            (vertex.position[1] / 1200.0 + 0.5) * 31.0,
        ];
        vertex.flags |= render_mesh::SCENERY;
    }
    let mut settings = crate::settings::Graphics {
        fog: false,
        surface_strength: 100,
        ..Default::default()
    };
    settings.set_filtering(2);
    settings.lighting.enabled = true;
    settings.lighting.ambient = 25;
    settings.lighting.directional.azimuth = 225;
    settings.lighting.directional.elevation = 25;
    settings.lighting.directional.intensity = 100;
    let mesh = renderer.upload(&vertices);
    let capture = |renderer: &mut Renderer, settings: &crate::settings::Graphics, mesh: &Mesh| {
        renderer.configure(settings);
        renderer.draw(&view, camera, &[mesh], None).unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer, &settings, &mesh);
    settings.surface_detail = 1;
    let bump = capture(&mut renderer, &settings, &mesh);
    assert!(
        original
            .iter()
            .zip(&bump)
            .filter(|(a, b)| a.abs_diff(**b) > 3)
            .count()
            > 100
    );
    settings.surface_detail = 2;
    let parallax = capture(&mut renderer, &settings, &mesh);
    assert_ne!(bump, parallax);
    settings.surface_detail = 0;
    assert_eq!(original, capture(&mut renderer, &settings, &mesh));
    let region = crate::visuals::catalog::Region {
        page: 256,
        palette: 0,
        rect: [0, 0, 32, 32],
    };
    let id = crate::visuals::catalog::image_id([32, 32], &region.pixels(&vram));
    let mut pack = crate::visuals::pack::Pack::default();
    pack.surfaces.insert(
        id.clone(),
        crate::visuals::relief::Surface {
            height: None,
            strength: 0.0,
            world_tile: None,
            uv_tile: false,
        },
    );
    renderer.set_asset_pack(&pack);
    settings.surface_detail = 2;
    assert_eq!(original, capture(&mut renderer, &settings, &mesh));
    pack.surfaces.insert(
        id,
        crate::visuals::relief::Surface {
            height: Some(std::sync::Arc::new(crate::visuals::pixels::Image {
                size: [8, 8],
                rgba: [128, 128, 128, 255].repeat(64),
            })),
            strength: 1.0,
            world_tile: None,
            uv_tile: false,
        },
    );
    renderer.set_asset_pack(&pack);
    settings.surface_detail = 1;
    assert_eq!(original, capture(&mut renderer, &settings, &mesh));
    renderer.set_asset_pack(&Default::default());
    for vertex in &mut vertices {
        vertex.flags &= !render_mesh::SCENERY;
    }
    let actor = renderer.upload(&vertices);
    settings.surface_detail = 2;
    assert_eq!(original, capture(&mut renderer, &settings, &actor));
    for vertex in &mut vertices {
        vertex.flags |= render_mesh::SCENERY;
        vertex.position[2] = 60000.0;
    }
    let distant = renderer.upload(&vertices);
    let far_detail = capture(&mut renderer, &settings, &distant);
    settings.surface_detail = 0;
    assert_eq!(far_detail, capture(&mut renderer, &settings, &distant));
    vram.words[0] = 0;
    renderer.update_vram(&vram);
    let cutout = capture(&mut renderer, &settings, &mesh);
    settings.surface_detail = 2;
    assert_eq!(cutout, capture(&mut renderer, &settings, &mesh));
}

#[test]
#[ignore = "requires a graphics adapter; finite lamps soften grating and sunlight dominates local illumination while covered lamps remain useful"]
fn finite_lamps_and_daylight_keep_local_shadows_balanced() {
    use crate::lighting::Light;
    use glam::Vec3;
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let matrix = Mat4::orthographic_rh(-1000.0, 1000.0, -1000.0, 1000.0, 1.0, 5000.0)
        * Mat4::look_to_rh(Vec3::new(0.0, 0.0, -2000.0), Vec3::Z, Vec3::NEG_Y);
    let quad = |center: Vec3, extent: glam::Vec2| {
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: (center
                + Vec3::new(
                    if i % 2 == 0 { -extent.x } else { extent.x },
                    if i / 2 == 0 { -extent.y } else { extent.y },
                    0.0,
                ))
            .to_array(),
            color: [0.5; 3],
            normal: [0.0; 3],
            opacity: 1.0,
            uv: [0.0; 2],
            page: 0,
            palette: 0,
            material: 0,
            flags: render_mesh::DOUBLE_SIDED,
        })
    };
    let receiver = renderer.upload(&quad(Vec3::ZERO, glam::Vec2::splat(1000.0)));
    let lamp = Light {
        position: Vec3::new(0.0, 0.0, -1200.0),
        color: [1.0; 3],
        radius: 5000.0,
    };
    let grating: Vec<_> = (-2..=2)
        .flat_map(|x| {
            quad(
                Vec3::new(x as f32 * 60.0, 0.0, -1000.0),
                glam::Vec2::new(10.0, 200.0),
            )
        })
        .collect();
    let grating = renderer.upload(&grating);
    let draw = |renderer: &mut Renderer, lights: &[Light], casters: &[&Mesh]| {
        renderer
            .draw_lit(
                &view,
                matrix,
                crate::renderer::SceneMeshes::new(&[&receiver], &[]),
                None,
                lights,
                casters,
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mean =
        |pixels: &[u8]| pixels.chunks_exact(4).map(|p| f32::from(p[0])).sum::<f32>() / 4096.0;
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 20;
    graphics.lighting.source_size = 0;
    graphics.lighting.resolution = 1024;
    renderer.configure(&graphics);
    let hard = draw(&mut renderer, &[lamp], &[&grating]);
    graphics.lighting.source_size = 120;
    renderer.configure(&graphics);
    let soft = draw(&mut renderer, &[lamp], &[&grating]);
    let dark = |p: &[u8]| p.chunks_exact(4).filter(|p| p[0] < 75).count();
    assert!(
        dark(&soft) < dark(&hard) / 2,
        "hard {}, soft {}",
        dark(&hard),
        dark(&soft)
    );
    let night_unlit = mean(&draw(&mut renderer, &[], &[]));
    let night_lit = mean(&draw(&mut renderer, &[lamp], &[]));
    let mut sky = crate::visuals::sky::Daylight::at(720.0, &graphics.sky);
    sky.direction = Vec3::new(0.0, -0.6, -0.8);
    renderer.daylight = Some(sky);
    graphics.sky.enabled = true;
    graphics.lighting.directional.radius = 10;
    graphics.lighting.directional.resolution = 2048;
    renderer.configure(&graphics);
    let noon_unlit = mean(&draw(&mut renderer, &[], &[]));
    let noon_lit = mean(&draw(&mut renderer, &[lamp], &[]));
    assert!(
        noon_lit - noon_unlit < (night_lit - night_unlit) * 0.15,
        "noon {noon_lit}/{noon_unlit}, night {night_lit}/{night_unlit}"
    );
    let roof = renderer.upload(&quad(
        Vec3::new(0.0, -650.0, -500.0),
        glam::Vec2::new(1500.0, 400.0),
    ));
    let covered_unlit = mean(&draw(&mut renderer, &[], &[&roof]));
    let covered_lit = mean(&draw(&mut renderer, &[lamp], &[&roof]));
    assert!(
        covered_lit - covered_unlit > (noon_lit - noon_unlit) * 2.0,
        "covered {covered_lit}/{covered_unlit}, noon {noon_lit}/{noon_unlit}"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_LIGHT_REVIEW and a GPU; inspect actual lantern grating and daytime fill"]
fn actual_lantern_area_light_review() {
    use glam::Vec3;
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_LIGHT_REVIEW").expect("Set TENCHU_LIGHT_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    for _ in 0..40 {
        game.step(0).unwrap();
    }
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let pack = crate::visuals::pack::Pack::from_env(&game.mission).unwrap();
    let emitters = crate::visuals::lights::Emitters::build(&game, &pack, true);
    review.renderer.set_emitters(&emitters);
    let models = review.models(&game.mission).unwrap();
    let geometry = render_mesh::stage(&game.mission, &game.math, Some(&models));
    let stage = review.renderer.upload(&geometry);
    let player = Vec3::from(game.player().position.map(|v| v as f32).to_array());
    let source = emitters
        .sources(&game)
        .into_iter()
        .min_by(|a, b| {
            a.light
                .position
                .distance_squared(player)
                .total_cmp(&b.light.position.distance_squared(player))
        })
        .unwrap();
    let p = source.light.position;
    eprintln!("Lantern {} at {p:?}", source.name);
    let mut scene = crate::demo::Scene {
        target: p + Vec3::Y * 400.0,
        distance: 5000.0,
        pitch: -0.4,
        ..Default::default()
    };
    assert!(crate::demo::framing::unobstructed_area(
        &mut scene, &geometry, 700.0
    ));
    let mut camera = crate::debug::camera::Camera {
        position: Vec3::ZERO,
        yaw: scene.yaw,
        pitch: scene.pitch,
        fov: 55.0,
    };
    camera.position = scene.target - camera.forward() * scene.distance;
    let camera = camera.matrix(1280.0 / 720.0);
    for (name, time, size) in [
        ("night-hard", 0.0, 0),
        ("night-area", 0.0, 100),
        ("day-hard", 720.0, 0),
        ("day-area", 720.0, 100),
    ] {
        let mut graphics = crate::settings::Graphics::default();
        graphics.set_preset(crate::settings::presets::MODERN_HIGH);
        graphics.lighting.source_size = size;
        graphics.lighting.softness = 0;
        graphics.post.enabled = false;
        review.lights = crate::lighting::sources(
            &game,
            &graphics.lighting,
            Some(p),
            None,
            emitters.sources(&game),
        )
        .iter()
        .map(|s| s.light)
        .collect();
        review.renderer.daylight = Some(crate::visuals::sky::Daylight::at(time, &graphics.sky));
        review.configure(graphics);
        review
            .capture(
                camera,
                &[&stage],
                None,
                true,
                &output.join(format!("{name}.png")),
            )
            .unwrap();
    }
}
