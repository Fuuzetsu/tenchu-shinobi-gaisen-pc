use super::*;
use tenchu_core::tenchu2::input::{ATTACK, CROUCH, JUMP, RIGHT, UP};

#[test]
#[ignore = "requires TENCHU2_DISC; live story state and facial textures continue after a snapshot, with complete XA decoding"]
fn native_story_session_restores_faces_camera_and_captions() {
    use super::story::Story;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for (sequence, event) in [(0, 201), (10, 4), (197, 111)] {
        let mut story = Story::load(&disc, sequence, event, 17).unwrap();
        for _ in 0..340 {
            story.step(true).unwrap();
        }
        let mut restored = Story::load(&disc, sequence, event, 17).unwrap();
        restored
            .restore(serde_json::from_value(serde_json::to_value(&story.state).unwrap()).unwrap());
        for (&id, model) in &story.resources.models {
            assert_eq!(
                model.vram.words, restored.resources.models[&id].vram.words,
                "event {event}, model {id}: restored expression"
            );
        }
        for frame in 0..180 {
            let draw = frame % 7 != 3;
            story.step(draw).unwrap();
            restored.step(draw).unwrap();
            assert_eq!(
                serde_json::to_value(&story.state).unwrap(),
                serde_json::to_value(&restored.state).unwrap()
            );
        }
        let clip = tenchu_assets::audio::stream::Clip::load(
            &disc,
            story.resources.audio.as_ref().unwrap(),
        )
        .unwrap();
        assert!(
            clip.frames
                .iter()
                .any(|f| f[0].unsigned_abs() > 256 || f[1].unsigned_abs() > 256)
        );
        eprintln!(
            "JP event {event}: snapshot continuation and {:.2}s XA audio for {:.2}s scene",
            clip.frames.len() as f64 / f64::from(clip.rate),
            f64::from(story.resources.script.duration) / 30.0
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; runs the native frame and all three campaign heroes"]
fn native_frame_controls_and_audio_continue_after_restoring_a_world() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    for hero in 0..3 {
        let sequence = layouts
            .iter()
            .find(|l| match hero {
                0 => l.sequence < 100,
                1 => (100..185).contains(&l.sequence),
                _ => l.sequence >= 185,
            })
            .unwrap()
            .sequence;
        let mut session = Session::load(&disc, sequence, 1).unwrap();
        let start = session.world.actors[0].actor.position;
        let mut moved = false;
        let mut sounds = 0;
        for tick in 0..180 {
            let held = match tick {
                0..30 => UP,
                30..60 => CROUCH | UP,
                60..70 => RIGHT,
                70 => JUMP,
                130 => ATTACK,
                _ => 0,
            };
            sounds += session
                .step(held)
                .unwrap_or_else(|e| panic!("hero {hero}, tick {tick}: {e}"))
                .len();
            moved |= session.world.actors[0].actor.position != start;
        }
        assert!(moved, "hero {hero} did not move");
        let mut restored = Session::load(&disc, sequence, 1).unwrap();
        restored.world =
            serde_json::from_slice(&serde_json::to_vec(&session.world).unwrap()).unwrap();
        restored
            .assets
            .resources
            .clips
            .clone_from(&session.assets.resources.clips);
        for tick in 0..120 {
            let held = if tick == 0 { ATTACK } else { 0 };
            assert_eq!(
                session.step(held).unwrap(),
                restored.step(held).unwrap(),
                "hero {hero} sound continuation {tick}"
            );
        }
        assert_eq!(
            serde_json::to_value(&session.world).unwrap(),
            serde_json::to_value(&restored.world).unwrap()
        );
        eprintln!(
            "JP hero {hero} sequence {sequence}: 300 native ticks, {sounds} sounds, pending {:?}",
            session.pending
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; enters real deep water through the complete native frame"]
fn native_swimming_uses_the_imported_water_surface() {
    use tenchu_core::tenchu2::{
        actor::State,
        terrain::{self, Cursor, Query},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sequence = Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| l.map == 0)
        .unwrap()
        .sequence;
    let mut session = Session::load(&disc, sequence, 1).unwrap();
    let terrain = &session.world.terrain;
    let mut cursor = Cursor::default();
    let mut water = None;
    for node in terrain
        .cells
        .iter()
        .flat_map(|c| &c.nodes)
        .filter(|n| n.attributes & 0xf00 == 0x200)
    {
        let [x1, z1, x2, z2] = node.bounds.map(i32::from);
        for x in [x1 + (x2 - x1) / 4, (x1 + x2) / 2, x2 - (x2 - x1) / 4] {
            for z in [z1 + (z2 - z1) / 4, (z1 + z2) / 2, z2 - (z2 - z1) / 4] {
                let p = Position::new(x * 10, i32::from(node.y) * 10, z * 10);
                if terrain::level(
                    terrain,
                    &mut cursor,
                    p.wrapping_sub(Position::new(0, 1000, 0)),
                    Query::Uncached,
                )
                .unwrap()
                    == Some(p.y)
                    && terrain::level(
                        terrain,
                        &mut cursor,
                        p.wrapping_add(Position::new(0, 10, 0)),
                        Query::Uncached,
                    )
                    .unwrap()
                    .is_some_and(|y| y - p.y > 1000)
                {
                    water = Some(p);
                    break;
                }
            }
            if water.is_some() {
                break;
            }
        }
        if water.is_some() {
            break;
        }
    }
    let water = water.expect("No deep water in the source map");
    let actor = &mut session.world.actors[0].actor;
    actor.position = water;
    actor.previous = water;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
    let mut entered = false;
    for _ in 0..60 {
        session.step(UP).unwrap();
        entered |= session.world.actors[0].actor.state == State::Swimming;
    }
    assert!(entered, "player never entered native swimming");
    assert!(session.world.actors[0].actor.health > 0);
    assert_ne!(session.world.actors[0].actor.position, water);
    eprintln!(
        "Native water at {water:?}: {:?}, {:?}",
        session.world.actors[0].actor.position, session.world.actors[0].actor.state
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; reaches a real exit and restores during the native end transition"]
fn native_mission_exit_stops_input_and_survives_restoring_mid_fade() {
    use tenchu_assets::tenchu2::campaign::Objective;
    use tenchu_core::tenchu2::mission::{Outcome, Phase};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 1, 1).unwrap();
    let Objective::Reach { region, .. } = session.world.mission.objective else {
        panic!("not an exit mission")
    };
    let actor = &mut session.world.actors[0].actor;
    actor.position = region.center.map(|v| i32::from(v) * 1000).into();
    actor.previous = actor.position;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
    for _ in 0..60 {
        session.step(0).unwrap();
        if matches!(session.world.mission.phase, Phase::Ending { .. }) {
            break;
        }
    }
    assert!(
        matches!(
            session.world.mission.phase,
            Phase::Ending {
                outcome: Outcome::Arrived,
                remaining: 100,
                ..
            }
        ),
        "{:?}",
        session.world.mission.phase
    );
    for _ in 0..50 {
        session.step(UP | JUMP).unwrap();
    }
    assert_eq!(session.world.actors[0].actor.pad.held, 0);
    assert!(session.frame.transition_fade.is_some());
    let mut restored = Session::load(&disc, 1, 1).unwrap();
    restored.world = serde_json::from_slice(&serde_json::to_vec(&session.world).unwrap()).unwrap();
    restored
        .assets
        .resources
        .clips
        .clone_from(&session.assets.resources.clips);
    for _ in 0..60 {
        assert_eq!(session.step(UP).unwrap(), restored.step(UP).unwrap());
        assert_eq!(session.world.mission.phase, restored.world.mission.phase);
    }
    assert!(matches!(
        session.world.mission.phase,
        Phase::Finished {
            outcome: Outcome::Arrived,
            ..
        }
    ));
    assert_eq!(
        serde_json::to_value(&session.world).unwrap(),
        serde_json::to_value(&restored.world).unwrap()
    );
    let tick = session.world.tick;
    session.step(ATTACK).unwrap();
    assert_eq!(session.world.tick, tick);
}

#[test]
#[ignore = "requires TENCHU2_DISC; completes a JP exit that directly enters results"]
fn native_results_follow_the_mission_and_preserve_campaign_rewards() {
    use tenchu_assets::tenchu2::campaign::Objective;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 134, 1).unwrap();
    let Objective::Reach { region, .. } = session.world.mission.objective else {
        panic!("not an exit mission")
    };
    let actor = &mut session.world.actors[0].actor;
    actor.position = region.center.map(|v| i32::from(v) * 1000).into();
    actor.previous = actor.position;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
    for _ in 0..160 {
        session.step(0).unwrap();
        if session.world.results.is_some() {
            break;
        }
    }
    let result = session
        .world
        .results
        .as_ref()
        .expect("exit did not enter results");
    assert_eq!(result.record.score, 400);
    assert_eq!(result.record.rank, 4);
    assert!(result.new_best);
    let mission = session.assets.campaign.result_mission(1, 135).unwrap();
    assert_eq!(session.world.progress.records[1][mission], result.record);
    assert_eq!(session.world.progress.stock[1].items[4], Some(11));
    let before = serde_json::to_value(&session.world).unwrap();
    session.world = serde_json::from_value(before.clone()).unwrap();
    session.step(ATTACK).unwrap();
    assert_eq!(serde_json::to_value(&session.world).unwrap(), before);
}
