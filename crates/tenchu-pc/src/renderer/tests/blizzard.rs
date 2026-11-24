use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; Blizzard mist must follow shelter and match the direct portable path without altering other weather"]
fn blizzard_mist_respects_shelter_and_portable_rendering() {
    use crate::settings::{Graphics, WeatherMode};
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let horizontal = |renderer: &mut Renderer, y, high| {
        let points = [
            [-80000.0, y, -80000.0],
            [high, y, -80000.0],
            [-80000.0, y, 80000.0],
            [high, y, 80000.0],
        ];
        renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: points[i],
            color: [0.6; 3],
            normal: Vec3::NEG_Y.to_array(),
            opacity: 1.0,
            flags: render_mesh::SCENERY | render_mesh::DOUBLE_SIDED,
            ..Vertex::zeroed()
        }))
    };
    let ground = horizontal(&mut renderer, 0.0, 80000.0);
    let roof = horizontal(&mut renderer, -3000.0, 0.0);
    let ceiling = horizontal(&mut renderer, -3000.0, 80000.0);
    let camera = Mat4::perspective_rh(1.0, 1.0, 10.0, 100000.0)
        * Mat4::look_at_rh(
            Vec3::new(-1200.0, -1500.0, -1000.0),
            Vec3::new(2000.0, 0.0, 15000.0),
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
    let mut graphics = Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.weather.amount = 100;
    graphics.weather.quality = 2;
    graphics.weather.wind = 0;
    for mode in [
        WeatherMode::Original,
        WeatherMode::Clear,
        WeatherMode::Snow,
        WeatherMode::Snowstorm,
        WeatherMode::Blizzard,
    ] {
        graphics.weather.mode = mode;
        graphics.weather.haze = 0;
        renderer.configure(&graphics);
        let bare = capture(&mut renderer, &roof);
        graphics.weather.haze = 100;
        renderer.configure(&graphics);
        let covered = capture(&mut renderer, &ceiling);
        assert_eq!(covered, bare, "enclosed view stays clear in {mode:?}");
        let mist = capture(&mut renderer, &roof);
        if mode == WeatherMode::Blizzard {
            assert!(
                mist.iter()
                    .zip(&bare)
                    .filter(|(a, b)| a.abs_diff(**b) > 10)
                    .count()
                    > 1000,
                "open view is visibly veiled"
            );
        } else {
            assert_eq!(mist, bare, "existing {mode:?} mist behavior");
        }
    }
    for (wind, heading) in [(0, 0), (100, 90), (100, 270)] {
        graphics.weather.wind = wind;
        graphics.weather.heading = heading;
        renderer.configure(&graphics);
        renderer.skip_rain_haze = true;
        let direct = capture(&mut renderer, &roof);
        renderer.skip_rain_haze = false;
        let cached = capture(&mut renderer, &roof);
        let mut errors: Vec<_> = cached
            .chunks_exact(4)
            .zip(direct.chunks_exact(4))
            .map(|(a, b)| {
                a[..3]
                    .iter()
                    .zip(&b[..3])
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap()
            })
            .collect();
        errors.sort_unstable();
        let mean = errors.iter().map(|n| f32::from(*n)).sum::<f32>() / errors.len() as f32;
        assert!(
            mean < 1.0 && errors[errors.len() * 99 / 100] <= 8,
            "wind{wind}, heading{heading}: mean{mean}, max{}",
            errors.last().unwrap()
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_BLIZZARD_REVIEW and a GPU; matched maximum Blizzard settings over actual roofs and streets"]
fn blizzard_on_actual_roofs_and_streets() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_BLIZZARD_REVIEW").expect("TENCHU_BLIZZARD_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let game = crate::session::Options::default().load(&assets).unwrap();
    let state = postcard::to_allocvec(&game).unwrap();
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let mut graphics = crate::settings::Graphics::default();
    graphics.set_preset(crate::settings::presets::MODERN_HIGH);
    graphics.sky.enabled = true;
    graphics.sky.time = 720;
    graphics.weather.mode = crate::settings::WeatherMode::Blizzard;
    graphics.weather.amount = 100;
    graphics.weather.wind = 100;
    graphics.weather.heading = 90;
    graphics.weather.haze = 100;
    graphics.weather.quality = 2;
    graphics.weather.snow = 100;
    let models =
        crate::visuals::models::Models::build(&game.mission, &Default::default(), &graphics)
            .unwrap();
    let mut vertices = render_mesh::stage(&game.mission, &game.math, Some(&models));
    let surfaces = crate::visuals::surfaces::Surfaces::new(&vertices);
    vertices.extend(render_mesh::characters(&game, Some(&models)));
    let mesh = review.renderer.upload(&vertices);
    review.renderer.daylight = Some(crate::visuals::sky::Daylight::at(720.0, &graphics.sky));
    for (shot, eye, target) in [
        (
            "roof",
            Vec3::new(-48300.0, -7600.0, -39400.0),
            Vec3::new(-44667.0, -5000.0, -34667.0),
        ),
        (
            "street",
            Vec3::new(-45500.0, -1800.0, -44700.0),
            Vec3::new(-42000.0, -800.0, -36500.0),
        ),
    ] {
        let camera = Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
            * Mat4::look_at_rh(eye, target, Vec3::NEG_Y);
        let mut weather = crate::visuals::weather::Weather::default();
        weather.prewarm(game.tick, eye, &surfaces, &graphics.weather, 40);
        let weather_state = postcard::to_allocvec(&weather).unwrap();
        for haze in [0, 100] {
            graphics.weather.haze = haze;
            review.renderer.weather = weather.sample(&graphics.weather, 0.5);
            let geometry = weather.geometry(camera, 0.5, &graphics.weather);
            eprintln!(
                "Blizzard {shot}, haze {haze}: {} visible particle vertices, {} budget",
                geometry.vertices.len(),
                graphics.weather.particles()
            );
            let effects = review.renderer.upload_effects(geometry);
            review.configure(graphics.clone());
            review
                .capture(
                    camera,
                    &[&mesh],
                    Some(&effects),
                    true,
                    &output.join(format!("blizzard-{shot}-haze-{haze}.png")),
                )
                .unwrap();
        }
        assert_eq!(postcard::to_allocvec(&weather).unwrap(), weather_state);
    }
    assert_eq!(postcard::to_allocvec(&game).unwrap(), state);
}
