use super::*;
use tenchu_assets::tenchu2::campaign::Objective;
use tenchu_core::tenchu2::input::JUMP;

fn start(sequence: usize) -> Playback<crate::source::DiscSource> {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut loadout = [0; 21];
    loadout[0] = 255;
    loadout[4] = 5;
    Playback::load(disc, sequence, 17, loadout).unwrap()
}

fn reach_exit(playback: &mut Playback<crate::source::DiscSource>) {
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("not gameplay")
    };
    let Objective::Reach { region, .. } = session.world.mission.objective else {
        panic!("not an exit")
    };
    let actor = &mut session.world.actors[0].actor;
    actor.position = region.center.map(|v| i32::from(v) * 1000).into();
    actor.previous = actor.position;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
}

fn next(playback: &mut Playback<crate::source::DiscSource>) {
    let generation = playback.generation;
    for _ in 0..6000 {
        playback.step(0).unwrap();
        if playback.generation != generation {
            return;
        }
    }
    panic!("sequence {} did not finish", playback.run.sequence);
}

#[test]
#[ignore = "requires TENCHU2_DISC; live exits, results, next mission, cutscene skipping and new-area reset"]
fn native_playback_advances_through_results_into_the_next_mission() {
    let mut playback = start(134);
    reach_exit(&mut playback);
    next(&mut playback);
    assert!(matches!(playback.content, Content::Results));
    assert_eq!(playback.run.sequence, 135);
    assert_eq!(playback.run.results.as_ref().unwrap().record.score, 400);
    let stock = playback.run.progress.stock.clone();
    for _ in 0..90 {
        playback.step(0).unwrap();
    }
    assert_eq!(
        serde_json::to_value(stock).unwrap(),
        serde_json::to_value(&playback.run.progress.stock).unwrap()
    );
    playback.action = Some(Action::Continue);
    playback.step(0).unwrap();
    assert_eq!(playback.run.sequence, 136);
    assert!(matches!(playback.content, Content::Story { .. }));
    playback.step(JUMP).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 137);
    let Content::Gameplay(session) = &playback.content else {
        panic!("not gameplay")
    };
    assert_eq!(session.world.actors[0].actor.kind, 1);
    assert_eq!(session.world.actors[0].actor.inventory.counts[4], 5);
    assert_eq!(session.world.statistics.killed, 0);
    assert_eq!(playback.run.summary.time_tenths, 0);
}

#[test]
#[ignore = "requires TENCHU2_DISC; live mission/story/boss/results and failure-story/retry transitions"]
fn native_playback_follows_boss_success_and_failure_branches() {
    let mut playback = start(9);
    if let Content::Gameplay(session) = &mut playback.content {
        session.world.actors[0].actor.health = 61;
        session.world.actors[0].actor.inventory.counts[4] = 3;
        session.world.statistics.stealth_kills = 2;
        session.world.statistics.killed = 2;
    }
    reach_exit(&mut playback);
    next(&mut playback);
    assert_eq!(playback.run.sequence, 10);
    assert!(matches!(playback.content, Content::Story { .. }));
    playback.step(0x2000).unwrap();
    next(&mut playback);
    assert_eq!(playback.run.sequence, 11);
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("not boss")
    };
    assert_eq!(session.world.actors[0].actor.health, 61);
    assert_eq!(session.world.actors[0].actor.inventory.counts[4], 3);
    assert_eq!(session.world.statistics.stealth_kills, 2);
    for index in session.world.mission.targets.into_iter().flatten() {
        session.world.actors[index].actor.health = 0;
    }
    next(&mut playback);
    assert_eq!(playback.run.sequence, 12);
    next(&mut playback);
    assert_eq!(playback.run.sequence, 13);
    assert!(matches!(playback.content, Content::Results));
    assert_eq!(
        playback
            .run
            .results
            .as_ref()
            .unwrap()
            .summary
            .kills
            .stealth_kills,
        2
    );
    playback.action = Some(Action::Retry);
    playback.step(0).unwrap();
    assert_eq!(playback.run.sequence, 9);
    assert_eq!(playback.run.summary.kills.killed, 0);

    let mut playback = start(11);
    let Content::Gameplay(session) = &mut playback.content else {
        unreachable!()
    };
    session.world.actors[0].actor.health = 0;
    session.world.actors[0].actor.motion.loops = 1;
    next(&mut playback);
    assert_eq!(playback.run.sequence, 14);
    next(&mut playback);
    assert!(matches!(playback.content, Content::Failed));
    playback.action = Some(Action::Retry);
    playback.step(0).unwrap();
    assert_eq!(playback.run.sequence, 9);
    let Content::Gameplay(session) = &playback.content else {
        unreachable!()
    };
    assert_eq!(
        session.world.actors[0].actor.health,
        session.world.actors[0].actor.max_health
    );
}
