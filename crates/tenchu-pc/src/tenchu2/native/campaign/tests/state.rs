use super::*;
use crate::source::DiscSource;
use tenchu_core::tenchu2::mission::Outcome;

#[test]
#[ignore = "requires TENCHU2_DISC; failed preparations preserve the current scene and source state with or without resource reuse"]
fn native_checkpoint_preparation_failure_is_atomic() {
    for sequence in [9, 23] {
        let mut playback = start(sequence);
        for _ in 0..30 {
            playback.step(0).unwrap();
        }
        let earlier = postcard::to_allocvec(&playback.snapshot().unwrap()).unwrap();
        for _ in 0..30 {
            playback.step(0).unwrap();
        }
        let current = postcard::to_allocvec(&playback.snapshot().unwrap()).unwrap();
        let generation = playback.generation;
        let alternate = start(if sequence == 9 { 119 } else { 9 });
        let other_scene = postcard::to_allocvec(&alternate.snapshot().unwrap()).unwrap();
        for (bytes, expected_reuse) in [(&earlier, true), (&other_scene, false)] {
            let saved = postcard::from_bytes(bytes).unwrap();
            let result: Result<()> = playback.restore_snapshot_with(saved, |_, reused| {
                assert_eq!(reused, expected_reuse);
                Err("deliberate preparation failure".into())
            });
            assert_eq!(result.unwrap_err(), "deliberate preparation failure");
            assert_eq!(playback.generation, generation);
            assert!(postcard::to_allocvec(&playback.snapshot().unwrap()).unwrap() == current);
        }
        let saved = postcard::from_bytes(&earlier).unwrap();
        playback
            .restore_snapshot_with(saved, |_, reused| {
                assert!(reused);
                Ok(())
            })
            .unwrap();
        assert!(postcard::to_allocvec(&playback.snapshot().unwrap()).unwrap() == earlier);
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; complete native histories retain thirty seconds, changed resources and exact branch continuation"]
fn native_rewind_capture_retains_history_and_branches() {
    for sequence in [9, 119, 191, 23] {
        let mut playback = start(sequence);
        if let Content::Gameplay(session) = &mut playback.content {
            session.world.actors[session.world.player]
                .actor
                .invulnerable = true;
        }
        let mut history = crate::rewind::History::default();
        let mut elapsed = std::time::Duration::ZERO;
        let mut expected = std::collections::BTreeMap::new();
        let ticks = if let Content::Story { story, .. } = &playback.content {
            u64::try_from(story.resources.script.duration)
                .unwrap()
                .saturating_sub(1)
                .min(901)
        } else {
            901
        };
        for tick in 0..ticks {
            if tick == 450
                && let Content::Gameplay(session) = &playback.content
            {
                let position = session.world.actors[session.world.player].actor.position;
                playback
                    .spawn_actor(
                        super::super::super::spawning::Request {
                            source: super::super::super::spawning::Source::Native(0xe0),
                            position: position
                                .wrapping_add(tenchu_core::math::Position::new(1200, 0, 0)),
                            yaw: 1024,
                            snap: true,
                        },
                        None,
                    )
                    .unwrap();
            }
            playback.step(0).unwrap();
            let started = std::time::Instant::now();
            history.recorder.begin();
            playback
                .snapshot()
                .unwrap()
                .capture(&mut history.recorder)
                .unwrap();
            let state = history.recorder.finish().unwrap();
            history.push(tick, 30, state, None).unwrap();
            elapsed += started.elapsed();
            if [0, 449, 450, ticks - 1].contains(&tick) {
                expected.insert(
                    tick as usize,
                    postcard::to_allocvec(&playback.snapshot().unwrap()).unwrap(),
                );
            }
        }
        eprintln!(
            "native rewind sequence={sequence}: capture+delta {:.3} ms/tick, raw={} bytes, history={} bytes, retained={}",
            elapsed.as_secs_f64() * 1000.0 / ticks as f64,
            expected[&0].len(),
            history.bytes,
            history.entries.len()
        );
        assert_eq!(history.entries.front().unwrap().tick, 0);
        assert_eq!(history.entries.len(), ticks as usize);
        for (&index, bytes) in expected.iter().rev() {
            assert!(
                history.snapshot(index).unwrap().bytes().unwrap() == *bytes,
                "sequence {sequence}, tick {index}"
            );
        }
        let mut restored = start(1);
        restored
            .restore_snapshot(history.snapshot(0).unwrap().decode().unwrap(), |_| Ok(()))
            .unwrap();
        history.branch(0).unwrap();
        let mut original = start(sequence);
        if let Content::Gameplay(session) = &mut original.content {
            session.world.actors[session.world.player]
                .actor
                .invulnerable = true;
        }
        original.step(0).unwrap();
        for tick in 1..=60 {
            together(&mut original, &mut restored, 1);
            history.recorder.begin();
            restored
                .snapshot()
                .unwrap()
                .capture(&mut history.recorder)
                .unwrap();
            let state = history.recorder.finish().unwrap();
            history.push(tick, 30, state, None).unwrap();
        }
        assert!(
            history.snapshot(60).unwrap().bytes().unwrap()
                == postcard::to_allocvec(&original.snapshot().unwrap()).unwrap()
        );
    }
}

fn compare(a: &Playback<DiscSource>, b: &Playback<DiscSource>) {
    assert!(postcard::to_allocvec(&a.run).unwrap() == postcard::to_allocvec(&b.run).unwrap());
    assert_eq!(
        std::mem::discriminant(&a.content),
        std::mem::discriminant(&b.content)
    );
    assert_eq!(a.previous_buttons, b.previous_buttons);
    assert_eq!(a.loadout, b.loadout);
    assert_eq!(a.origin, b.origin);
    assert!(postcard::to_allocvec(&a.death).unwrap() == postcard::to_allocvec(&b.death).unwrap());
    match (&a.content, &b.content) {
        (Content::Gameplay(a), Content::Gameplay(b)) => {
            assert!(
                postcard::to_allocvec(&a.world).unwrap()
                    == postcard::to_allocvec(&b.world).unwrap()
            );
            assert!(
                postcard::to_allocvec(&a.frame).unwrap()
                    == postcard::to_allocvec(&b.frame).unwrap()
            );
        }
        (
            Content::Story {
                story: a,
                audio: aa,
            },
            Content::Story {
                story: b,
                audio: ba,
            },
        ) => {
            assert!(
                postcard::to_allocvec(&a.state).unwrap()
                    == postcard::to_allocvec(&b.state).unwrap()
            );
            assert_eq!(
                aa.as_ref().map(|a| (a.rate, &a.frames)),
                ba.as_ref().map(|a| (a.rate, &a.frames))
            );
        }
        _ => (),
    }
}

#[test]
fn campaign_origin_preserves_legacy_quickstate_encoding() {
    for (old_flag, origin) in [(false, Origin::Quick), (true, Origin::UnlockedMenu)] {
        let legacy = postcard::to_allocvec(&old_flag).unwrap();
        assert_eq!(postcard::from_bytes::<Origin>(&legacy).unwrap(), origin);
        assert_eq!(postcard::to_allocvec(&origin).unwrap(), legacy);
    }
    assert_eq!(
        postcard::from_bytes::<Origin>(&postcard::to_allocvec(&Origin::Campaign).unwrap()).unwrap(),
        Origin::Campaign
    );
    assert!(postcard::from_bytes::<Origin>(&[3]).is_err());
}

fn together(a: &mut Playback<DiscSource>, b: &mut Playback<DiscSource>, ticks: usize) {
    for _ in 0..ticks {
        let x = a.step(0).unwrap();
        let y = b.step(0).unwrap();
        compare(a, b);
        assert_eq!((x.quit, x.stop_music), (y.quit, y.stop_music));
        assert_eq!(
            x.sounds.iter().map(|s| s.sound).collect::<Vec<_>>(),
            y.sounds.iter().map(|s| s.sound).collect::<Vec<_>>()
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; campaign checkpoints preserve native gameplay/story/area transitions and retry state"]
fn native_campaign_checkpoints_preserve_transitions_and_retry() {
    for sequence in [9, 119, 191] {
        let mut original = start(sequence);
        original.origin = Origin::UnlockedMenu;
        let mut settings = crate::settings::Settings {
            keep_enemies_active: true,
            ..Default::default()
        };
        settings.bleeding.enabled = true;
        settings.bleeding.bandages = 3;
        original.configure(&settings).unwrap();
        for _ in 0..30 {
            original.step(0).unwrap();
        }
        let bytes = original.capture_state().unwrap();
        let mut restored = start(1);
        let generation = restored.generation;
        restored.restore_state(&bytes).unwrap();
        assert_eq!(restored.generation, generation + 1);
        assert!(bytes == restored.capture_state().unwrap());
        together(&mut original, &mut restored, 90);
        for playback in [&mut original, &mut restored] {
            let Content::Gameplay(session) = &mut playback.content else {
                unreachable!()
            };
            session.world.mission.phase = Phase::Finished {
                outcome: Outcome::Arrived,
                finished_at: session.world.tick,
            };
        }
        together(&mut original, &mut restored, 1);
        assert!(matches!(original.content, Content::Story { .. }));
        together(&mut original, &mut restored, 40);
        let bytes = original.capture_state().unwrap();
        restored.load_current().unwrap();
        restored.restore_state(&bytes).unwrap();
        assert!(bytes == restored.capture_state().unwrap());
        together(&mut original, &mut restored, 60);
        let previous = original.run.sequence;
        for _ in 0..6000 {
            together(&mut original, &mut restored, 1);
            if original.run.sequence != previous {
                break;
            }
        }
        assert_ne!(original.run.sequence, previous);
        compare(&original, &restored);
        for playback in [&mut original, &mut restored] {
            let destination = playback.run.retry(&playback.campaign).unwrap();
            playback.arrive(destination).unwrap();
        }
        compare(&original, &restored);
        eprintln!(
            "Campaign {sequence}: gameplay/story checkpoint, area transition and retry match"
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; snapshots preserve frozen/live death holds and pending Continue"]
fn native_campaign_checkpoints_preserve_death_holds() {
    for live in [false, true] {
        let mut original = start(9);
        original
            .configure(&crate::settings::Settings {
                wait_after_death: true,
                live_death_scene: live,
                ..Default::default()
            })
            .unwrap();
        death::start_death(&mut original);
        for _ in 0..300 {
            original.step(0).unwrap();
            if original.death.waiting() {
                break;
            }
        }
        assert!(original.death.waiting());
        let bytes = original.capture_state().unwrap();
        let mut restored = start(119);
        restored.restore_state(&bytes).unwrap();
        together(&mut original, &mut restored, 90);
        original.confirm();
        let bytes = original.capture_state().unwrap();
        restored.restore_state(&bytes).unwrap();
        together(&mut original, &mut restored, 1);
        assert!(!original.death.waiting());
        compare(&original, &restored);
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; malformed campaign snapshots do not replace live content, progress or generation"]
fn native_campaign_checkpoints_reject_mismatched_runs() {
    use super::super::state::{Owned, Scene};
    let mut playback = start(9);
    playback.step(0).unwrap();
    let original = playback.capture_state().unwrap();
    let generation = playback.generation;
    for change in [
        (|s: &mut Owned| s.signature.clear()) as fn(&mut Owned),
        |s| s.run.hero = 3,
        |s| s.run.difficulty = 0,
        |s| s.run.sequence = 119,
        |s| s.run.mission = 100,
        |s| {
            if let Scene::Gameplay(g) = &mut s.content {
                g.core.0.player = usize::MAX;
            }
        },
        |s| {
            if let Scene::Gameplay(g) = &mut s.content {
                g.core.1.campaign.sequences[9].resource = u16::MAX;
            }
        },
    ] {
        let mut saved: Owned = crate::savestate::decode(&original).unwrap();
        change(&mut saved);
        assert!(
            playback
                .restore_state(&crate::savestate::encode(&saved).unwrap())
                .is_err()
        );
        assert_eq!(playback.generation, generation);
        assert!(playback.capture_state().unwrap() == original);
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; loading a checkpoint from mission selection restores in-memory progress without writing the progress file"]
fn native_campaign_checkpoints_leave_progress_files_unchanged() {
    let root = std::env::temp_dir().join(format!(
        "tenchu-native-checkpoint-progress-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = root.join("progress.json");
    let file = super::super::super::save::SaveFile::at(path.clone());
    file.write(&Default::default()).unwrap();
    let stored = std::fs::read(&path).unwrap();
    let mut original = start(9);
    original.origin = Origin::Campaign;
    original.run.progress.records[0][1].score = 527;
    original.run.progress.records[0][1].time_tenths = 59999;
    original.step(0x2000).unwrap();
    let bytes = original.capture_state().unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut restored = Playback::menu(disc, 2, 1, Some(file)).unwrap();
    assert!(restored.capture_state().is_err());
    restored.restore_state(&bytes).unwrap();
    assert!(restored.save.is_some());
    assert_eq!(std::fs::read(&path).unwrap(), stored);
    assert!(bytes == restored.capture_state().unwrap());
    together(&mut original, &mut restored, 30);
    restored.action = Some(Action::SelectMission);
    restored.step(0).unwrap();
    let Content::MissionSelection(selection) = &restored.content else {
        panic!("checkpoint lost its campaign menu")
    };
    assert!(!selection.state.all_unlocked);
    assert!(selection.state.available(1, &selection.resources.rules));
    assert!(!selection.state.available(2, &selection.resources.rules));
    assert_eq!(std::fs::read(&path).unwrap(), stored);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires both discs; native T1 stage checkpoints restore scripts and retry context after results"]
fn native_campaign_checkpoints_restore_imported_stages_after_results() {
    use crate::tenchu2::native::gaisen;
    let load = |character| {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let source = gaisen::Source::open(
            std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
            0,
            character,
            2,
            tenchu_assets::script::Language::English,
        )
        .unwrap();
        Playback::gaisen(disc, source, 2, 2, 17, Some([0; 21]), 6000).unwrap()
    };
    let mut original = load(0);
    for _ in 0..60 {
        original.step(0).unwrap();
    }
    let bytes = original.capture_state().unwrap();
    let mut restored = load(0);
    let Content::Gameplay(session) = &mut restored.content else {
        unreachable!()
    };
    session.world.mission.phase = Phase::Finished {
        outcome: Outcome::Arrived,
        finished_at: 180,
    };
    restored.step(0).unwrap();
    assert!(matches!(restored.content, Content::Results(_)));
    restored.restore_state(&bytes).unwrap();
    assert!(bytes == restored.capture_state().unwrap());
    together(&mut original, &mut restored, 90);
    assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
    for playback in [&mut original, &mut restored] {
        let destination = playback.run.retry(&playback.campaign).unwrap();
        playback.arrive(destination).unwrap();
    }
    compare(&original, &restored);
    assert_eq!(original.gaisen_water_depth, 6000);
    let mut wrong = load(1);
    let before = wrong.capture_state().unwrap();
    let generation = wrong.generation;
    assert!(wrong.restore_state(&bytes).is_err());
    assert!(before == wrong.capture_state().unwrap());
    assert_eq!(wrong.generation, generation);
}
