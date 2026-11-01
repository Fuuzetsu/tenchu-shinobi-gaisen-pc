use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

fn horizontal(renderer: &mut Renderer, y: f32, x: [f32; 2], z: [f32; 2]) -> Mesh {
    let p = [
        [x[0], y, z[0]],
        [x[1], y, z[0]],
        [x[0], y, z[1]],
        [x[1], y, z[1]],
    ];
    renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
        position: p[i],
        color: [0.6; 3],
        opacity: 1.0,
        flags: render_mesh::SCENERY | render_mesh::DOUBLE_SIDED,
        ..Vertex::zeroed()
    }))
}

#[test]
#[ignore = "requires a GPU; rain haze must blend continuously when a view crosses a roof edge"]
fn rain_haze_crosses_shelter_without_bands() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let ground = horizontal(&mut renderer, 0.0, [-50000.0, 50000.0], [-50000.0, 50000.0]);
    let roof = horizontal(&mut renderer, -3000.0, [-50000.0, 0.0], [-50000.0, 50000.0]);
    let ceiling = horizontal(
        &mut renderer,
        -3000.0,
        [-50000.0, 50000.0],
        [-50000.0, 50000.0],
    );
    let camera = Mat4::perspective_rh(1.0, 1.0, 10.0, 80000.0)
        * Mat4::look_at_rh(
            Vec3::new(-1200.0, -1500.0, -1000.0),
            Vec3::new(2000.0, 0.0, 9000.0),
            Vec3::NEG_Y,
        );
    let capture = |renderer: &mut Renderer, cover: &Mesh| {
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[&ground], &[]),
                None,
                &[],
                &[&ground, cover],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut graphics = crate::settings::Graphics::default();
    graphics.weather.mode = crate::settings::WeatherMode::Rain;
    graphics.weather.wind = 0;
    graphics.weather.quality = 2;
    graphics.weather.haze = 0;
    renderer.configure(&graphics);
    let bare = capture(&mut renderer, &roof);
    graphics.weather.haze = 100;
    renderer.configure(&graphics);
    let covered = capture(&mut renderer, &ceiling);
    let haze = capture(&mut renderer, &roof);
    let row = |image: &[u8]| {
        (4..60)
            .map(|x| i16::from(image[(36 * 64 + x) * 4]))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        row(&bare),
        row(&covered),
        "An enclosed view must stay clear"
    );
    let row = row(&haze);
    let jump = row.windows(2).map(|p| (p[0] - p[1]).abs()).max().unwrap();
    assert!(
        row.iter().max().unwrap() - row.iter().min().unwrap() > 8,
        "{row:?}"
    );
    assert!(
        jump <= 4,
        "Haze crosses a roof edge in visible steps ({jump}): {row:?}"
    );
}

#[test]
#[ignore = "requires a GPU; rain wetness/puddles feather beneath eaves without soaking deep shelter"]
fn rain_surface_softens_eaves_without_soaking_interiors() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let ground = horizontal(&mut renderer, 0.0, [-10000.0, 10000.0], [-10000.0, 10000.0]);
    let roof = horizontal(&mut renderer, -1500.0, [-10000.0, 0.0], [-10000.0, 10000.0]);
    let camera = Mat4::orthographic_rh(-3000.0, 3000.0, -3000.0, 3000.0, 1.0, 20000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -10000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let capture = |renderer: &mut Renderer| {
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[&ground], &[]),
                None,
                &[],
                &[&ground, &roof],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer);
    let mut graphics = crate::settings::Graphics::default();
    graphics.weather.mode = crate::settings::WeatherMode::Rain;
    graphics.weather.wind = 0;
    graphics.weather.haze = 0;
    graphics.weather.reflections = 0;
    for quality in [0, 2] {
        graphics.weather.quality = quality;
        renderer.configure(&graphics);
        renderer.weather.wet = 1.0;
        renderer.weather.puddles = 1.0;
        let wet = capture(&mut renderer);
        let row: Vec<_> = (4..60)
            .map(|x| i16::from(original[(32 * 64 + x) * 4]) - i16::from(wet[(32 * 64 + x) * 4]))
            .collect();
        assert_eq!(row[4], 0, "Deep shelter must stay dry: {row:?}");
        assert!(
            *row.last().unwrap() > 25,
            "Open ground must get wet: {row:?}"
        );
        let jump = row.windows(2).map(|p| (p[0] - p[1]).abs()).max().unwrap();
        assert!(
            jump <= 16,
            "Rain edge at quality {quality} has a {jump}-level cutoff: {row:?}"
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_WEATHER_REVIEW and a GPU; review shelter in M2, M7 and TRIAL"]
fn rain_shelter_in_campaign_and_trial() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_WEATHER_REVIEW").expect("Set TENCHU_WEATHER_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    for mission in [2, 7, 0] {
        let mut game = if mission == 0 {
            let tables = tenchu_assets::trial::Tables::load(&assets).unwrap();
            let trial = &tables.presets(&assets).unwrap()[1];
            let theme = tenchu_assets::trial::Theme::load(&assets, trial.theme).unwrap();
            let mut random = tenchu_core::random::Random::new(1);
            let prepared = tenchu_assets::stage::MissionAssets::load_trial(
                &assets,
                &tables,
                &theme,
                trial,
                0,
                || random.next15() as u16,
            )
            .unwrap();
            tenchu_core::game::Game::with_random(
                prepared,
                &tenchu_assets::executable::Executable::new(&assets.executable).unwrap(),
                0,
                random,
            )
            .unwrap()
        } else {
            crate::session::Options {
                mission,
                ..Default::default()
            }
            .load(&assets)
            .unwrap()
        };
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let state = crate::session::tests::state(&game);
        let center = Vec3::from(game.player().position.map(|n| n as f32).to_array());
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
        graphics.weather.wind = 0;
        graphics.weather.quality = 2;
        graphics.weather.haze = 100;
        graphics.weather.reflections = 0;
        for (index, offset) in [
            Vec3::new(5000.0, -2000.0, -5000.0),
            Vec3::new(-5000.0, -2000.0, 5000.0),
        ]
        .into_iter()
        .enumerate()
        {
            let camera =
                Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
                    * Mat4::look_at_rh(center + offset, center, Vec3::NEG_Y);
            for (name, mode, wet, haze) in [
                ("dry", crate::settings::WeatherMode::Original, 0.0, 0),
                ("wet", crate::settings::WeatherMode::Rain, 1.0, 0),
                ("haze", crate::settings::WeatherMode::Rain, 1.0, 100),
            ] {
                graphics.weather.mode = mode;
                graphics.weather.haze = haze;
                review.renderer.weather.wet = wet;
                review.renderer.weather.puddles = wet;
                review.configure(graphics.clone());
                review
                    .capture(
                        camera,
                        &[&mesh],
                        None,
                        true,
                        &output.join(format!("mission-{mission}-{index}-{name}.png")),
                    )
                    .unwrap();
            }
        }
        assert_eq!(state, crate::session::tests::state(&game));
    }
}
