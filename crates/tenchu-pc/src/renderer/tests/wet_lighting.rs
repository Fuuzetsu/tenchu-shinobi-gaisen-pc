use super::*;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; coarse shadows must preserve thin receivers, actors, light changes and resize/toggle restoration"]
fn quarter_resolution_shadows_preserve_receiver_boundaries() {
    let mut vram = Vram::default();
    for y in 0..16 {
        for x in 0..16 {
            vram.words[y * 1024 + x] = [0x4210, 0x7fff][(x + y) % 2];
        }
    }
    let (mut renderer, _) = fixture(&vram);
    if renderer.gl {
        return;
    }
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.post.enabled = true;
    graphics.lighting.enabled = true;
    graphics.lighting.shadows = true;
    graphics.lighting.shadow_count = 8;
    graphics.lighting.resolution = 256;
    graphics.lighting.source_size = 100;
    graphics.sky.enabled = true;
    let mut terrain = rectangle(
        1600,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        false,
        false,
        true,
    );
    for v in &mut terrain {
        v.uv = [
            (v.position[0] + 600.0) / 80.0,
            (v.position[1] + 600.0) / 80.0,
        ];
        v.flags |= render_mesh::SCENERY;
        v.normal = [0.0, 0.0, -1.0];
    }
    let mut slope = terrain.clone();
    for v in &mut slope {
        v.position[0] *= 0.5;
        v.position[2] = 1400.0 + v.position[0] * 0.3;
        v.normal = Vec3::new(0.3, 0.0, -1.0).normalize().to_array();
    }
    terrain.extend(slope);
    // Sub-sample-width receivers cannot borrow the background plane's visibility.
    let mut rail = rectangle(700, 128, None, false, false, true);
    for v in &mut rail {
        v.position[0] *= 0.01;
        v.flags |= render_mesh::SCENERY;
    }
    terrain.extend(rail);
    let ground = renderer.upload(&terrain);
    let mut actor = renderer.upload(&[]);
    let lights: Vec<_> = (0..8)
        .map(|i| crate::lighting::Light {
            position: Vec3::new(
                (i % 4) as f32 * 150.0 - 200.0,
                (i / 4) as f32 * 400.0 - 200.0,
                200.0,
            ),
            color: [0.3, 0.2, 0.1],
            radius: 4000.0,
            flame: false,
            emission_radius: 0.0,
        })
        .collect();
    for (phase, size) in [[256, 256], [257, 253], [256, 256]].into_iter().enumerate() {
        renderer.resize(size);
        graphics.weather.mode = if phase == 1 {
            crate::settings::WeatherMode::Snow
        } else {
            crate::settings::WeatherMode::Rain
        };
        renderer.weather.wet = 1.0;
        renderer.weather.puddles = 1.0;
        renderer.weather.snow = f32::from(u8::from(phase == 1));
        renderer.daylight =
            (phase == 2).then(|| crate::visuals::sky::Daylight::at(720.0, &graphics.sky));
        let mut moving = rectangle(900, 160, None, false, false, true);
        for v in &mut moving {
            v.position[0] = v.position[0] * 0.15 + phase as f32 * 80.0 - 100.0;
            v.position[1] *= 0.3;
        }
        renderer.update_mesh(&mut actor, &mut moving);
        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow sampling regression"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let readback = crate::capture::Readback::new(&renderer, &target);
        let mut captures = Vec::new();
        for enabled in [false, true, false] {
            graphics.rendering.quarter_resolution_shadows = enabled;
            renderer.configure(&graphics);
            renderer
                .draw_lit(
                    &view,
                    game_camera(Matrix::default(), size[0] as f32 / size[1] as f32),
                    SceneMeshes::new(&[&ground], &[&actor]),
                    None,
                    &lights[phase..],
                    &[&ground, &actor],
                )
                .unwrap();
            let mut pixels = Vec::new();
            readback
                .read(&renderer, &target, |bytes, stride| {
                    for row in bytes.chunks_exact(stride).take(size[1] as usize) {
                        pixels.extend_from_slice(&row[..size[0] as usize * 4]);
                    }
                    Ok(())
                })
                .unwrap();
            captures.push(pixels);
        }
        assert_eq!(
            captures[0], captures[2],
            "toggle restoration, phase {phase}"
        );
        if phase == 0 {
            assert_ne!(
                captures[0], captures[1],
                "fixture must exercise shadow interpolation"
            );
        }
        let difference: Vec<_> = captures[0]
            .iter()
            .zip(&captures[1])
            .map(|(&a, &b)| a.abs_diff(b))
            .collect();
        let mean = difference.iter().map(|&d| f32::from(d)).sum::<f32>() / difference.len() as f32;
        let changed =
            difference.iter().filter(|&&d| d > 16).count() as f32 / difference.len() as f32;
        assert!(
            mean < 1.0 && changed < 0.01,
            "phase {phase}: mean {mean}, large changes {changed}"
        );
    }
}

