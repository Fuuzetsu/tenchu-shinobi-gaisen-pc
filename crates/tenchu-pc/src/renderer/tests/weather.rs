use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; wet reflections respond to lights, puddles ripple and rain obscures distance with black fog off"]
fn rain_controls_change_reflections_ripples_and_visibility() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let p = [
        [-5000.0, 0.0, -5000.0],
        [5000.0, 0.0, -5000.0],
        [-5000.0, 0.0, 5000.0],
        [5000.0, 0.0, 5000.0],
    ];
    let ground = renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
        position: p[i],
        normal: Vec3::NEG_Y.to_array(),
        color: [0.25; 3],
        flags: render_mesh::DOUBLE_SIDED | render_mesh::SCENERY,
        opacity: 1.0,
        ..Vertex::zeroed()
    }));
    let camera = Mat4::perspective_rh(1.0, 1.0, 10.0, 30000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -1500.0, -2000.0), Vec3::ZERO, Vec3::NEG_Y);
    let light = crate::lighting::Light {
        position: Vec3::new(0.0, -1500.0, 2000.0),
        color: [1.0, 0.45, 0.15],
        radius: 10000.0,
    };
    let capture = |renderer: &mut Renderer, lights: &[crate::lighting::Light]| {
        renderer
            .draw_lit(&view, camera, &[&ground], None, lights, &[&ground])
            .unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer, &[light]);
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.weather.mode = crate::settings::WeatherMode::Rain;
    graphics.weather.haze = 0;
    graphics.weather.reflections = 0;
    graphics.lighting.enabled = true;
    graphics.lighting.directional.intensity = 0;
    renderer.configure(&graphics);
    renderer.weather.wet = 1.0;
    renderer.weather.puddles = 1.0;
    let matte = capture(&mut renderer, &[light]);
    graphics.weather.reflections = 200;
    renderer.configure(&graphics);
    let reflected = capture(&mut renderer, &[light]);
    assert!(
        reflected
            .chunks_exact(4)
            .zip(matte.chunks_exact(4))
            .any(|(a, b)| i16::from(a[0]) - i16::from(b[0]) > 25)
    );
    let no_light = capture(&mut renderer, &[]);
    assert_ne!(reflected, no_light);
    renderer.weather.time = 0.6;
    assert_ne!(
        reflected,
        capture(&mut renderer, &[light]),
        "Puddle ripples must move with cosmetic time"
    );
    graphics.weather.haze = 100;
    renderer.configure(&graphics);
    let haze = capture(&mut renderer, &[light]);
    graphics.weather.haze = 0;
    renderer.configure(&graphics);
    assert_ne!(
        haze,
        capture(&mut renderer, &[light]),
        "Rain haze must work with original fog disabled"
    );
    renderer.configure(&crate::settings::Graphics::default());
    assert_eq!(original, capture(&mut renderer, &[light]));
}

#[test]
#[ignore = "requires a GPU; checks shelter, snow, wet surfaces and unchanged original rendering"]
fn rain_and_snow_respect_shelter_and_restore_original_pixels() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let mut quad = |y: f32, size: f32| {
        let p = [
            [-size, y, -size],
            [size, y, -size],
            [-size, y, size],
            [size, y, size],
        ];
        renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: p[i],
            color: [0.5; 3],
            flags: render_mesh::DOUBLE_SIDED | render_mesh::SCENERY,
            opacity: 1.0,
            ..Vertex::zeroed()
        }))
    };
    let ground = quad(0.0, 1500.0);
    let roof = quad(-500.0, 400.0);
    let camera = Mat4::orthographic_rh(-1500.0, 1500.0, -1500.0, 1500.0, 1.0, 10000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -3000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let capture = |renderer: &mut Renderer| {
        renderer
            .draw_lit(&view, camera, &[&ground], None, &[], &[&ground, &roof])
            .unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer);
    let mut graphics = crate::settings::Graphics::default();
    graphics.weather.mode = crate::settings::WeatherMode::Rain;
    graphics.weather.haze = 0;
    renderer.configure(&graphics);
    renderer.weather.wet = 1.0;
    let rain = capture(&mut renderer);
    let at = |image: &[u8], x: usize, y: usize| image[(y * 64 + x) * 4];
    assert_eq!(at(&original, 32, 32), at(&rain, 32, 32));
    assert!(at(&rain, 10, 10) < at(&original, 10, 10) - 10);
    renderer.weather.wet = 0.0;
    renderer.weather.snow = 1.0;
    let snow = capture(&mut renderer);
    assert_eq!(at(&original, 32, 32), at(&snow, 32, 32));
    assert!(at(&snow, 10, 10) > at(&original, 10, 10) + 25);
    renderer.configure(&crate::settings::Graphics::default());
    assert_eq!(original, capture(&mut renderer));
}

