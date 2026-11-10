use super::*;
use tenchu_assets::{game::GameAssets, stage::MissionAssets};

mod director;
mod doors;
mod heat;
mod host;
mod pickups;
mod pitfalls;
mod scrolling;
mod water;

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; imports original terrain, scenery and placements into all three native controllers"]
fn gaisen_stages_load_and_advance_native_worlds() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for stage in 0..11 {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        for hero in 0..3 {
            let mut session = Session::gaisen(
                &disc,
                &source,
                Options {
                    hero,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
            )
            .unwrap_or_else(|e| panic!("stage {stage} hero {hero}: {e}"));
            assert_eq!(session.assets.level.gaisen.as_ref().unwrap().id, stage);
            assert_eq!(session.assets.level.map, *source.map);
            assert_eq!(
                &session.assets.level.objects[..source.objects.len()],
                source.objects.as_slice()
            );
            for actor in &session.world.actors {
                let definition = session
                    .assets
                    .resources
                    .registry
                    .actor(actor.actor.kind)
                    .unwrap();
                assert!(session.assets.models.contains_key(&definition.model));
            }
            for _ in 0..120 {
                session.step(0).unwrap();
            }
            let player = &session.world.actors[session.world.player].actor;
            assert!(
                player.health > 0,
                "stage {stage} hero {hero} start {:?}",
                player.position
            );
            assert!(
                session.pending.is_empty(),
                "stage {stage} hero {hero}: {:?}",
                session.pending
            );
            eprintln!(
                "T1 stage {stage}, native hero {hero}: {} actors; player {:?}; HP {}",
                session.world.actors.len(),
                player.position,
                player.health
            );
            if let Some(path) = std::env::var_os("TENCHU_GAISEN_REVIEW")
                && hero == 0
                && matches!(stage, 0 | 1 | 2 | 4 | 7 | 8)
            {
                let path = std::path::PathBuf::from(path);
                std::fs::create_dir_all(&path).unwrap();
                render::capture(
                    &session,
                    &crate::settings::Settings::default(),
                    &path.join(format!("stage-{stage}.png")),
                )
                .unwrap();
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; imported stages finish through native results and retry their own geometry and loadout"]
fn gaisen_results_and_retry_keep_the_imported_stage() {
    use super::campaign::{Action, Content, Playback};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let source = gaisen::Source::open(
        std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
        0,
        0,
        2,
        tenchu_assets::script::Language::English,
    )
    .unwrap();
    let mut items = [0; 21];
    items[0] = 255;
    items[4] = 7;
    let mut playback = Playback::gaisen(disc, source, 2, 2, 17, Some(items), 6000).unwrap();
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("missing imported gameplay")
    };
    assert_eq!(
        session.world.terrain.gaisen.as_ref().unwrap().water_depth,
        6000
    );
    for _ in 0..15000 {
        if let Content::Gameplay(session) = &mut playback.content {
            let scene = session.gaisen.as_ref().unwrap();
            if !session.cinematic() && scene.director.source.tick > 0 {
                director::force_next(&scene.director, &mut session.world, &session.assets);
            }
        }
        playback.step(0).unwrap();
        if matches!(playback.content, Content::Results(_)) {
            break;
        }
    }
    let Content::Results(results) = &playback.content else {
        panic!("missing native results")
    };
    assert!(results.values.summary.population.bosses > 0);
    playback.action = Some(Action::Retry);
    for _ in 0..40 {
        playback.step(0).unwrap();
        if matches!(playback.content, Content::Gameplay(_)) {
            break;
        }
    }
    let Content::Gameplay(session) = &playback.content else {
        panic!("missing imported retry")
    };
    assert_eq!(session.world.map, 24);
    assert_eq!(
        session.world.terrain.gaisen.as_ref().unwrap().water_depth,
        6000
    );
    assert_eq!(session.assets.level.gaisen.as_ref().unwrap().id, 0);
    assert_eq!(session.world.actors[session.world.player].actor.kind, 2);
    assert_eq!(
        session.world.actors[session.world.player]
            .actor
            .inventory
            .amounts()[4],
        7
    );
    assert_eq!(session.world.statistics.killed, 0);
    playback.action = Some(Action::SelectMission);
    assert!(playback.step(0).unwrap().quit);
}
