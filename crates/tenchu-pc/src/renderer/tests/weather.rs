use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

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
            false,
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
