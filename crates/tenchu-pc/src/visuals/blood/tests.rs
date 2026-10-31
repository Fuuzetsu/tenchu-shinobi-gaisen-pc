use super::*;

#[test]
fn changing_one_blood_category_preserves_other_cached_marks() {
    let mut blood = Blood::default();
    let mut settings = settings::Blood::modern();
    blood.configure(&settings);
    for kind in [
        Kind::Ground,
        Kind::Body,
        Kind::Death { player: false },
        Kind::Death { player: true },
    ] {
        blood.retain(
            Mark(
                vec![Patch {
                    part: None,
                    vertices: vec![Vertex::zeroed(); 6],
                }],
                kind,
            ),
            8192,
        );
    }
    let page = blood.pages().next().unwrap().clone();
    settings.hit = BloodStyle::Off;
    settings.drips = BloodStyle::Original;
    blood.configure(&settings);
    assert!(std::sync::Arc::ptr_eq(&page, blood.pages().next().unwrap()));
    settings.ground = BloodStyle::Off;
    blood.configure(&settings);
    assert_eq!((blood.marks.len(), blood.marks.vertices), (3, 18));
    settings.player_death = false;
    blood.configure(&settings);
    assert_eq!((blood.marks.len(), blood.marks.vertices), (2, 12));
    settings.model_stains = false;
    blood.configure(&settings);
    assert_eq!(blood.marks.len(), 1);
    settings.death = BloodStyle::Off;
    blood.configure(&settings);
    assert!(blood.marks.is_empty());
}