#[test]
#[ignore = "requires a GPU; split lighting must preserve coplanar order, cutouts, special materials and moving actors"]
fn wet_lighting_preserves_surfaces_and_actors() {
    let mut vram = Vram::default();
    for y in 0..16 {
        for x in 0..16 {
            vram.words[y * 1024 + x] = [0, 0x4210, 0x801f, 0x7fff][(x / 4 + y / 4) % 4];
        }
    }
    let (mut renderer, target) = fixture(&vram);
    if renderer.gl {
        return;
    }
    let view = target.create_view(&Default::default());
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.post.enabled = true;
    graphics.lighting.enabled = true;
    graphics.lighting.shadows = true;
    graphics.lighting.resolution = 256;
    graphics.lighting.source_size = 100;
    graphics.weather.mode = crate::settings::WeatherMode::Rain;
    graphics.weather.reflections = 150;
    graphics.weather.haze = 25;
    renderer.configure(&graphics);
    renderer.weather.wet = 1.0;
    renderer.weather.puddles = 1.0;
    assert!(renderer.wet_lighting.is_some());
    let mut actor = renderer.upload(&[]);
    for (weather, reflections) in [
        (crate::settings::WeatherMode::Rain, 150),
        (crate::settings::WeatherMode::Rain, 0),
        (crate::settings::WeatherMode::Snow, 0),
    ] {
        graphics.weather.mode = weather;
        graphics.weather.reflections = reflections;
        renderer.weather.snow = f32::from(u8::from(weather.snow()));
        for (special, water) in [
            (0, false),
            (render_mesh::WATER, false),
            (render_mesh::WATER, true),
            (render_mesh::WATER | render_mesh::WATERFALL, true),
            (render_mesh::UNLIT, false),
        ] {
            graphics.water.enabled = water;
            graphics.sky.enabled = true;
            renderer.configure(&graphics);
            let mut vertices = rectangle(
                1000,
                128,
                Some(Texture {
                    page: 256,
                    palette: 0,
                }),
                true,
                false,
                true,
            );
            for v in &mut vertices {
                v.uv = [
                    (v.position[0] + 600.0) / 80.0,
                    (v.position[1] + 600.0) / 80.0,
                ];
                v.position = [
                    v.position[0] * 4.0,
                    500.0 + v.position[0] * 0.15,
                    v.position[1] * 4.0 + 3000.0,
                ];
                v.normal = Vec3::new(0.15, -1.0, 0.0).normalize().to_array();
                v.flags |= render_mesh::SCENERY;
            }
            let mut backing = vertices.clone();
            for v in &mut backing {
                v.flags = render_mesh::SCENERY | render_mesh::DOUBLE_SIDED | special;
                v.color = [0.1, 0.3, 0.2];
            }
            vertices.extend(backing);
            let mut behind = vertices.clone();
            for v in &mut behind {
                v.color = [0.6, 0.1, 0.3];
                v.flags = render_mesh::SCENERY | render_mesh::DOUBLE_SIDED;
            }
            vertices.extend(behind);
            let effects = renderer.upload_effects(render_effects::Geometry {
                vertices: vertices[..6].to_vec(),
                batches: vec![render_effects::Batch {
                    blend: 1,
                    range: 0..6,
                }],
            });
            let mut ground = renderer.upload(&vertices);
            for indexed in [false, true] {
                if indexed {
                    renderer.update_indexed_mesh(&mut ground, &mut indexed_geometry(&vertices));
                }
                for x in [-250.0, 0.0, 250.0] {
                    let mut moving = rectangle(850, 180, None, false, false, true);
                    for v in &mut moving {
                        v.position[0] = v.position[0] * 0.3 + x;
                        v.position[1] *= 0.4;
                    }
                    renderer.update_mesh(&mut actor, &mut moving);
                    renderer.weather.time += 0.3;
                    renderer.visual_time += 0.3;
                    renderer.daylight =
                        (x > 0.0).then(|| crate::visuals::sky::Daylight::at(720.0, &graphics.sky));
                    let light = crate::lighting::Light {
                        position: Vec3::new(x - 500.0, -700.0, 1800.0),
                        color: [1.0, 0.5, 0.2],
                        radius: 7000.0,
                        flame: false,
                        emission_radius: 0.0,
                    };
                    let camera =
                        game_camera(Matrix::default(), 1.0) * Mat4::from_rotation_y(x / 2500.0);
                    assert!(crate::renderer::wet_lighting::useful(
                        &renderer,
                        camera,
                        &[&ground]
                    ));
                    let mut captures = Vec::new();
                    for skip in [true, false] {
                        renderer.skip_wet_lighting = skip;
                        renderer
                            .draw_lit(
                                &view,
                                camera,
                                SceneMeshes::new(&[&ground], &[&actor]),
                                Some(&effects),
                                &[light],
                                &[&ground, &actor],
                            )
                            .unwrap();
                        captures.push(read_pixels(&renderer, &target));
                    }
                    let differences: Vec<_> = captures[0]
                        .iter()
                        .zip(&captures[1])
                        .map(|(&a, &b)| a.abs_diff(b))
                        .collect();
                    let maximum = *differences.iter().max().unwrap();
                    let mean = differences.iter().map(|&d| f32::from(d)).sum::<f32>()
                        / differences.len() as f32;
                    assert!(
                        maximum <= 4 && mean < 0.1,
                        "material {special}, indexed {indexed}, actor {x}: max {maximum}, mean {mean}"
                    );
                }
            }
        }
    }
    graphics.sky.enabled = true;
    graphics.weather.haze = 0;
    renderer.configure(&graphics);
    renderer.daylight = Some(crate::visuals::sky::Daylight::at(720.0, &graphics.sky));
    let camera = game_camera(Matrix::default(), 1.0);
    for depth in [249000.0, 249975.0] {
        let mut vertices = rectangle(1000, 200, None, false, false, true);
        for v in &mut vertices {
            v.flags |= render_mesh::SCENERY;
            v.position[0] *= 200.0;
            v.position[1] *= 200.0;
            v.position[2] = depth;
        }
        let ground = renderer.upload(&vertices);
        let mut captures = Vec::new();
        for skip in [true, false] {
            renderer.skip_wet_lighting = skip;
            renderer
                .draw_lit(
                    &view,
                    camera,
                    SceneMeshes::new(&[&ground], &[]),
                    None,
                    &[],
                    &[&ground],
                )
                .unwrap();
            captures.push(read_pixels(&renderer, &target));
        }
        let maximum = captures[0]
            .iter()
            .zip(&captures[1])
            .map(|(&a, &b)| a.abs_diff(b))
            .max()
            .unwrap();
        assert!(
            maximum <= 2,
            "Sky ordering at depth {depth}: max difference {maximum}"
        );
    }
    graphics.samples = 4;
    renderer.configure(&graphics);
    if renderer.samples != 1 {
        assert!(renderer.wet_lighting.is_none());
    }
    graphics.samples = 1;
    graphics.weather.mode = crate::settings::WeatherMode::Original;
    renderer.configure(&graphics);
    assert!(renderer.wet_lighting.is_none());
}
