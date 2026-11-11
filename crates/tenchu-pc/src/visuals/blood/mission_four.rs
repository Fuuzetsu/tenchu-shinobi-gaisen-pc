use super::*;
use tenchu_core::math::Position;

#[test]
#[ignore = "requires TENCHU_DISC; issue #65 blood on Mission 4's blended ground, optional GPU captures"]
fn blood_stains_the_mission_four_route() {
    let disc = std::env::var_os("TENCHU_DISC").unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options {
        mission: 4,
        ..Default::default()
    }
    .load(&assets)
    .unwrap();
    game.events = Default::default();
    let output = std::env::var_os("TENCHU_ISSUE65_REVIEW").map(std::path::PathBuf::from);
    if let Some(output) = &output {
        std::fs::create_dir_all(output).unwrap();
    }
    for prepared in [false, true] {
        let graphics = settings::Graphics {
            fog: false,
            smooth_models: prepared,
            ..Default::default()
        };
        let models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
        let surface = Surfaces::stage(&game.mission, &game.math, Some(&models));
        for position in [
            [-49197, 8000, 2251],
            [-50702, 8000, -85],
            [-47954, 8000, 1125],
            [-46250, 5500, 9986],
            [-40497, -4000, 45142],
        ] {
            let center = Vec3::from(position.map(|v| v as f32));
            let floor = surface
                .cast(center - Vec3::Y * 500.0, center + Vec3::Y * 500.0)
                .unwrap_or_else(|| {
                    panic!("no rendered floor at {position:?}, prepared={prepared}")
                });
            assert!(floor.normal.y < -0.3);
            assert_eq!(floor.flags & render_mesh::WATER, 0);
            for style in [
                BloodStyle::Original,
                BloodStyle::OriginalHd,
                BloodStyle::Modern,
            ] {
                let settings = settings::Blood {
                    ground: style,
                    retain_original: true,
                    conform_retained: true,
                    ..Default::default()
                };
                let mut blood = Blood::default();
                game.effects = Default::default();
                blood.tick(&game, &models, &surface, &settings, None);
                game.effects.slots[0] = Some(Effect::Blood(tenchu_core::effects::Blood {
                    hint: None,
                    position: Position::from(position).wrapping_sub(Position::new(0, 1000, 0)),
                    velocity: [0; 3],
                    scale: 18000,
                    rotation: 57 * 4096,
                    time: 60,
                    brightness: 128,
                    sprite: 1,
                    mode: 0,
                    gore: false,
                }));
                for _ in 0..150 {
                    game.step(0).unwrap();
                    blood.tick(&game, &models, &surface, &settings, None);
                }
                assert!(
                    blood
                        .pages()
                        .flat_map(|p| p.iter())
                        .any(|v| Vec3::from(v.position).distance(center) < 500.0),
                    "no {style:?} stain at {position:?}, prepared={prepared}"
                );
                for vertex in blood.pages().flat_map(|page| page.iter()) {
                    let p = Vec3::from(vertex.position);
                    let hit = surface.cast(p - Vec3::Y, p + Vec3::Y).unwrap();
                    assert!(p.distance(hit.position) < 0.6, "floating stain at {p:?}");
                    assert_eq!(
                        render_mesh::original_hd_blood(vertex.flags),
                        style == BloodStyle::OriginalHd
                    );
                }
                if matches!(position, [-49197, 8000, 2251] | [-47954, 8000, 1125])
                    && !prepared
                    && let Some(output) = &output
                {
                    capture(
                        &game,
                        &models,
                        &blood,
                        graphics.clone(),
                        output,
                        style,
                        center,
                    );
                }
                let marks = blood.marks.len();
                blood.drops.push(Drop {
                    kind: DropKind::Wound,
                    previous: center - Vec3::Y * 1000.0,
                    position: center - Vec3::Y * 1000.0,
                    velocity: Vec3::ZERO,
                    life: 60,
                });
                let receivers = Receivers {
                    world: &surface,
                    hinges: Vec::new(),
                };
                for tick in 0..60 {
                    blood.advance_drops(tick, &game.mission.images, &receivers);
                }
                assert!(blood.drops.is_empty());
                assert!(
                    blood.marks.len() > marks,
                    "wound drip missing at {position:?}"
                );
            }
        }
    }
}

fn capture(
    game: &Game,
    models: &Models,
    blood: &Blood,
    mut graphics: settings::Graphics,
    output: &std::path::Path,
    style: BloodStyle,
    focus: Vec3,
) {
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    graphics.blood = blood.settings;
    review.configure(graphics);
    let stage =
        review
            .renderer
            .upload(&render_mesh::stage(&game.mission, &game.math, Some(models)));
    review
        .renderer
        .set_projected_decals(blood.projected_pages());
    let name = if focus.x == -49197.0 {
        format!("{style:?}")
    } else {
        format!("{style:?}-guard")
    };
    let eye = focus + Vec3::new(-1132.0, -2588.0, 1574.0);
    let delta = focus - eye;
    let camera = crate::debug::camera::Camera {
        position: eye,
        yaw: delta.x.atan2(delta.z),
        pitch: (-delta.y).atan2(delta.x.hypot(delta.z)),
        fov: 50.0,
    };
    let effects = crate::render_effects::build_options(
        game,
        false,
        Some(camera.source_view()),
        false,
        Some(models),
    );
    let effects = review.renderer.upload_effects(effects);
    review
        .capture(
            camera.matrix(1280.0 / 720.0),
            &[&stage],
            Some(&effects),
            false,
            &output.join(format!("{name}.png")),
        )
        .unwrap();
    let view = tenchu_core::math::Matrix {
        rotation: [[-3213, 0, -2563], [-989, 3772, 1238], [2360, 1579, -2959]],
        translation: Position::new(-37086, -18293, 29675),
    };
    let effects =
        crate::render_effects::build_options(game, false, Some(view), false, Some(models));
    let effects = review.renderer.upload_effects(effects);
    review
        .capture(
            crate::renderer::game_camera(view, 1280.0 / 720.0),
            &[&stage],
            Some(&effects),
            false,
            &output.join(format!("{name}-report-camera.png")),
        )
        .unwrap();
}