#[test]
#[ignore = "requires a GPU; moving actors do not erase snow and wind shifts shelter past roofs"]
fn snowfall_shelter_excludes_actors_and_follows_the_wind() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let quad = |y: f32, size: f32, flags| {
        let p = [
            [-size, y, -size],
            [size, y, -size],
            [-size, y, size],
            [size, y, size],
        ];
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: p[i],
            color: [0.5; 3],
            flags: flags | render_mesh::DOUBLE_SIDED,
            opacity: 1.0,
            ..Vertex::zeroed()
        })
    };
    let ground = renderer.upload(&quad(0.0, 1500.0, render_mesh::SCENERY));
    let roof = renderer.upload(&quad(-500.0, 400.0, render_mesh::SCENERY));
    let actor = renderer.upload(&quad(-500.0, 400.0, 0));
    let camera = Mat4::orthographic_rh(-1500.0, 1500.0, -1500.0, 1500.0, 1.0, 10000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -3000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let mut graphics = crate::settings::Graphics::default();
    graphics.weather.mode = crate::settings::WeatherMode::Snow;
    graphics.weather.wind = 0;
    renderer.configure(&graphics);
    renderer.weather.snow = 1.0;
    let capture = |renderer: &mut Renderer, extra: Option<&Mesh>| {
        let mut casters = vec![&ground];
        casters.extend(extra);
        renderer
            .draw_lit(&view, camera, &[&ground], None, &[], &casters)
            .unwrap();
        read_pixels(renderer, &target)
    };
    let open = capture(&mut renderer, None);
    assert_eq!(open, capture(&mut renderer, Some(&actor)));
    let calm = capture(&mut renderer, Some(&roof));
    let at = |image: &[u8], x: usize| image[(32 * 64 + x) * 4];
    assert!(at(&calm, 32) + 20 < at(&open, 32));
    graphics.weather.wind = 100;
    graphics.weather.heading = 90;
    renderer.configure(&graphics);
    renderer.weather.snow = 1.0;
    let wind = capture(&mut renderer, Some(&roof));
    assert!(at(&wind, 32) > at(&calm, 32) + 20);
    assert!(
        wind.iter()
            .zip(&open)
            .any(|(&a, &b)| i16::from(b) - i16::from(a) > 30)
    );
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_WEATHER_REVIEW and a GPU; reviews precipitation on actual roofs"]
fn weather_on_actual_roofs() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_WEATHER_REVIEW").expect("Set TENCHU_WEATHER_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let game = crate::session::Options::default().load(&assets).unwrap();
    let state = crate::session::tests::state(&game);
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let models = review.models(&game.mission).unwrap();
    let vertices = render_mesh::stage(&game.mission, &game.math, Some(&models));
    let surfaces = crate::visuals::surfaces::Surfaces::new(&vertices);
    let stage = review.renderer.upload(&vertices);
    let center = Vec3::new(-48300.0, -7600.0, -39400.0);
    let camera = Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
        * Mat4::look_at_rh(center, Vec3::new(-44667.0, -5000.0, -34667.0), Vec3::NEG_Y);
    for (name, mode, ticks) in [
        ("clear", crate::settings::WeatherMode::Original, 0),
        ("rain", crate::settings::WeatherMode::Rain, 900),
        ("snow", crate::settings::WeatherMode::Snow, 1500),
    ] {
        let mut graphics = crate::settings::Graphics::default();
        graphics.set_preset(3);
        graphics.fog = false;
        graphics.sky.enabled = true;
        graphics.weather.mode = mode;
        graphics.weather.amount = 100;
        let mut weather = crate::visuals::weather::Weather::default();
        for tick in 0..ticks {
            weather.tick(tick, center, &surfaces, &graphics.weather);
        }
        let mut geometry = weather.geometry(camera, 0.5, &graphics.weather);
        let mut original = crate::render_effects::build_presented(
            &game,
            false,
            None,
            false,
            Some(&models),
            None,
            crate::render_effects::Style::default(),
        );
        original.remove_flag(render_mesh::ORIGINAL_WEATHER);
        geometry.append(original);
        let effects = review.renderer.upload_effects(geometry);
        review.renderer.weather = weather.sample(&graphics.weather, 0.5);
        review.renderer.daylight = Some(crate::visuals::sky::Daylight::at(720.0, &graphics.sky));
        review.configure(graphics);
        review
            .capture(
                camera,
                &[&stage],
                Some(&effects),
                true,
                &output.join(format!("{name}.png")),
            )
            .unwrap();
    }
    assert_eq!(state, crate::session::tests::state(&game));
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_WEATHER_REVIEW and a GPU; reviews wet lamps, puddles, haze and eave drips in Mission 1"]
fn rain_on_actual_roofs_and_ground() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_WEATHER_REVIEW").expect("Set TENCHU_WEATHER_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    let state = crate::session::tests::state(&game);
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let models = review.models(&game.mission).unwrap();
    let mut vertices = render_mesh::stage(&game.mission, &game.math, Some(&models));
    let surfaces = crate::visuals::surfaces::Surfaces::new(&vertices);
    vertices.extend(render_mesh::characters(&game, Some(&models)));
    let mesh = review.renderer.upload(&vertices);
    let pack = crate::visuals::pack::Pack::from_env(&game.mission).unwrap();
    let emitters = crate::visuals::lights::Emitters::build(&game, &pack, true);
    review.renderer.set_emitters(&emitters);
    for (shot, eye, target) in [
        (
            "roof",
            Vec3::new(-48300.0, -7600.0, -39400.0),
            Vec3::new(-44667.0, -5000.0, -34667.0),
        ),
        (
            "ground",
            Vec3::new(-45500.0, -1800.0, -44700.0),
            Vec3::new(-42000.0, -800.0, -36500.0),
        ),
    ] {
        let camera = Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
            * Mat4::look_at_rh(eye, target, Vec3::NEG_Y);
        let mut graphics = crate::settings::Graphics::default();
        graphics.set_preset(3);
        graphics.fog = false;
        graphics.sky.enabled = true;
        graphics.sky.time = 600;
        graphics.weather.mode = crate::settings::WeatherMode::Rain;
        graphics.weather.amount = 100;
        graphics.weather.pooling = 600;
        graphics.weather.quality = 2;
        graphics.weather.wetness = 100;
        graphics.weather.puddles = 80;
        graphics.weather.runoff_amount = 100;
        graphics.weather.reflections = 180;
        let mut weather = crate::visuals::weather::Weather::default();
        for tick in 0..600 {
            weather.tick(tick, eye, &surfaces, &graphics.weather);
        }
        review.lights = crate::lighting::sources(
            &game,
            &graphics.lighting,
            Some(eye),
            None,
            emitters.sources(&game),
        )
        .into_iter()
        .map(|source| source.light)
        .collect();
        review.renderer.daylight = Some(crate::visuals::sky::Daylight::at(600.0, &graphics.sky));
        for (name, reflections, haze, streaks) in [
            ("matte", 0, 0, 100),
            ("wet", 180, 0, 100),
            ("storm", 180, 80, 150),
            ("drips", 180, 0, 0),
        ] {
            graphics.weather.reflections = reflections;
            graphics.weather.haze = haze;
            graphics.weather.streaks = streaks;
            review.renderer.weather = weather.sample(&graphics.weather, 0.5);
            let effects =
                review
                    .renderer
                    .upload_effects(weather.geometry(camera, 0.5, &graphics.weather));
            review.configure(graphics.clone());
            review
                .capture(
                    camera,
                    &[&mesh],
                    Some(&effects),
                    true,
                    &output.join(format!("{shot}-{name}.png")),
                )
                .unwrap();
        }
    }
    assert_eq!(state, crate::session::tests::state(&game));
}

