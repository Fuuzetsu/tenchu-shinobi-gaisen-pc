use super::*;
use std::collections::BTreeMap;
use tenchu_core::input;

#[test]
#[ignore = "requires TENCHU_DISC; slot selection is UI state, including across model rebuilds and file restores"]
fn rewind_keeps_the_selected_savestate_slot() {
    let mut app = application(Options::default());
    app.campaign.game.events = Default::default();
    app.settings.rewind_seconds = 5;
    for _ in 0..12 {
        assert!(app.game_tick(0));
    }
    app.debug.state_slot = 4;
    app.seek_rewind(4).unwrap();
    assert_eq!(app.debug.state_slot, 4);

    app.settings.player_model = "disc:144".into();
    app.seek_rewind(8).unwrap();
    assert_eq!(app.debug.state_slot, 4, "model rebuild lost slot selection");
    app.set_paused(false);
    assert!(app.game_tick(0));
    assert_eq!(app.debug.state_slot, 4);

    let bytes = app.capture_state().unwrap();
    app.debug.state_slot = 2;
    app.restore_state(&bytes).unwrap();
    assert_eq!(app.debug.state_slot, 2, "file restore lost slot selection");
}

#[test]
#[ignore = "requires TENCHU_DISC; repeatable CPU benchmark for rewind recording and restore"]
fn profile_rewind_recording() {
    fn report(name: &str, times: &mut [f64]) {
        times.sort_by(f64::total_cmp);
        eprintln!(
            "{name}: mean {:.3} ms, p95 {:.3} ms, max {:.3} ms",
            times.iter().sum::<f64>() / times.len() as f64,
            times[times.len() * 95 / 100],
            times.last().unwrap()
        );
    }
    for mission in [1, 2, 6, 9] {
        let mut app = application(Options {
            mission,
            ..Default::default()
        });
        app.campaign.game.events = Default::default();
        app.campaign
            .game
            .roster
            .get_mut(app.campaign.game.player_id)
            .invulnerable = true;
        let mut simulation = Vec::new();
        let mut capture = Vec::new();
        let mut compress = Vec::new();
        for tick in 0..300 {
            let started = Instant::now();
            assert!(app.game_tick(if tick % 100 < 50 {
                input::UP
            } else {
                input::RIGHT
            }));
            simulation.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            let state = app.capture_rewind().unwrap();
            capture.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            app.rewind.push(app.campaign.game.tick, 30, state).unwrap();
            compress.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        eprintln!(
            "M{mission}: {} KiB compressed history",
            app.rewind.bytes / 1024
        );
        report("simulation", &mut simulation);
        report("capture", &mut capture);
        report("compress", &mut compress);
        let mut restore = Vec::new();
        for _ in 0..120 {
            let started = Instant::now();
            app.rewind_to(app.rewind.previous(app.campaign.game.tick).unwrap())
                .unwrap();
            restore.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        report("restore", &mut restore);
        let mut seek = Vec::new();
        let last = app.rewind.entries.len() - 1;
        for index in [0, last, last / 2, 0, last, last / 4] {
            let started = Instant::now();
            app.rewind_to(index).unwrap();
            seek.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        report("timeline seek", &mut seek);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; capture cannot change gameplay, rewinding restores live items and resuming discards the old future"]
fn rewind_restores_bounded_history_and_branches_without_changing_gameplay() {
    for character in 0..2 {
        let options = Options {
            character,
            ..Default::default()
        };
        let mut app = application(options);
        let mut reference = application(options);
        for a in [&mut app, &mut reference] {
            a.campaign.game.events = Default::default();
            a.campaign
                .game
                .roster
                .get_mut(a.campaign.game.player_id)
                .invulnerable = true;
            a.campaign.game.set_inventory(4, 10).unwrap();
            a.campaign.game.hud.cursor = 4;
        }
        assert!(app.toggle_rewind().is_err());
        app.settings.rewind_seconds = 30;
        let mut expected = BTreeMap::new();
        let mut active_items = 0;
        for tick in 0..1080 {
            let buttons = if tick % 240 == 100 {
                input::TRIANGLE
            } else {
                match tick % 80 {
                    0..12 => input::UP,
                    12..18 => input::LEFT,
                    _ => 0,
                }
            };
            assert!(app.game_tick(buttons));
            assert!(reference.game_tick(buttons));
            if app.rewind.entries.back().unwrap().tick == app.campaign.game.tick {
                let current = postcard::to_allocvec(&app.campaign.game).unwrap();
                assert_eq!(
                    current,
                    postcard::to_allocvec(&reference.campaign.game).unwrap()
                );
                active_items += usize::from(
                    app.campaign
                        .game
                        .items
                        .slots
                        .iter()
                        .any(|i| i.processor().is_some()),
                );
                expected.insert(app.campaign.game.tick, current);
                while expected.len() > 31 {
                    expected.pop_first();
                }
            }
        }
        assert!(active_items > 3);
        assert!(app.rewind.entries.len() <= 901);
        assert!(app.rewind.ticks(app.campaign.game.tick) <= 930);
        let bytes = app.rewind.bytes;
        let latest = app.campaign.game.tick;
        let count = app.rewind.entries.len();
        let started = Instant::now();
        let sample = app.capture_rewind().unwrap();
        eprintln!(
            "hero {character}: history {} KiB, snapshot {:?}",
            bytes / 1024,
            started.elapsed()
        );
        drop(sample);
        for _ in 0..6 {
            app.rewind_to(app.rewind.previous(app.campaign.game.tick).unwrap())
                .unwrap();
            assert_eq!(
                &postcard::to_allocvec(&app.campaign.game).unwrap(),
                &expected[&app.campaign.game.tick]
            );
            assert!(app.paused && app.debug.pause_view);
            assert!(!app.trial.eligible);
            assert_eq!(app.controls.buttons(), 0);
            assert_eq!(app.rewind.entries.back().unwrap().tick, latest);
        }
        let branch = app.campaign.game.tick;
        for tick in [latest - 2, latest - 8, latest, branch] {
            app.seek_rewind(tick).unwrap();
            assert_eq!(app.campaign.game.tick, tick);
            assert_eq!(
                &postcard::to_allocvec(&app.campaign.game).unwrap(),
                &expected[&tick]
            );
            assert_eq!(app.rewind.entries.len(), count);
            assert_eq!(app.rewind.bytes, bytes);
            assert!(app.paused && app.reverse.preview);
        }
        app.toggle_menu();
        app.toggle_menu();
        assert_eq!(app.rewind.entries.len(), count);
        app.set_paused(false);
        assert!(!app.reverse.preview);
        assert_eq!(app.rewind.entries.back().unwrap().tick, branch);
        assert!(app.rewind.bytes < bytes);
        for _ in 0..4 {
            assert!(app.game_tick(input::DOWN));
        }
        let changed = postcard::to_allocvec(&app.campaign.game).unwrap();
        app.rewind_to(app.rewind.previous(app.campaign.game.tick).unwrap())
            .unwrap();
        assert_eq!(app.campaign.game.tick, branch + 3);
        assert_ne!(
            &postcard::to_allocvec(&app.campaign.game).unwrap(),
            &expected[&app.campaign.game.tick]
        );
        assert_ne!(postcard::to_allocvec(&app.campaign.game).unwrap(), changed);
        let file = app.capture_state().unwrap();
        app.restore_state(&file).unwrap();
        assert_eq!(app.settings.rewind_seconds, 30);
        assert!(app.rewind.entries.is_empty());
        assert!(app.toggle_rewind().is_err());
        app.record_rewind();
        assert!(!app.rewind.entries.is_empty());
        app.action(Action::Replay);
        assert!(app.rewind.entries.is_empty());
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; timed reverse playback must preserve exact state and geometry caches, honor pause, and branch on resume"]
fn continuous_rewind_obeys_speed_and_preserves_scene_allocations() {
    use std::time::Duration;
    let mut app = application(Options::default());
    app.campaign.game.events = Default::default();
    app.settings.rewind_seconds = 10;
    let mut expected = BTreeMap::new();
    for _ in 0..120 {
        app.game_tick(input::UP);
        if app.campaign.game.tick >= 90 {
            expected.insert(
                app.campaign.game.tick,
                postcard::to_allocvec(&app.campaign.game).unwrap(),
            );
        }
    }
    let pointers = |game: &Game| {
        crate::visuals::catalog::models(&game.mission)
            .into_iter()
            .map(|m| (m.id, m.geometry.as_ptr() as usize))
            .collect::<Vec<_>>()
    };
    let scene = pointers(&app.campaign.game);
    let before = app.campaign.game.tick;
    app.settings.rewind_speed = 200;
    app.toggle_rewind().unwrap();
    for _ in 0..10 {
        app.advance_simulation(Duration::from_nanos(16_666_667));
        assert_eq!(pointers(&app.campaign.game), scene);
        assert_eq!(
            &postcard::to_allocvec(&app.campaign.game).unwrap(),
            &expected[&app.campaign.game.tick]
        );
    }
    assert_eq!(app.campaign.game.tick, before - 10);
    app.focused = false;
    app.advance_simulation(Duration::from_millis(100));
    assert_eq!(app.campaign.game.tick, before - 10);
    app.focused = true;
    app.settings.rewind_speed = 50;
    for _ in 0..4 {
        app.advance_simulation(Duration::from_nanos(16_666_667));
    }
    assert_eq!(app.campaign.game.tick, before - 11);
    app.toggle_rewind().unwrap();
    app.advance_simulation(Duration::from_secs(1));
    assert_eq!(app.campaign.game.tick, before - 11);
    app.set_paused(false);
    app.game_tick(input::DOWN);
    assert_ne!(
        &postcard::to_allocvec(&app.campaign.game).unwrap(),
        &expected[&app.campaign.game.tick]
    );
    app.toggle_rewind().unwrap();
    while app.reverse.active {
        app.advance_simulation(Duration::from_millis(100));
    }
    assert!(app.rewind.entries.len() > 100);
    assert!(app.paused);
    let oldest = app.campaign.game.tick;
    let latest = app.rewind.entries.back().unwrap().tick;
    app.seek_rewind(u64::MAX).unwrap();
    assert_eq!(app.campaign.game.tick, latest);
    app.seek_rewind(0).unwrap();
    assert_eq!(app.campaign.game.tick, oldest);
    let count = app.rewind.entries.len();
    app.seek_rewind(latest).unwrap();
    app.set_paused(false);
    assert_eq!(app.rewind.entries.len(), count);
    app.game_tick(0);
    assert_eq!(app.rewind.entries.back().unwrap().tick, latest + 1);
}
