use super::*;
use tenchu_assets::{
    executable::Executable,
    game::GameAssets,
    script::{Event, Language},
    stage::MissionAssets,
};

#[test]
#[ignore = "requires TENCHU_DISC; issue #105 localized SA execution bleeding, stains and saved continuation; optional GPU captures"]
fn execution_severed_wounds_bleed_without_changing_the_scene() {
    let disc = std::env::var_os("TENCHU_DISC").unwrap();
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let output = std::env::var_os("TENCHU_EXECUTION_BLOOD_REVIEW").map(std::path::PathBuf::from);
    if let Some(output) = &output {
        std::fs::create_dir_all(output).unwrap();
    }
    let mut results = Vec::new();
    for language in [Language::English, Language::Japanese] {
        let mut mission = MissionAssets::load_localized(&assets, 10, 0, 1, language).unwrap();
        mission.scripts.events = vec![Event {
            id: 0,
            sequence: 100,
            next: [255; 2],
            target: 255,
            mode: 0,
            value: 0,
            bounds: [[0; 2]; 3],
        }];
        let mut game =
            Game::new(mission, &Executable::new(&assets.executable).unwrap(), 0, 1).unwrap();
        game.combat.dismemberment.execution_scene = true;
        let settings = settings::Blood {
            hit: BloodStyle::Modern,
            ground: BloodStyle::OriginalHd,
            drips: BloodStyle::OriginalHd,
            death: BloodStyle::OriginalHd,
            retain_original: true,
            severed: 100,
            severed_seconds: 6,
            ..settings::Blood::modern()
        };
        let baseline_settings = settings::Blood {
            severed: 0,
            ..settings
        };
        let graphics = settings::Graphics {
            blood: settings,
            ..Default::default()
        };
        let models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
        let surfaces = Surfaces::stage(&game.mission, &game.math, Some(&models));
        let mut blood = Blood::default();
        let mut baseline = Blood::default();
        let mut restored: Option<Blood> = None;
        let mut detached_at = None;
        let mut drops = 0;
        let mut jets = 0;
        let mut finished = false;
        for _ in 0..2500 {
            game.step(0).unwrap();
            blood.tick(&game, &models, &surfaces, &settings, None);
            baseline.tick(&game, &models, &surfaces, &baseline_settings, None);
            if let Some(restored) = &mut restored {
                restored.tick(&game, &models, &surfaces, &settings, None);
                // Rendering and repeated presentation calls must not advance the effect.
                restored.tick(&game, &models, &surfaces, &settings, None);
                restored.animated_geometry(&game, &models, None, View::default(), Vec::new());
            }
            let actor = game.roster.get(game.roster.find(0x8f).unwrap());
            let Some(severed) = &actor.severed else {
                assert!(blood.severed.is_empty());
                continue;
            };
            // CHRANIM.C preserves HUMANOID_LIFE_INACTIVE for scripted death poses.
            assert_eq!(actor.health, -1);
            let elapsed = game.tick - *detached_at.get_or_insert(game.tick);
            let part = game.scene.models[actor.parts[severed.index]].local;
            for drop in blood
                .drops
                .iter()
                .filter(|d| d.kind == DropKind::Severed && d.life == 89)
            {
                drops += usize::from(drop.previous.distance(translation(part)) < 60.0);
                jets += usize::from(drop.velocity.length() > 35.0);
            }
            assert!(baseline.drops.iter().all(|d| d.kind != DropKind::Severed));
            if elapsed == 60 {
                restored =
                    Some(postcard::from_bytes(&postcard::to_allocvec(&blood).unwrap()).unwrap());
            }
            if [20, 120, 300].contains(&elapsed) {
                eprintln!(
                    "{language:?} +{elapsed}: drops={drops}, jets={jets}, ground={} (disabled={})",
                    blood.pages().map(|p| p.len()).sum::<usize>(),
                    baseline.pages().map(|p| p.len()).sum::<usize>()
                );
                let state = crate::session::tests::state(&game);
                if let Some(output) = &output {
                    capture(
                        &game,
                        &models,
                        &blood,
                        graphics.clone(),
                        &output.join(format!("{language:?}-{elapsed}.png")),
                    );
                }
                assert_eq!(state, crate::session::tests::state(&game));
                if let Some(restored) = &restored {
                    assert_eq!(
                        postcard::to_allocvec(restored).unwrap(),
                        postcard::to_allocvec(&blood).unwrap()
                    );
                }
            }
            if elapsed == 300 {
                assert!(
                    blood.drops.iter().all(|d| d.kind != DropKind::Severed),
                    "bleeding did not stop"
                );
                let extra_stains = blood.marks.len().saturating_sub(baseline.marks.len());
                results.push((language, drops, jets, extra_stains));
                blood.clear();
                for _ in 0..30 {
                    game.step(0).unwrap();
                    blood.tick(&game, &models, &surfaces, &settings, None);
                    assert!(
                        blood.drops.iter().all(|d| d.kind != DropKind::Severed),
                        "clear restarted bleeding"
                    );
                }
                finished = true;
                break;
            }
        }
        assert!(
            finished,
            "{language:?} execution did not reach the review window"
        );
    }
    for (language, drops, jets, stains) in results {
        assert!(
            drops > 0 && jets > 0 && stains > 0,
            "{language:?}: detached drops={drops}, stump jets={jets}, extra stains={stains}"
        );
    }
}

fn capture(
    game: &Game,
    models: &Models,
    blood: &Blood,
    graphics: settings::Graphics,
    output: &std::path::Path,
) {
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    review.configure(graphics);
    let actor = game.roster.get(game.roster.find(0x8f).unwrap());
    let center = Vec3::from(actor.position.map(|v| v as f32).to_array()) - Vec3::Y * 500.0;
    let eye = center + Vec3::new(2300.0, -1400.0, 3300.0);
    let delta = center - eye;
    let camera = crate::debug::camera::Camera {
        position: eye,
        yaw: delta.x.atan2(delta.z),
        pitch: (-delta.y).atan2(delta.x.hypot(delta.z)),
        fov: 50.0,
    };
    let stage =
        review
            .renderer
            .upload(&render_mesh::stage(&game.mission, &game.math, Some(models)));
    let actors = review.renderer.upload(&render_mesh::characters_presented(
        game,
        Some(models),
        true,
        None,
    ));
    let effects = blood.animated_geometry(
        game,
        models,
        None,
        View {
            camera: Some(camera.source_view()),
            alpha: 1.0,
            ..Default::default()
        },
        Vec::new(),
    );
    let effects = review.renderer.upload_effects(effects);
    review
        .renderer
        .set_projected_decals(blood.projected_pages());
    review
        .capture(
            camera.matrix(1280.0 / 720.0),
            &[&stage, &actors],
            Some(&effects),
            false,
            output,
        )
        .unwrap();
}
