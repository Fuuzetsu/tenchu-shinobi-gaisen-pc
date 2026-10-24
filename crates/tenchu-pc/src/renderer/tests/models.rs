use super::*;
use glam::Vec3;

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_MODEL_REVIEW and a GPU; reviews refinement and foliage in actual missions without changing gameplay"]
fn refined_characters_and_foliage_preserve_game_state() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_MODEL_REVIEW").expect("Set TENCHU_MODEL_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    for (mission, character) in [(1, 0), (1, 1), (4, 0), (5, 1)] {
        let mut game = crate::session::Options {
            mission,
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let state = crate::session::tests::state(&game);
        let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
        let head = game.player().rendered[2];
        let rotation = crate::visuals::models::rotation(head);
        let center = Vec3::from(head.translation.map(|v| v as f32).to_array());
        let mut shots = vec![
            (
                "portrait",
                center + rotation * Vec3::new(250.0, -50.0, -600.0),
                center + rotation * Vec3::new(0.0, -30.0, 0.0),
            ),
            (
                "body",
                center + rotation * Vec3::new(-750.0, 180.0, -2000.0),
                center + Vec3::Y * 400.0,
            ),
        ];
        if mission == 1 {
            shots.push((
                "garden",
                Vec3::new(-40000.0, -1800.0, -16500.0),
                Vec3::new(-31000.0, -1000.0, -16500.0),
            ));
        }
        for (name, level) in [("original", 0), ("refined", 1), ("fine", 2)] {
            let mut graphics = crate::settings::Graphics::default();
            graphics.set_preset(3);
            graphics.fog = false;
            graphics.models.characters = level;
            graphics.models.foliage = level;
            graphics.lighting.enabled = true;
            graphics.lighting.directional.intensity = 40;
            graphics.lighting.ambient = 75;
            review.configure(graphics);
            let models = review.models(&game.mission).unwrap();
            let stage = render_mesh::stage(&game.mission, &game.math, Some(&models));
            let actors = render_mesh::characters_presented(&game, Some(&models), true, None);
            eprintln!(
                "mission {mission} character {character} {name}: {} stage and {} actor vertices",
                stage.len(),
                actors.len()
            );
            assert!(stage.iter().chain(&actors).all(|v|Vec3::from(v.position).is_finite() && Vec3::from(v.normal).is_finite()));
            let stage = review.renderer.upload(&stage);
            let actors = review.renderer.upload(&actors);
            for &(shot, eye, target) in &shots {
                let camera =
                    Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
                        * Mat4::look_at_rh(eye, target, Vec3::NEG_Y);
                let mut effects = crate::render_effects::build_presented(
                    &game,
                    false,
                    None,
                    false,
                    Some(&models),
                    None,
                    false,
                );
                effects.hide_ui();
                let effects = review.renderer.upload_effects(effects);
                review
                    .capture(
                        camera,
                        &[&stage, &actors],
                        Some(&effects),
                        true,
                        &output.join(format!("m{mission}-c{character}-{shot}-{name}.png")),
                    )
                    .unwrap();
            }
            if character == 0 || mission == 5 {
                let labels: &[&str] = match mission {
                    1 => &["SAKURA", "TEIBOKU", "TUTUZI"],
                    4 => &["KSUGI", "KUSAA"],
                    _ => &["MUHYO00"],
                };
                for source in crate::visuals::catalog::models(&game.mission)
                    .into_iter()
                    .filter(|s| labels.contains(&s.label.as_str()))
                {
                    let mut vertices = Vec::new();
                    render_mesh::append_geometry(
                        &mut vertices,
                        source.geometry,
                        Matrix::default(),
                        Some(&models),
                    );
                    let (min, max) = vertices.iter().fold(
                        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                        |(min, max), v| {
                            (
                                min.min(Vec3::from(v.position)),
                                max.max(Vec3::from(v.position)),
                            )
                        },
                    );
                    let center = (min + max) * 0.5;
                    let radius = (max - min).length() * 0.5;
                    let camera =
                        Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
                            * Mat4::look_at_rh(
                                center + Vec3::new(-0.7, -0.3, -1.0).normalize() * radius * 2.8,
                                center,
                                Vec3::NEG_Y,
                            );
                    let mesh = review.renderer.upload(&vertices);
                    let backdrop = review.backdrop(0.2);
                    review
                        .capture(
                            camera,
                            &[&backdrop, &mesh],
                            None,
                            true,
                            &output.join(format!("m{mission}-{}-{name}.png", source.label)),
                        )
                        .unwrap();
                }
            }
        }
        assert_eq!(state, crate::session::tests::state(&game));
    }
}
