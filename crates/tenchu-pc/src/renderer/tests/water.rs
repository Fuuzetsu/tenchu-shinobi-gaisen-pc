use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; water animation affects only tagged geometry and restores exact pixels when disabled"]
fn water_ripples_are_visual_scoped_and_optional() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let mut vertices = Vec::new();
    for (left, right, flag) in [(-1500.0, 0.0, render_mesh::WATER), (0.0, 1500.0, 0)] {
        let p = [
            [left, 0.0, -2000.0],
            [right, 0.0, -2000.0],
            [left, 0.0, 2000.0],
            [right, 0.0, 2000.0],
        ];
        vertices.extend([0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: p[i],
            color: [0.2, 0.3, 0.35],
            flags: render_mesh::DOUBLE_SIDED | flag,
            ..Vertex::zeroed()
        }));
    }
    let mesh = renderer.upload(&vertices);
    let camera = Mat4::perspective_rh(60.0_f32.to_radians(), 1.0, 10.0, 10000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -700.0, -2500.0), Vec3::ZERO, Vec3::NEG_Y);
    let capture = |renderer: &mut Renderer| {
        renderer
            .draw_lit(&view, camera, &[&mesh], None, &[], &[])
            .unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer);
    let mut graphics = crate::settings::Graphics::default();
    graphics.water.enabled = true;
    graphics.water.ripples = 100;
    graphics.water.reflections = 100;
    renderer.configure(&graphics);
    let first = capture(&mut renderer);
    renderer.visual_time = 1.0;
    let second = capture(&mut renderer);
    assert!(
        first
            .iter()
            .zip(&second)
            .filter(|(a, b)| a.abs_diff(**b) > 1)
            .count()
            > 30
    );
    for y in 0..64 {
        for x in 33..64 {
            let i = (y * 64 + x) * 4;
            assert_eq!(&original[i..i + 4], &second[i..i + 4]);
        }
    }
    renderer.configure(&crate::settings::Graphics::default());
    assert_eq!(original, capture(&mut renderer));
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_WATER_REVIEW and a GPU; reviews actual waterfall bends, surface flow and gameplay state"]
fn water_and_waterfalls_on_actual_geometry() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_WATER_REVIEW").expect("Set TENCHU_WATER_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    for mission in [1, 4] {
        let mut game = crate::session::Options {
            mission,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        for _ in 0..127 {
            game.step(0).unwrap();
        }
        let state = crate::session::tests::state(&game);
        let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
        for (label, enabled, detail) in [
            ("original", false, 0),
            ("shading", true, 0),
            ("curved", true, 1),
            ("fine", true, 2),
        ] {
            let mut graphics = crate::settings::Graphics::default();
            graphics.set_preset(3);
            graphics.fog = false;
            graphics.water.enabled = enabled;
            graphics.water.detail = detail;
            if mission == 1 {
                graphics.sky.enabled = true;
                review.renderer.daylight =
                    Some(crate::visuals::sky::Daylight::at(570.0, &graphics.sky));
            } else {
                review.renderer.daylight = None;
            }
            review.configure(graphics.clone());
            let models = review.models(&game.mission).unwrap();
            let vertices = render_mesh::stage(&game.mission, &game.math, Some(&models));
            let stage = review.renderer.upload(&vertices);
            let shadow_vertices =
                render_mesh::shadow_stage(&game.mission, &game.math, Some(&models));
            let spray = crate::visuals::water::Spray::new(&shadow_vertices);
            let water: Vec<_> = shadow_vertices
                .iter()
                .filter(|v| v.flags & render_mesh::WATER != 0)
                .collect();
            if enabled {
                assert!(!water.is_empty());
                let min = water.iter().fold(Vec3::splat(f32::INFINITY), |p, v| {
                    p.min(Vec3::from(v.position))
                });
                let max = water.iter().fold(Vec3::splat(f32::NEG_INFINITY), |p, v| {
                    p.max(Vec3::from(v.position))
                });
                eprintln!(
                    "{label}: {} waterfall vertices, {min:?} .. {max:?}",
                    water.len()
                );
            }
            let shots = if mission == 1 {
                vec![(
                    "pond",
                    Vec3::new(-40000.0, -1800.0, -16500.0),
                    Vec3::new(-31000.0, 0.0, -16500.0),
                )]
            } else {
                vec![
                    (
                        "bend",
                        Vec3::new(-21961.41, -12031.06, 33448.08),
                        Vec3::new(-18116.666, -11333.333, 34916.668),
                    ),
                    (
                        "wide",
                        Vec3::new(-25439.89, -13127.48, 29309.41),
                        Vec3::new(-18116.666, -11333.333, 34916.668),
                    ),
                ]
            };
            for (shot, eye, target) in shots {
                let camera =
                    Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
                        * Mat4::look_at_rh(eye, target, Vec3::NEG_Y);
                let mut geometry = crate::render_effects::build_presented(
                    &game,
                    false,
                    None,
                    false,
                    Some(&models),
                    None,
                    false,
                );
                geometry.append(spray.geometry(5.0, camera, &graphics.water));
                geometry.hide_ui();
                eprintln!(
                    "{label}/{shot}: {} cosmetic vertices",
                    geometry.vertices.len()
                );
                let effects = review.renderer.upload_effects(geometry);
                review.renderer.visual_time = 5.0;
                review
                    .capture(
                        camera,
                        &[&stage],
                        Some(&effects),
                        true,
                        &output.join(format!("m{mission}-{shot}-{label}.png")),
                    )
                    .unwrap();
            }
        }
        assert_eq!(state, crate::session::tests::state(&game));
    }
}
