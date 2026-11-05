use super::*;

#[test]
#[ignore = "requires a GPU; specialised ground blood must preserve surface textures, alpha order, snow and fallback styles"]
fn projected_blood_shader_preserves_surface_flags_and_blending() {
    use crate::visuals::surfaces::{Stamp, Surfaces};
    use bytemuck::Zeroable;
    use glam::Vec3;
    use std::sync::Arc;

    let mut vram = Vram::default();
    for y in 0..16 {
        for x in 0..16 {
            vram.words[y * 1024 + x] = [0x7fff, 0, 31, 0x801f][(x / 4 + y / 4) % 4];
        }
    }
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let p = [
        [-2000.0, 0.0, -2000.0],
        [2000.0, 0.0, -2000.0],
        [-2000.0, 0.0, 2000.0],
        [2000.0, 0.0, 2000.0],
    ];
    let vertices = [0, 1, 2, 1, 3, 2].map(|i| Vertex {
        position: p[i],
        normal: Vec3::NEG_Y.to_array(),
        uv: [
            (p[i][0] + 2000.0) * 15.0 / 4000.0,
            (p[i][2] + 2000.0) * 15.0 / 4000.0,
        ],
        page: 256,
        color: [0.7; 3],
        flags: render_mesh::TEXTURED | render_mesh::DOUBLE_SIDED | render_mesh::SCENERY,
        opacity: 1.0,
        ..Vertex::zeroed()
    });
    let ground = renderer.upload(&vertices);
    let surfaces = Surfaces::new(&vertices);
    let pages: Vec<_> = (0..8)
        .map(|i| {
            let mut vertices = surfaces.project(&Stamp {
                center: Vec3::new(
                    (i % 3) as f32 * 600.0 - 600.0,
                    0.0,
                    (i / 3) as f32 * 600.0 - 600.0,
                ),
                normal: Vec3::NEG_Y,
                radius: 900.0,
                depth: 15.0,
                seed: i,
                shape: None,
            });
            for v in &mut vertices {
                match i {
                    1 => {
                        v.flags &= !render_mesh::SOFT;
                        v.color = [1.0; 3];
                    }
                    3 => v.flags |= render_mesh::UNLIT,
                    5 => {
                        v.flags |= render_mesh::FOLIAGE
                            | render_mesh::FLUTTER
                            | (180 << render_mesh::WIND_WEIGHT_SHIFT)
                    }
                    6 => v.flags |= render_mesh::WEAPON_COAT,
                    7 => v.flags |= render_mesh::TRANSLUCENT,
                    _ => (),
                }
            }
            Arc::new(vertices)
        })
        .collect();
    assert!(pages.iter().all(|p| !p.is_empty()));
    renderer.set_decals(&pages);
    let camera = Mat4::orthographic_rh(-2000.0, 2000.0, -2000.0, 2000.0, 1.0, 15000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -5000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let light = crate::lighting::Light {
        position: Vec3::new(-1500.0, -2000.0, 500.0),
        color: [1.0, 0.4, 0.2],
        radius: 12000.0,
        flame: false,
        emission_radius: 0.0,
    };
    let capture = |renderer: &mut Renderer| {
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[&ground], &[]),
                None,
                &[light],
                &[&ground],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.motion.wind = 50;
    for samples in renderer.sample_counts.clone() {
        graphics.samples = samples;
        for (mode, wet) in [
            (crate::settings::WeatherMode::Original, 0.0),
            (crate::settings::WeatherMode::Snow, 0.0),
            (crate::settings::WeatherMode::Snow, 0.7),
            (crate::settings::WeatherMode::Clear, 0.7),
            (crate::settings::WeatherMode::Rain, 0.7),
        ] {
            graphics.weather.mode = mode;
            for hdr in [false, true] {
                graphics.post.enabled = hdr;
                renderer.configure(&graphics);
                renderer.weather = crate::visuals::weather::SurfaceWeather {
                    snow: 0.8,
                    wet,
                    puddles: if wet > 0.0 { 0.5 } else { 0.0 },
                    time: 10.0,
                    flash: 0.0,
                };
                renderer.skip_blood_shader = true;
                let expected = capture(&mut renderer);
                assert!(expected.chunks_exact(4).any(|p| p[..3] != [0; 3]));
                renderer.skip_blood_shader = false;
                assert_eq!(
                    capture(&mut renderer),
                    expected,
                    "samples {samples}, mode {mode:?}, wet {wet}, HDR {hdr}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a graphics adapter; refined sprites keep their position, retail blends and fade endpoints"]
fn original_hd_blood_keeps_placement_and_fades() {
    let mut vram = Vram::default();
    let image = tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [16, 16],
    };
    let mask = tenchu_assets::texture::Image {
        uv: [16, 0],
        ..image
    };
    vram.images.extend([image, mask]);
    for y in 5..11 {
        for x in 3..11 {
            vram.words[y * 1024 + x] = 0x8010;
            vram.words[y * 1024 + x + 16] = 0xffff;
        }
    }
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let background = renderer.upload(&rectangle(1000, 100, None, false, false, false));
    renderer.draw(&view, camera, &[&background], None).unwrap();
    let backdrop = read_pixels(&renderer, &target);
    for blend in [0, 2, 3, 5] {
        for fade in [1.0, 0.2, 0.0] {
            let mut original = None;
            for hd in [false, true] {
                let mut vertices = render_effects::quad(
                    [
                        [-120.0, -120.0, 0.5],
                        [120.0, -120.0, 0.5],
                        [-120.0, 120.0, 0.5],
                        [120.0, 120.0, 0.5],
                    ],
                    [[if blend == 5 { 1.0 } else { fade }; 3]; 4],
                    Some(if blend == 3 { mask } else { image }),
                    blend,
                );
                for v in &mut vertices {
                    v.flags |= render_mesh::STAIN | if hd { render_mesh::BLOOD_HD } else { 0 };
                    if blend == 5 {
                        v.flags = (v.flags & !render_mesh::TRANSLUCENT) | render_mesh::ALPHA;
                    }
                    v.opacity = fade;
                }
                let effect = renderer.upload_effects(render_effects::Geometry {
                    vertices,
                    batches: vec![render_effects::Batch { blend, range: 0..6 }],
                });
                renderer
                    .draw(&view, camera, &[&background], Some(&effect))
                    .unwrap();
                let pixels = read_pixels(&renderer, &target);
                if fade == 0.0 && blend != 0 {
                    assert_eq!(
                        pixels, backdrop,
                        "HD {hd}, blend {blend}: fade left stale pixels"
                    );
                    continue;
                }
                let mut total = 0.0;
                let mut center = glam::Vec2::ZERO;
                for (i, (p, b)) in pixels
                    .chunks_exact(4)
                    .zip(backdrop.chunks_exact(4))
                    .enumerate()
                {
                    let weight = (0..3).map(|c| f32::from(p[c].abs_diff(b[c]))).sum::<f32>();
                    total += weight;
                    center += glam::Vec2::new((i % 64) as f32, (i / 64) as f32) * weight;
                }
                assert!(total > 0.0, "HD {hd}, blend {blend}, fade {fade}");
                center /= total;
                let interior: [u8; 3] = pixels[(32 * 64 + 28) * 4..(32 * 64 + 28) * 4 + 3]
                    .try_into()
                    .unwrap();
                if let Some((reference_center, reference_color)) = original {
                    assert!(
                        center.distance(reference_center) < 0.15,
                        "blend {blend}: {center:?} vs {reference_center:?}"
                    );
                    assert!(
                        interior
                            .into_iter()
                            .zip(reference_color)
                            .all(|(a, b)| a.abs_diff(b) <= 1),
                        "blend {blend}, fade {fade}: {interior:?} vs {reference_color:?}"
                    );
                } else {
                    original = Some((center, interior));
                }
            }
        }
    }
}

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
    for hd in [false, true] {
        for screen in [false, true] {
            for samples in renderer.sample_counts.clone() {
                graphics.samples = samples;
                for filtered in [false, true] {
                    graphics.bilinear = filtered;
                    renderer.configure(&graphics);
                    for sprite in [0, 2, 4, 56, 12] {
                        let image = game.mission.images[sprite];
                        for step in 0..64 {
                            let angle = step as f32 * 0.137;
                            let (sin, cos) = angle.sin_cos();
                            let half = if screen { 78.0 } else { 260.0 };
                            let points =
                                [[-half, -half], [half, -half], [-half, half], [half, half]].map(
                                    |[x, y]| {
                                        [
                                            x * cos - y * sin + step as f32 * 0.37,
                                            x * sin + y * cos,
                                            if screen { 0.5 } else { 1000.0 },
                                        ]
                                    },
                                );
                            let mut vertices =
                                render_effects::quad(points, [[1.; 3]; 4], Some(image), 0);
                            for v in &mut vertices {
                                v.flags = (v.flags & !render_mesh::SCREEN)
                                    | if screen { render_mesh::SCREEN } else { 0 }
                                    | render_mesh::DOUBLE_SIDED
                                    | render_mesh::UNLIT
                                    | render_mesh::STAIN
                                    | if hd { render_mesh::BLOOD_HD } else { 0 };
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
                            assert_eq!(
                                white, 0,
                                "HD {hd}, screen {screen}, samples {samples}, sprite {sprite}, rotation {step}"
                            );
                        }
                    }
                }
            }
        }
    }
    if let Some(output) = std::env::var_os("TENCHU_BLOOD_REVIEW") {
        review_original_hd(&game, output.as_ref());
    }
}

fn review_original_hd(game: &tenchu_core::game::Game, output: &std::path::Path) {
    use crate::capture::review::Review;
    let state = crate::session::tests::state(game);
    let mut review = Review::new(&game.mission).unwrap();
    review.configure(crate::settings::Graphics::default());
    let backdrop = review.backdrop(0.28);
    let mut surfaces = Vec::new();
    let mut labels = Vec::new();
    for (hd, center, title) in [(false, -105.0, "Original"), (true, 105.0, "Original HD")] {
        labels.push((center as i16 - 55, -112, title.to_owned(), [1.0; 3]));
        for (slot, sprite) in [0, 2, 12].into_iter().enumerate() {
            let x = center + (slot as f32 - 1.0) * 62.0;
            let image = game.mission.images[sprite];
            let mut vertices = render_effects::quad(
                [
                    [x - 27.0, -82.0, 0.5],
                    [x + 27.0, -82.0, 0.5],
                    [x - 27.0, -28.0, 0.5],
                    [x + 27.0, -28.0, 0.5],
                ],
                [[1.0; 3]; 4],
                Some(image),
                0,
            );
            for v in &mut vertices {
                v.flags |= render_mesh::STAIN | if hd { render_mesh::BLOOD_HD } else { 0 };
            }
            surfaces.push((0, vertices));
        }
        let mut pool = Vec::new();
        for geometry in &game.mission.common_models[&31] {
            for face in render_mesh::draw_tmd_faces(geometry) {
                render_mesh::append_face(&mut pool, geometry, face, Matrix::default(), false);
            }
        }
        let bounds = pool.iter().fold(
            [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ],
            |mut b, v| {
                for (axis, coordinate) in [v.position[0], v.position[2]].into_iter().enumerate() {
                    b[axis] = b[axis].min(coordinate);
                    b[axis + 2] = b[axis + 2].max(coordinate);
                }
                b
            },
        );
        let scale = (178.0 / (bounds[2] - bounds[0])).min(105.0 / (bounds[3] - bounds[1]));
        for v in &mut pool {
            v.position = [
                center + (v.position[0] - (bounds[0] + bounds[2]) * 0.5) * scale,
                52.0 + (v.position[2] - (bounds[1] + bounds[3]) * 0.5) * scale,
                0.5,
            ];
            v.flags |= render_mesh::SCREEN
                | render_mesh::STAIN
                | if hd { render_mesh::BLOOD_HD } else { 0 };
        }
        for triangle in pool.chunks_exact(3) {
            let blend = if triangle[0].flags & render_mesh::TRANSLUCENT != 0 {
                ((triangle[0].page >> 5) & 3) as u8 + 1
            } else {
                0
            };
            surfaces.push((blend, triangle.to_vec()));
        }
    }
    let mut geometry = crate::capture::review::effects(&surfaces, glam::Mat4::IDENTITY);
    geometry.append(render_effects::text(&game.mission.font, &labels));
    let effects = review.renderer.upload_effects(geometry);
    review
        .capture(
            glam::Mat4::IDENTITY,
            &[&backdrop],
            Some(&effects),
            false,
            output,
        )
        .unwrap();
    assert_eq!(state, crate::session::tests::state(game));
}
