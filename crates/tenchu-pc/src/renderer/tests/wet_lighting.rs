use super::*;
use glam::Vec3;

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
