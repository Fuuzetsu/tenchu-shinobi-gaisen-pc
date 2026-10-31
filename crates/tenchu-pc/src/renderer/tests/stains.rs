use super::*;

#[test]
#[ignore = "requires a graphics adapter; retained original blood uses retail texture colors/cutouts rather than the analytic stain tint"]
fn original_blood_decals_preserve_retail_texels() {
    let mut vram = Vram::default();
    vram.words[0] = 31;
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let background = renderer.upload(&rectangle(1500, 102, None, false, false, false));
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
    for v in &mut vertices {
        v.flags |= render_mesh::STAIN | render_mesh::ALPHA;
    }
    let page = std::sync::Arc::new(vertices);
    renderer.set_decals([&page]);
    renderer.draw(&view, camera, &[&background], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[255, 0, 0]);
    vram.words[0] = 0;
    renderer.update_vram(&vram);
    renderer.draw(&view, camera, &[&background], None).unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[102; 3]);
}

#[test]
#[ignore = "requires a graphics adapter; alpha stains respect depth and texture cutouts, blend softly, and clear without altering the original pass"]
fn projected_blood_blends_without_filling_cutouts_or_occluding_foreground() {
    let mut vram = Vram::default();
    vram.words[0] = 0x7fff;
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let source = rectangle(
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
    let mesh = renderer.upload(&source);
    let background = renderer.upload(&rectangle(1500, 102, None, false, false, false));
    let mut vertices = crate::visuals::surfaces::Surfaces::new(&source).project(
        &crate::visuals::surfaces::Stamp {
            center: glam::Vec3::new(0.0, 0.0, 1000.0),
            normal: glam::Vec3::NEG_Z,
            radius: 400.0,
            depth: 1.0,
            seed: 0,
            shape: None,
        },
    );
    for v in &mut vertices {
        v.opacity *= 0.5;
    }
    let count = vertices.len() as u32;
    let effect = renderer.upload_effects(render_effects::Geometry {
        vertices,
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..count,
        }],
    });
    renderer
        .draw(&view, camera, &[&background, &mesh], None)
        .unwrap();
    let original = read_pixel(&renderer, &target);
    renderer
        .draw(&view, camera, &[&background, &mesh], Some(&effect))
        .unwrap();
    let stained = read_pixel(&renderer, &target);
    assert!(stained[0] > stained[1] + 20, "{stained:?}");
    assert!(
        stained[1] > 90 && stained[1] < 170,
        "soft alpha blend: {stained:?}"
    );
    renderer
        .draw(&view, camera, &[&background, &mesh], None)
        .unwrap();
    assert_eq!(read_pixel(&renderer, &target), original);
    let foreground = renderer.upload(&rectangle(800, 60, None, false, false, false));
    renderer
        .draw(
            &view,
            camera,
            &[&background, &mesh, &foreground],
            Some(&effect),
        )
        .unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[60; 3]);
    vram.words[0] = 0;
    renderer.update_vram(&vram);
    renderer
        .draw(&view, camera, &[&background, &mesh], Some(&effect))
        .unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[102; 3]);
}

#[test]
#[ignore = "requires a graphics adapter; retained decals reuse uploads, survive updates, cull outside the view and clear without stale pixels"]
fn retained_decal_pages_match_streamed_drawing_and_reuse_gpu_buffers() {
    use std::sync::Arc;
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let source = rectangle(1000, 128, None, false, false, false);
    let mesh = renderer.upload(&source);
    let vertices = crate::visuals::surfaces::Surfaces::new(&source).project(
        &crate::visuals::surfaces::Stamp {
            center: glam::Vec3::new(0., 0., 1000.),
            normal: glam::Vec3::NEG_Z,
            radius: 400.,
            depth: 1.,
            seed: 4,
            shape: None,
        },
    );
    assert!(
        vertices.len() < 100,
        "analytic edges must not require a subdivided mesh"
    );
    let effect = renderer.upload_effects(render_effects::Geometry {
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..vertices.len() as u32,
        }],
        vertices: vertices.clone(),
    });
    renderer
        .draw(&view, camera, &[&mesh], Some(&effect))
        .unwrap();
    let expected = read_pixels(&renderer, &target);
    let mut page = Arc::new(vertices);
    assert_eq!(renderer.set_decals([&page]), 1);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(read_pixels(&renderer, &target), expected);
    for _ in 0..60 {
        assert_eq!(renderer.set_decals([&page]), 0);
    }
    for vertex in Arc::make_mut(&mut page) {
        vertex.position[0] += 10000.0;
    }
    assert_eq!(renderer.set_decals([&page]), 1);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(read_pixel(&renderer, &target)[..3], [128; 3]);
    renderer.set_decals(std::iter::empty());
    assert!(renderer.decals.is_empty());
}

#[test]
#[ignore = "requires TENCHU_DISC and graphics adapter; rotated original blood billboards cannot sample adjacent mask texels"]
fn original_blood_billboards_have_no_white_borders() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let game = crate::session::Options {
        mission: 4,
        character: 1,
        ..Default::default()
    }
    .load(&assets)
    .unwrap();
    let (mut renderer, target) = fixture(&game.mission.vram);
    let target_view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut graphics = crate::settings::Graphics {
        exposure: 300,
        upscale: 4,
        magnification: 3,
        ..Default::default()
    };
    for filtered in [false, true] {
        graphics.bilinear = filtered;
        renderer.configure(&graphics);
        for sprite in [0, 2, 4, 56, 12] {
            let image = game.mission.images[sprite];
            for step in 0..64 {
                let angle = step as f32 * 0.137;
                let (sin, cos) = angle.sin_cos();
                let points =
                    [[-260., -260.], [260., -260.], [-260., 260.], [260., 260.]].map(|[x, y]| {
                        [
                            x * cos - y * sin + step as f32 * 0.37,
                            x * sin + y * cos,
                            1000.,
                        ]
                    });
                let mut vertices = render_effects::quad(points, [[1.; 3]; 4], Some(image), 0);
                for v in &mut vertices {
                    v.flags = (v.flags & !render_mesh::SCREEN)
                        | render_mesh::DOUBLE_SIDED
                        | render_mesh::UNLIT
                        | render_mesh::STAIN;
                }
                let effect = renderer.upload_effects(render_effects::Geometry {
                    vertices,
                    batches: vec![render_effects::Batch {
                        blend: 0,
                        range: 0..6,
                    }],
                });
                renderer
                    .draw(&target_view, camera, &[], Some(&effect))
                    .unwrap();
                let pixels = read_pixels(&renderer, &target);
                let white = pixels
                    .chunks_exact(4)
                    .filter(|p| p[0] > 80 && p[1] > 80 && p[2] > 80)
                    .count();
                assert_eq!(white, 0, "sprite {sprite}, rotation {step}");
            }
        }
    }
}