#[test]
#[ignore = "requires TENCHU_DISC; independent styles, retained retail textures, finite drips and player death toggles without gameplay changes"]
fn original_and_modern_blood_styles_are_independent_and_persistent() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    for character in 0..2 {
        let mut game = crate::session::Options {
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let models = Models::build(
            &game.mission,
            &Default::default(),
            &settings::Graphics::default(),
        )
        .unwrap();
        let surfaces = Surfaces::new(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let position = Vec3::from(game.player().position.map(|v| v as f32).to_array());
        let mut blood = Blood::default();
        for ground in BloodStyle::ALL {
            for retain_original in [false, true] {
                let settings = settings::Blood {
                    hit: BloodStyle::Off,
                    death: BloodStyle::Off,
                    ground,
                    retain_original,
                    ..Default::default()
                };
                blood.tick(&game, &models, &surfaces, &settings);
                let state = crate::session::tests::state(&game);
                assert!(blood.stain(
                    &game,
                    &Receivers::new(&game, &models, &surfaces),
                    position - Vec3::Y * 500.0,
                    position + Vec3::Y * 500.0,
                    300.0,
                    8192
                ));
                assert_eq!(!blood.marks.is_empty(), settings.retained_ground());
                if settings.retained_ground() {
                    let page = blood.pages().next().unwrap().clone();
                    assert!(
                        page.iter().all(|v| (v.flags & render_mesh::SOFT != 0)
                            == (ground == BloodStyle::Modern))
                    );
                    if ground == BloodStyle::Original {
                        assert!(
                            page.iter().all(
                                |v| v.flags & render_mesh::TEXTURED != 0 && v.color == [1.0; 3]
                            )
                        );
                        let [u, v] = std::array::from_fn(|axis| {
                            page.iter()
                                .map(|v| v.uv[axis])
                                .fold(f32::INFINITY, f32::min) as u16
                        });
                        let [end_u, end_v] = std::array::from_fn(|axis| {
                            page.iter().map(|v| v.uv[axis]).fold(0.0, f32::max) as u16
                        });
                        let mut channels = [0u32; 3];
                        for y in v..=end_v {
                            for x in u..=end_u {
                                let pixel = game.mission.vram.texel(
                                    page[0].page as u16,
                                    page[0].palette as u16,
                                    [x, y],
                                );
                                for (axis, channel) in channels.iter_mut().enumerate() {
                                    *channel += u32::from((pixel >> (axis * 5)) & 31);
                                }
                            }
                        }
                        assert!(
                            channels[0] > channels[1] + channels[2],
                            "retained spots need the red disc artwork, not its white subtraction mask"
                        );
                    }
                    for _ in 0..300 {
                        game.tick += 1;
                        blood.tick(&game, &models, &surfaces, &settings);
                    }
                    assert!(std::sync::Arc::ptr_eq(&page, blood.pages().next().unwrap()));
                    game.tick -= 300;
                }
                assert!(state == crate::session::tests::state(&game));
                blood = Blood::default();
            }
        }
        for drips in [BloodStyle::Original, BloodStyle::Modern] {
            let settings = settings::Blood {
                hit: BloodStyle::Off,
                drips,
                ground: BloodStyle::Off,
                death: BloodStyle::Off,
                ..Default::default()
            };
            blood.tick(&game, &models, &surfaces, &settings);
            blood.wound(
                &game,
                &models,
                game.player().root_model,
                position - Vec3::Y * 750.0,
                &settings,
            );
            assert_eq!(blood.wounds.len(), 1);
            let mut seen = false;
            for _ in 0..180 {
                game.tick += 1;
                blood.tick(&game, &models, &surfaces, &settings);
                let geometry = blood.animated_geometry(&game, &models, None, None, 1.0);
                seen |= !geometry.vertices.is_empty();
                assert!(
                    geometry.vertices.iter().all(
                        |v| (v.flags & render_mesh::SOFT != 0) == (drips == BloodStyle::Modern)
                    )
                );
            }
            assert!(
                seen && blood.drops.is_empty() && blood.wounds.is_empty() && blood.marks.is_empty()
            );
            blood = Blood::default();
        }
        let player = game.roster.get_mut(game.player_id);
        player.health = 0;
        player.status = tenchu_core::actor::State::Dead;
        player.velocity = [0; 3];
        for death in BloodStyle::ALL {
            for player_death in [false, true] {
                let settings = settings::Blood {
                    hit: BloodStyle::Off,
                    ground: BloodStyle::Off,
                    death,
                    player_death,
                    retain_original: true,
                    ..Default::default()
                };
                blood.tick(&game, &models, &surfaces, &settings);
                for _ in 0..120 {
                    game.tick += 1;
                    let state = crate::session::tests::state(&game);
                    blood.tick(&game, &models, &surfaces, &settings);
                    assert!(state == crate::session::tests::state(&game));
                }
                assert_eq!(
                    blood.marks.len(),
                    usize::from(death != BloodStyle::Off && player_death)
                );
                if let Some(page) = blood.pages().next() {
                    assert!(page.iter().all(
                        |v| (v.flags & render_mesh::SOFT != 0) == (death == BloodStyle::Modern)
                    ));
                }
                blood.configure(&settings::Blood {
                    player_death: false,
                    ..settings
                });
                assert!(blood.marks.is_empty());
                blood = Blood::default();
            }
        }
        let hit = surfaces::Hit {
            position,
            normal: Vec3::new(-0.45, -0.85, 0.25).normalize(),
            fraction: 0.5,
            flags: 0,
        };
        let pool = Blood::original_pool(&game, game.player(), hit, 1.0, 1.0);
        let flat = Blood::original_pool(
            &game,
            game.player(),
            surfaces::Hit {
                normal: Vec3::NEG_Y,
                ..hit
            },
            1.0,
            1.0,
        );
        assert!(pool.len() > 0);
        assert!(
            pool.0
                .iter()
                .flat_map(|p| &p.vertices)
                .zip(flat.0.iter().flat_map(|p| &p.vertices))
                .all(|(v, reference)| {
                    let height = (Vec3::from(v.position) - hit.position).dot(hit.normal);
                    let reference_height = hit.position.y - reference.position[1];
                    (height - reference_height).abs() < 0.1
                }),
            "retained original stars must preserve their ground offset on slopes"
        );
    }
}

#[test]
fn retained_stains_respect_both_count_and_vertex_budgets() {
    let mut blood = Blood::default();
    for i in 0..4000 {
        blood.retain(
            Mark(
                vec![Patch {
                    part: (i % 3 == 0).then_some(Part::Actor {
                        root: i,
                        model: i + 1,
                        kind: 0,
                        inspection: false,
                    }),
                    vertices: vec![Vertex::zeroed(); if i % 2 == 0 { 30 } else { 3000 }],
                }],
                Kind::Ground,
            ),
            usize::from(u16::MAX),
        );
    }
    assert!(
        blood.marks.len() < 4000,
        "vertex budget must evict before the count limit"
    );
    let measured = blood.pages().map(|p| p.len()).sum::<usize>()
        + blood
            .marks
            .attached
            .iter()
            .map(|(_, m)| m.len())
            .sum::<usize>();
    assert_eq!(blood.marks.vertices, measured);
    assert!(measured <= VERTEX_LIMIT);
    blood.marks.retain_attached(|_| false);
    assert_eq!(
        blood.marks.vertices,
        blood.pages().map(|p| p.len()).sum::<usize>()
    );
    blood.trim(32);
    assert_eq!(blood.marks.len(), 32);
    blood.clear();
    assert!(blood.marks.is_empty());
    assert_eq!(blood.marks.vertices, 0);
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
    let settings = settings::Blood::modern();
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
            max_attached = max_attached.max(blood.marks.attached.len());
            max_world = max_world.max(blood.marks.len() - blood.marks.attached.len());
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
                let mesh = blood.geometry(&game, &models, None, Some(game.rendered_view), 0.5);
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
    let world_before = blood.marks.len() - blood.marks.attached.len();
    assert!(world_before > 0);
    let disabled = settings::Blood::default();
    blood.tick(&game, &models, &surfaces, &disabled);
    assert!(
        blood
            .geometry(&game, &models, None, Some(game.rendered_view), 1.0)
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
            blood.geometry(game, models, None, Some(game.rendered_view), 1.0)
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
        .attached
        .iter()
        .rev()
        .flat_map(|(_, m)| &m.0)
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
                    review.renderer.upload_effects(blood.geometry(
                        game,
                        models,
                        None,
                        Some(game.rendered_view),
                        1.0,
                    ))
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
        let effects = review.renderer.upload_effects(blood.geometry(
            game,
            models,
            None,
            Some(game.rendered_view),
            1.0,
        ));
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

#[test]
fn full_retention_keeps_old_stains_and_only_changes_the_active_page() {
    let mut blood = Blood::default();
    let start = std::time::Instant::now();
    let template = Mark(
        vec![Patch {
            part: None,
            vertices: vec![Vertex::zeroed(); 36],
        }],
        Kind::Ground,
    );
    for i in 0..u16::MAX {
        let mut vertices = template.0[0].vertices.clone();
        vertices[0].position[0] = f32::from(i);
        blood.retain(
            Mark(
                vec![Patch {
                    part: None,
                    vertices,
                }],
                Kind::Ground,
            ),
            usize::from(u16::MAX),
        );
    }
    assert_eq!(blood.marks.len(), usize::from(u16::MAX));
    assert_eq!(blood.pages().next().unwrap()[0].position[0], 0.0);
    let old_pages: Vec<_> = blood.pages().cloned().collect();
    blood.retain(template, usize::from(u16::MAX));
    let pages: Vec<_> = blood.pages().collect();
    assert_eq!(pages[0][0].position[0], 1.0);
    assert_eq!(blood.marks.vertices, usize::from(u16::MAX) * 36);
    assert!(
        old_pages[1..old_pages.len() - 1]
            .iter()
            .zip(&pages[1..pages.len() - 1])
            .all(|(a, b)| std::sync::Arc::ptr_eq(a, b))
    );
    eprintln!(
        "65,535 retained stains: {} vertices, {} pages, {:?} to insert and evict",
        blood.marks.vertices,
        pages.len(),
        start.elapsed()
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; checks finite wound bleeding, immediate duration changes and player death pools without gameplay mutations"]
fn wound_drips_stop_and_player_death_leaves_a_pool() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    for character in 0..2 {
        let mut game = crate::session::Options {
            mission: 1,
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let graphics = settings::Graphics::default();
        let models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
        let surfaces = Surfaces::new(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let mut blood = Blood::default();
        let mut settings = settings::Blood::modern();
        blood.tick(&game, &models, &surfaces, &settings);
        let root = game.player().root_model;
        let position =
            Vec3::from(game.player().position.map(|v| v as f32).to_array()) - Vec3::Y * 750.0;
        game.effects.slots[200] = Some(tenchu_core::effects::Effect::Blood(
            tenchu_core::effects::Blood {
                hint: None,
                position: tenchu_core::math::Position::from(position.to_array().map(|v| v as i32)),
                velocity: [0; 3],
                scale: 8192,
                rotation: 0,
                brightness: 128,
                sprite: 0,
                gore: false,
                time: 20,
                mode: 0,
            },
        ));
        game.roster.get_mut(game.player_id).health -= 1;
        game.tick += 1;
        blood.tick(&game, &models, &surfaces, &settings);
        assert!(
            blood.wounds.is_empty(),
            "discarded blood must not create wounds"
        );
        assert!(blood.samples.iter().all(Option::is_none));
        blood.wound(&game, &models, root, position, &settings);
        assert_eq!(blood.wounds.len(), 1);
        let mut emitted = false;
        for _ in 0..150 {
            game.tick += 1;
            let state = crate::session::tests::state(&game);
            blood.tick(&game, &models, &surfaces, &settings);
            assert_eq!(state, crate::session::tests::state(&game));
            emitted |= !blood.drops.is_empty();
        }
        assert!(emitted);
        assert!(blood.wounds.is_empty() && blood.drops.is_empty());
        assert!(blood.marks.len() > 1);
        settings.seconds = 60;
        blood.wound(&game, &models, root, position, &settings);
        game.tick += 1;
        blood.tick(&game, &models, &surfaces, &settings);
        assert!(!blood.wounds.is_empty());
        settings.seconds = 0;
        game.tick += 1;
        blood.tick(&game, &models, &surfaces, &settings);
        assert!(blood.wounds.is_empty());
        blood.clear();
        let player = game.roster.get_mut(game.player_id);
        player.health = 0;
        player.status = tenchu_core::actor::State::Dead;
        player.velocity = [0; 3];
        for _ in 0..120 {
            game.tick += 1;
            let state = crate::session::tests::state(&game);
            blood.tick(&game, &models, &surfaces, &settings);
            assert_eq!(state, crate::session::tests::state(&game));
        }
        assert!(blood.pools.is_empty(), "pool growth must finish");
        assert_eq!(blood.marks.len(), 1, "one retained pool per dead player");
        assert!(!blood.pages().next().unwrap().is_empty());
        blood.clear();
        game.tick += 1;
        blood.tick(&game, &models, &surfaces, &settings);
        assert!(
            blood.marks.is_empty() && blood.pools.is_empty(),
            "clear must not immediately recreate a corpse pool"
        );
    }
}
