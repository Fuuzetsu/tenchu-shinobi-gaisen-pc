use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; rain/snow affect windward walls, stay off leeward/calm walls, and obey upstream shelter"]
fn windward_walls_receive_weather_and_sheltered_walls_stay_dry() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let mut wall = |x, top| {
        let p = [
            [x, top, -2500.0],
            [x, 1000.0, -2500.0],
            [x, top, 2500.0],
            [x, 1000.0, 2500.0],
        ];
        renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: p[i],
            color: [0.4; 3],
            opacity: 1.0,
            flags: render_mesh::SCENERY | render_mesh::DOUBLE_SIDED,
            ..Vertex::zeroed()
        }))
    };
    let receiver = wall(0.0, -4000.0);
    let shelter = wall(-100.0, -10000.0);
    let capture = |renderer: &mut Renderer, side: f32, sheltered| {
        let camera = Mat4::orthographic_rh(-1000.0, 1000.0, -1000.0, 1000.0, 1.0, 10000.0)
            * Mat4::look_at_rh(
                Vec3::new(side * 3000.0, -1500.0, 0.0),
                Vec3::new(0.0, -1500.0, 0.0),
                Vec3::NEG_Y,
            );
        let mut casters = vec![&receiver];
        if sheltered {
            casters.push(&shelter);
        }
        renderer
            .draw_lit(&view, camera, &[&receiver], None, &[], &casters)
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mean = |image: &[u8]| {
        let mut sum = 0.0;
        for y in 16..48 {
            for x in 16..48 {
                sum += f32::from(image[(y * 64 + x) * 4]);
            }
        }
        sum / (32.0 * 32.0)
    };
    let original = [
        capture(&mut renderer, -1.0, false),
        capture(&mut renderer, 1.0, false),
    ];
    let mut graphics = crate::settings::Graphics::default();
    graphics.weather.haze = 0;
    graphics.weather.reflections = 0;
    graphics.weather.wind = 100;
    for mode in [
        crate::settings::WeatherMode::Rain,
        crate::settings::WeatherMode::Snow,
    ] {
        graphics.weather.mode = mode;
        renderer.weather.wet = f32::from(mode == crate::settings::WeatherMode::Rain);
        renderer.weather.snow = f32::from(mode == crate::settings::WeatherMode::Snow);
        for quality in [0, 2] {
            graphics.weather.quality = quality;
            for (side, heading, index) in [(-1.0, 90, 0), (1.0, 270, 1)] {
                graphics.weather.wind = 100;
                graphics.weather.heading = heading;
                renderer.configure(&graphics);
                let incoming = capture(&mut renderer, side, false);
                let delta = mean(&incoming) - mean(&original[index]);
                if mode == crate::settings::WeatherMode::Rain {
                    assert!(
                        delta < -4.0,
                        "windward rain must wet walls: {delta}, quality {quality}"
                    );
                } else {
                    assert!(
                        delta > 4.0,
                        "windward snow must dust walls: {delta}, quality {quality}"
                    );
                }
                if side < 0.0 {
                    let covered = capture(&mut renderer, side, true);
                    assert!(
                        covered == original[index],
                        "An upstream wall must block {mode:?} at quality {quality}, mean delta {}",
                        mean(&covered) - mean(&original[index]),
                    );
                }
                graphics.weather.heading = (heading + 180) % 360;
                renderer.configure(&graphics);
                assert!(
                    capture(&mut renderer, side, false) == original[index],
                    "Leeward walls must remain dry/clear"
                );
                graphics.weather.wind = 0;
                renderer.configure(&graphics);
                assert!(
                    capture(&mut renderer, side, false) == original[index],
                    "Calm vertical precipitation cannot hit vertical walls"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_WEATHER_REVIEW and a GPU; compares windward, leeward and sheltered Mission 1 walls/roofs"]
fn directional_weather_on_actual_walls_and_roofs() {
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
    let mesh = review.renderer.upload(&render_mesh::stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.set_filtering(2);
    graphics.weather.haze = 0;
    graphics.weather.reflections = 0;
    graphics.weather.wind = 100;
    graphics.weather.quality = 2;
    for (shot, eye, target) in [
        (
            "roof",
            Vec3::new(-48300.0, -7600.0, -39400.0),
            Vec3::new(-44667.0, -5000.0, -34667.0),
        ),
        (
            "walls",
            Vec3::new(-45500.0, -1800.0, -44700.0),
            Vec3::new(-42000.0, -800.0, -36500.0),
        ),
    ] {
        let camera = Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
            * Mat4::look_at_rh(eye, target, Vec3::NEG_Y);
        for (name, mode) in [
            ("original", crate::settings::WeatherMode::Original),
            ("rain", crate::settings::WeatherMode::Rain),
            ("snow", crate::settings::WeatherMode::Snow),
        ] {
            graphics.weather.mode = mode;
            review.renderer.weather.wet = f32::from(mode == crate::settings::WeatherMode::Rain);
            review.renderer.weather.snow = f32::from(mode == crate::settings::WeatherMode::Snow);
            for heading in [0, 180] {
                graphics.weather.heading = heading;
                review.configure(graphics.clone());
                review
                    .capture(
                        camera,
                        &[&mesh],
                        None,
                        true,
                        &output.join(format!("{shot}-{name}-{heading}.png")),
                    )
                    .unwrap();
            }
        }
    }
    assert_eq!(state, crate::session::tests::state(&game));
}