#[test]
#[ignore = "requires a GPU; distant snow and projected blood/footprints share the snow surface"]
fn distant_snow_and_surface_stains() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let p = [
        [-100000.0, 0.0, -100000.0],
        [100000.0, 0.0, -100000.0],
        [-100000.0, 0.0, 100000.0],
        [100000.0, 0.0, 100000.0],
    ];
    let vertices = [0, 1, 2, 1, 3, 2].map(|i| Vertex {
        position: p[i],
        normal: Vec3::NEG_Y.to_array(),
        color: [0.15; 3],
        flags: render_mesh::DOUBLE_SIDED | render_mesh::SCENERY,
        opacity: 1.0,
        ..Vertex::zeroed()
    });
    let ground = renderer.upload(&vertices);
    let camera = Mat4::orthographic_rh(-100000.0, 100000.0, -100000.0, 100000.0, 1.0, 300000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -200000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let capture = |renderer: &mut Renderer, effect: Option<&EffectMesh>| {
        renderer
            .draw_lit(&view, camera, &[&ground], effect, &[], &[&ground])
            .unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer, None);
    let mut graphics = crate::settings::Graphics::default();
    graphics.weather.mode = crate::settings::WeatherMode::Snow;
    renderer.configure(&graphics);
    renderer.weather.snow = 0.6;
    let settled = capture(&mut renderer, None);
    let open: Vec<_> = (8..56)
        .flat_map(|y| (8..56).map(move |x| (y * 64 + x) * 4))
        .map(|i| settled[i])
        .collect();
    assert!(
        open.iter().max().unwrap() - open.iter().min().unwrap() < 20,
        "Open snow should form a continuous layer, not rectangular noise patches"
    );
    renderer.weather.snow = 1.0;
    let snow = capture(&mut renderer, None);
    for x in [6, 32, 58] {
        assert!(
            snow[(32 * 64 + x) * 4] > original[(32 * 64 + x) * 4] + 70,
            "snow at x={x}"
        );
    }
    let mut decal = |flag| {
        renderer.upload_effects(render_effects::Geometry {
            vertices: vertices
                .iter()
                .map(|v| Vertex {
                    flags: v.flags | render_mesh::ALPHA | flag,
                    ..*v
                })
                .collect(),
            batches: vec![render_effects::Batch {
                blend: 5,
                range: 0..6,
            }],
        })
    };
    let blood = decal(render_mesh::STAIN | render_mesh::SOFT);
    let foot = decal(render_mesh::FOOTPRINT);
    let stain = capture(&mut renderer, Some(&blood));
    let track = capture(&mut renderer, Some(&foot));
    let at = (32 * 64 + 32) * 4;
    let mut previous = track[at];
    for opacity in [0.75, 0.5, 0.25, 0.01] {
        let fade = renderer.upload_effects(render_effects::Geometry {
            vertices: vertices
                .iter()
                .map(|v| Vertex {
                    flags: v.flags | render_mesh::ALPHA | render_mesh::FOOTPRINT,
                    opacity,
                    ..*v
                })
                .collect(),
            batches: vec![render_effects::Batch {
                blend: 5,
                range: 0..6,
            }],
        });
        let faded = capture(&mut renderer, Some(&fade))[at];
        assert!(
            faded > previous,
            "Footprint burial must lighten the rendered mark continuously"
        );
        previous = faded;
    }
    assert!(stain[at] > stain[at + 1] + 40);
    assert!(track[at] < snow[at] - 30 && track[at] > original[at] + 30);
    renderer.weather.snow = 0.0;
    let bare_blood = capture(&mut renderer, Some(&blood));
    assert!(
        stain[at] > bare_blood[at] + 30,
        "blood should stain the white snow rather than expose the dark substrate"
    );
    assert_eq!(
        capture(&mut renderer, Some(&foot)),
        capture(&mut renderer, None)
    );
    renderer.configure(&crate::settings::Graphics::default());
    assert_eq!(capture(&mut renderer, Some(&foot)), original);
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_WEATHER_REVIEW and a GPU; reviews blood on actual snow-covered terrain"]
fn blood_on_actual_snow_covered_terrain() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_WEATHER_REVIEW").expect("Set TENCHU_WEATHER_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    let state = crate::session::tests::state(&game);
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let models = review.models(&game.mission).unwrap();
    let vertices = render_mesh::stage(&game.mission, &game.math, Some(&models));
    let surfaces = crate::visuals::surfaces::Surfaces::new(&vertices);
    let center = Vec3::from(game.player().position.map(|n| n as f32).to_array());
    let mut blood = Vec::new();
    for (i, delta) in [
        [1000.0, 0.0, 500.0],
        [1000.0, 0.0, 1200.0],
        [1800.0, 0.0, 900.0],
    ]
    .into_iter()
    .enumerate()
    {
        let point = center + Vec3::from(delta);
        let hit = surfaces
            .cast(point - Vec3::Y * 2000.0, point + Vec3::Y * 2000.0)
            .unwrap();
        blood.extend(surfaces.project(&crate::visuals::surfaces::Stamp {
            center: hit.position,
            normal: hit.normal,
            radius: 200.0,
            depth: 100.0,
            seed: i as u32,
            shape: None,
        }));
    }
    assert!(!blood.is_empty());
    let effects = review.renderer.upload_effects(render_effects::Geometry {
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..blood.len() as u32,
        }],
        vertices: blood,
    });
    let stage = review.renderer.upload(&vertices);
    let actors = review
        .renderer
        .upload(&render_mesh::characters(&game, Some(&models)));
    let camera = Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
        * Mat4::look_at_rh(
            center + Vec3::new(3500.0, -5500.0, -4500.0),
            center + Vec3::new(1000.0, 0.0, 500.0),
            Vec3::NEG_Y,
        );
    let mut graphics = crate::settings::Graphics::default();
    graphics.weather.mode = crate::settings::WeatherMode::Snow;
    graphics.weather.wind = 0;
    graphics.fog = false;
    review.configure(graphics);
    review.renderer.weather.snow = 1.0;
    review
        .capture(
            camera,
            &[&stage, &actors],
            Some(&effects),
            true,
            &output.join("blood-on-snow.png"),
        )
        .unwrap();
    assert_eq!(state, crate::session::tests::state(&game));
}
