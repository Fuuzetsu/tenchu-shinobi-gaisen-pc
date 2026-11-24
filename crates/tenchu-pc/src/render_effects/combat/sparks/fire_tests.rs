use super::*;
use crate::{render_effects::Style, render_mesh};
use tenchu_core::{
    effects::{self, Graphic},
    math::{Matrix, Position},
    terrain::Cursor,
};

#[test]
#[ignore = "requires TENCHU_DISC; bonfire embers retain source motion, expiry and RNG across presentation settings"]
fn fire_sparks_follow_source_slots_and_gravity_order() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    game.effects = Default::default();
    game.rendered_view = Matrix::default();
    game.effects.bleeds(
        Position::new(100, -200, 2000),
        [0, -60, 0],
        effects::Burst {
            spread: 100,
            count: 10,
            lifetime: 30,
            color: [100, 100, 60],
        },
        &mut game.random,
    );
    let rng = game.random;
    let style = Style {
        sparks: Some(100),
        ..Default::default()
    };
    let mut newborn = Vec::new();
    super::super::draw(&mut newborn, &game, Matrix::default(), style, None);
    assert!(
        newborn.is_empty(),
        "bonfire slots born after DrawBleed wait until the next source update"
    );
    assert!(style.replaces(&Graphic::Bleed, [100, 100, 60]));
    assert!(!Style::default().replaces(&Graphic::Bleed, [100, 100, 60]));
    for graphic in [Graphic::GunBleed, Graphic::Smoke(0)] {
        assert!(!style.replaces(&graphic, [100, 100, 60]));
    }
    for color in [[128, 0, 0], [255, 255, 120], [100, 100, 61]] {
        assert!(!style.replaces(&Graphic::Bleed, color));
    }
    let mut cursor = Cursor::new(&game.mission.terrain);
    for tick in 0..32 {
        let previous = game.effects.active_slots().to_vec();
        game.effects.begin_tick();
        game.effects
            .step(
                &mut game.random,
                effects::View {
                    position: Position::ZERO,
                    matrix: Matrix::default(),
                },
                &mut game.scene,
                &mut effects::Environment {
                    terrain: &game.mission.terrain,
                    cursor: &mut cursor,
                    clock: tick,
                    owner_position: Position::ZERO,
                    reference: Position::ZERO,
                },
            )
            .unwrap();
        let state = postcard::to_allocvec(&game).unwrap();
        let live: Vec<_> = game
            .effects
            .active_slots()
            .iter()
            .enumerate()
            .filter_map(|(slot, effect)| match effect {
                Some(Effect::Bleed(spark)) => Some((slot, spark)),
                _ => None,
            })
            .collect();
        for density in [0, 100, 300, u16::MAX] {
            for alpha in [0.0, 0.25, 0.5, 1.0] {
                let mut surfaces = Vec::new();
                super::super::draw(
                    &mut surfaces,
                    &game,
                    Matrix::default(),
                    Style {
                        alpha,
                        sparks: Some(density),
                        ..style
                    },
                    None,
                );
                assert_eq!(
                    surfaces.len(),
                    live.len() * usize::from(density.min(300) / 100) * 2
                );
                assert!(surfaces.iter().flat_map(|s| &s.vertices).all(|v| {
                    v.position.iter().all(|p| p.is_finite())
                        && v.flags & (render_mesh::SOFT | render_mesh::UNLIT)
                            == render_mesh::SOFT | render_mesh::UNLIT
                        && v.flags & (render_mesh::TEXTURED | render_mesh::SCREEN) == 0
                        && (0.0..=1.0).contains(&v.opacity)
                        && v.color[0] > v.color[1]
                        && v.color[1] > v.color[2]
                }));
                if density == 100 {
                    for ((slot, current), pair) in live.iter().zip(surfaces.chunks_exact(2)) {
                        let Some(Effect::Bleed(previous)) = &previous[*slot] else {
                            unreachable!()
                        };
                        let from = Vec3::from(previous.position.map(|n| n as f32).to_array());
                        let to = Vec3::from(current.position.map(|n| n as f32).to_array());
                        let center = pair[1]
                            .vertices
                            .iter()
                            .map(|v| Vec3::from(v.position))
                            .sum::<Vec3>()
                            / 6.0;
                        assert!(center.distance(from.lerp(to, alpha)) < 0.001);
                    }
                }
            }
        }
        if tick == 0 {
            for camera in [None, Some(Matrix::default())] {
                let original = crate::render_effects::build_presented(
                    &game,
                    false,
                    camera,
                    false,
                    None,
                    None,
                    Style::default(),
                );
                let modern = crate::render_effects::build_presented(
                    &game, false, camera, false, None, None, style,
                );
                let source_color = [100.0 / 255.0, 100.0 / 255.0, 60.0 / 255.0];
                let squares = |geometry: &crate::render_effects::Geometry| {
                    geometry
                        .vertices
                        .iter()
                        .filter(|v| v.flags & render_mesh::TEXTURED == 0 && v.color == source_color)
                        .count()
                };
                assert_eq!(squares(&original), 60, "ten source quads in either camera");
                assert_eq!(
                    squares(&modern),
                    0,
                    "no original squares beneath modern embers"
                );
            }
        }
        assert_eq!(postcard::to_allocvec(&game).unwrap(), state);
        assert_eq!(game.random, rng);
    }
    assert!(game.effects.active_slots().iter().all(Option::is_none));
    game.effects.slots[200] = Some(Effect::Bleed(effects::Bleed {
        position: Position::new(0, 0, 2000),
        velocity: [0, -59, 0],
        color: [100, 100, 60],
        time: 30,
        mode: 0,
    }));
    let mut surfaces = Vec::new();
    super::super::draw(&mut surfaces, &game, Matrix::default(), style, None);
    assert!(
        surfaces.is_empty(),
        "overflow slots never produce immortal embers"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; optional TENCHU_FIRE_SPARK_REVIEW captures matched source Mission 5 execution frames on a GPU"]
fn execution_fire_sparks_use_the_authored_bonfire() {
    use tenchu_assets::{
        executable::Executable,
        game::GameAssets,
        misc::Kind,
        script::{Event, Language},
        stage::MissionAssets,
    };
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut mission = MissionAssets::load_localized(&assets, 10, 0, 1, Language::Japanese).unwrap();
    mission.scripts.events = vec![Event {
        id: 0,
        sequence: 100,
        next: [255; 2],
        target: 255,
        mode: 0,
        value: 0,
        bounds: [[0; 2]; 3],
    }];
    let mut game = Game::new(mission, &Executable::new(&assets.executable).unwrap(), 0, 1).unwrap();
    game.combat.dismemberment.execution_scene = true;
    let directory = std::env::var_os("TENCHU_FIRE_SPARK_REVIEW").map(std::path::PathBuf::from);
    if let Some(directory) = &directory {
        std::fs::create_dir_all(directory).unwrap();
    }
    let mut captured = 0;
    let mut live_frames = 0;
    for _ in 0..1400 {
        game.step(0).unwrap();
        if !game.cinematic.active {
            continue;
        }
        let live = game
            .effects
            .active_slots()
            .iter()
            .flatten()
            .filter(|effect| matches!(effect, Effect::Bleed(spark) if fire_spark(spark.color)))
            .count();
        live_frames += usize::from(live > 0);
        if ![1042, 1176, 1264].contains(&game.cinematic.elapsed) {
            continue;
        }
        assert!(
            game.misc
                .objects
                .iter()
                .any(|object| matches!(object.kind, Kind::Bonfire(_)))
        );
        assert!(
            live > 0,
            "authored fire particles at {}",
            game.cinematic.elapsed
        );
        let state = postcard::to_allocvec(&game).unwrap();
        let mut settings = crate::settings::Settings::default();
        settings
            .graphics
            .set_preset(crate::settings::presets::MODERN_HIGH);
        for enabled in [false, true] {
            settings.graphics.effects.sparks = enabled;
            let mut surfaces = Vec::new();
            super::super::draw(
                &mut surfaces,
                &game,
                game.rendered_view,
                Style {
                    sparks: enabled.then_some(settings.graphics.effects.spark_density),
                    ..Default::default()
                },
                None,
            );
            assert_eq!(surfaces.is_empty(), !enabled);
            if let Some(directory) = &directory {
                crate::capture::frame_with_settings(
                    &game,
                    &directory.join(format!(
                        "execution-{}-sparks-{enabled}.png",
                        game.cinematic.elapsed
                    )),
                    &assets,
                    &settings,
                )
                .unwrap();
            }
        }
        assert_eq!(postcard::to_allocvec(&game).unwrap(), state);
        eprintln!(
            "Execution frame {}: {live} live source fire particles",
            game.cinematic.elapsed
        );
        captured += 1;
    }
    assert_eq!(captured, 3);
    assert!(live_frames > 1000);
}
