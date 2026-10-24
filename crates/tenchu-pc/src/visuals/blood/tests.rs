use super::*;

#[test]
fn retained_stains_respect_both_count_and_vertex_budgets() {
    let mut blood = Blood::default();
    for i in 0..2000 {
        blood.retain(
            Mark(vec![Patch {
                part: None,
                vertices: vec![Vertex::zeroed(); if i % 2 == 0 { 30 } else { 3000 }],
            }]),
            1024,
        );
    }
    assert!(blood.marks.len() <= 1024);
    assert!(blood.marks.iter().map(Mark::len).sum::<usize>() <= VERTEX_LIMIT);
    blood.trim(32);
    assert!(blood.marks.len() <= 32);
    blood.clear();
    assert!(blood.marks.is_empty());
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_RECORDING; real combat must make attached wounds and persistent ground stains without changing replay state"]
fn combat_replay_stains_surfaces_without_changing_gameplay() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let text = std::fs::read_to_string(
        std::env::var_os("TENCHU_RECORDING").expect("Set TENCHU_RECORDING"),
    )
    .unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let start = crate::recording::Start::parse(&text).unwrap().unwrap();
    let mut game = start.load(&assets).unwrap();
    let mut original = start.load(&assets).unwrap();
    let output = std::env::var_os("TENCHU_BLOOD_REVIEW").map(std::path::PathBuf::from);
    let mut review = output.as_ref().map(|path| {
        std::fs::create_dir_all(path).unwrap();
        crate::capture::review::Review::new(&game.mission).unwrap()
    });
    let mut graphics = settings::Graphics::default();
    graphics.water.enabled = true;
    graphics.models.characters = 1;
    graphics.models.foliage = 1;
    graphics.motion = settings::Motion {
        hair: true,
        cloth: true,
        equipment: true,
        wind: 60,
        ..Default::default()
    };
    if std::env::var_os("TENCHU_BLOOD_REVIEW_SNOW").is_some() {
        graphics.weather.mode = settings::WeatherMode::Snow;
        if let Some(review) = &mut review {
            review.renderer.weather.snow = 1.0;
        }
    }
    if let Some(review) = &mut review {
        review.configure(graphics.clone());
    }
    let mut models = review.as_mut().map_or_else(
        || Models::build(&game.mission, &Default::default(), &graphics).unwrap(),
        |review| review.models(&game.mission).unwrap(),
    );
    let surfaces = Surfaces::new(&render_mesh::stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let mut weather = crate::visuals::weather::Weather::default();
    let mut weather_settings = settings::Weather {
        mode: settings::WeatherMode::Rain,
        ..Default::default()
    };
    let mut sky = crate::visuals::sky::Clock::default();
    let sky_settings = settings::Sky {
        enabled: true,
        speed: 72,
        ..Default::default()
    };
    let mut blood = Blood::default();
    let mut motion = crate::visuals::motion::Motion::default();
    let settings = settings::Blood {
        enabled: true,
        ..Default::default()
    };
    blood.tick(&game, &models, &surfaces, &settings);
    let mut max_wounds = 0;
    let mut max_world = 0;
    let mut max_attached = 0;
    let mut last_count = 0;
    let mut capture_tick = None;
    let mut captures = 0;
    for (count, buttons) in crate::session::parse_inputs(&text).unwrap() {
        for _ in 0..count {
            game.step(buttons).unwrap();
            original.step(buttons).unwrap();
            motion.tick(&game, &models, &graphics.motion);
            models.motion = motion.sample(1.0);
            blood.tick(&game, &models, &surfaces, &settings);
            weather_settings.mode = if game.tick % 1800 < 900 {
                settings::WeatherMode::Rain
            } else {
                settings::WeatherMode::Snow
            };
            let camera = crate::renderer::view_matrix(game.rendered_view)
                .inverse()
                .transform_point3(Vec3::ZERO);
            weather.tick(game.tick, camera, &surfaces, &weather_settings);
            sky.tick(game.tick, &sky_settings);
            for alpha in [0.0, 0.5, 1.0] {
                weather.sample(&weather_settings, alpha);
                sky.sample(&sky_settings, alpha);
            }
            max_wounds = max_wounds.max(blood.wounds.len());
            max_attached = max_attached.max(
                blood
                    .marks
                    .iter()
                    .filter(|m| m.0.iter().any(|p| p.part.is_some()))
                    .count(),
            );
            max_world = max_world.max(
                blood
                    .marks
                    .iter()
                    .filter(|m| m.0.iter().any(|p| p.part.is_none()))
                    .count(),
            );
            if blood.wounds.len() > last_count {
                eprintln!(
                    "tick {}: {} wounds, {} marks",
                    game.tick,
                    blood.wounds.len(),
                    blood.marks.len()
                );
            }
            if blood.wounds.len() > last_count && capture_tick.is_none() && captures < 3 {
                capture_tick = Some(game.tick + 20);
            }
            if capture_tick == Some(game.tick) {
                if let (Some(review), Some(output)) = (&mut review, &output) {
                    capture(review, output, &game, &models, &blood);
                }
                capture_tick = None;
                captures += 1;
            }
            last_count = blood.wounds.len();
            if game.tick.is_multiple_of(60) {
                assert_eq!(
                    crate::session::tests::state(&game),
                    crate::session::tests::state(&original)
                );
                for alpha in [0.0, 0.5, 1.0] {
                    models.motion = motion.sample(alpha);
                    let vertices =
                        render_mesh::characters_presented(&game, Some(&models), true, None);
                    assert!(
                        vertices.iter().all(|v| v
                            .position
                            .iter()
                            .chain(&v.normal)
                            .all(|f| f.is_finite()))
                    );
                }
                let mesh = blood.geometry(&game, &models, None, true, 0.5);
                assert!(mesh.vertices.len() <= VERTEX_LIMIT + DROP_LIMIT * 6);
                assert!(
                    mesh.vertices
                        .iter()
                        .all(|v| v.position.iter().all(|c| c.is_finite()))
                );
            }
        }
    }
    assert!(
        max_attached > 0,
        "combat must retain actual attached geometry"
    );
    assert!(
        max_wounds > 0,
        "recording must include actual combat wounds"
    );
    assert!(
        max_world > 0,
        "recording must include blood landing on world geometry"
    );
    assert_eq!(
        crate::session::tests::state(&game),
        crate::session::tests::state(&original)
    );
    let world_before = blood
        .marks
        .iter()
        .filter(|m| m.0.iter().any(|p| p.part.is_none()))
        .count();
    assert!(world_before > 0);
    let disabled = settings::Blood::default();
    blood.tick(&game, &models, &surfaces, &disabled);
    assert!(
        blood
            .geometry(&game, &models, None, true, 1.0)
            .vertices
            .is_empty()
    );
    blood.tick(&game, &models, &surfaces, &settings);
    assert!(
        blood.marks.is_empty(),
        "re-enabling must not resurrect old wounds"
    );
    eprintln!(
        "{} ticks; maximum {max_wounds} wounds, {max_world} world stains",
        game.tick
    );
}

fn capture(
    review: &mut crate::capture::review::Review,
    output: &std::path::Path,
    game: &Game,
    models: &Models,
    blood: &Blood,
) {
    let renderer = &mut review.renderer;
    renderer.update_vram(&game.mission.vram);
    let stage = renderer.upload(&render_mesh::stage(&game.mission, &game.math, Some(models)));
    let actors = renderer.upload(&render_mesh::characters_presented(
        game,
        Some(models),
        true,
        None,
    ));
    let camera = crate::renderer::game_camera(game.rendered_view, 1280.0 / 720.0);
    for enabled in [false, true] {
        let mut geometry = if enabled {
            blood.geometry(game, models, None, true, 1.0)
        } else {
            Geometry {
                vertices: Vec::new(),
                batches: Vec::new(),
            }
        };
        geometry.append(crate::render_effects::build_options(
            game,
            false,
            None,
            true,
            Some(models),
        ));
        let effects = review.renderer.upload_effects(geometry);
        review
            .capture(
                camera,
                &[&stage, &actors],
                Some(&effects),
                true,
                &output.join(format!(
                    "{}-{}.png",
                    game.tick,
                    if enabled { "blood" } else { "original" }
                )),
            )
            .unwrap();
    }
    let poses = transforms(game, models, None, true);
    if let Some(patch) = blood
        .marks
        .iter()
        .rev()
        .flat_map(|m| &m.0)
        .find(|p| p.part.is_some())
        && let Some(matrix) = patch.part.and_then(|key| poses.get(&key))
    {
        let points: Vec<_> = patch
            .vertices
            .iter()
            .map(|v| rotation(*matrix) * Vec3::from(v.position) + translation(*matrix))
            .collect();
        let center = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        for side in [-1.0, 1.0] {
            let camera =
                glam::Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 30000.0)
                    * glam::Mat4::look_at_rh(
                        center + normal * side * 1200.0 - Vec3::Y * 100.0,
                        center,
                        Vec3::NEG_Y,
                    );
            for enabled in [false, true] {
                let effects = enabled.then(|| {
                    review
                        .renderer
                        .upload_effects(blood.geometry(game, models, None, true, 1.0))
                });
                review
                    .capture(
                        camera,
                        &[&actors],
                        effects.as_ref(),
                        true,
                        &output.join(format!("{}-wound-{side}-{enabled}.png", game.tick)),
                    )
                    .unwrap();
            }
        }
    }
    if let Some(Wound {
        part: Part::Actor { root, .. },
        ..
    }) = blood.wounds.back()
        && let Some(actor) = game
            .roster
            .actors
            .iter()
            .flatten()
            .find(|actor| actor.root_model == *root)
    {
        let center = Vec3::from(actor.position.map(|p| p as f32).to_array()) - Vec3::Y * 700.0;
        let camera =
            glam::Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 30000.0)
                * glam::Mat4::look_at_rh(
                    center + Vec3::new(-1800.0, -600.0, -2000.0),
                    center,
                    Vec3::NEG_Y,
                );
        let effects = review
            .renderer
            .upload_effects(blood.geometry(game, models, None, true, 1.0));
        review
            .capture(
                camera,
                &[&actors],
                Some(&effects),
                true,
                &output.join(format!("{}-body.png", game.tick)),
            )
            .unwrap();
    }
}
