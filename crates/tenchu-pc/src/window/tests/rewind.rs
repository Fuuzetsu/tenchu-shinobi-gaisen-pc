use super::*;
use std::collections::BTreeMap;
use tenchu_core::input;

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
        assert!(app.rewind_step().is_err());
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
        assert!(app.rewind.entries.len() <= 31);
        assert!(app.rewind.ticks(app.campaign.game.tick) <= 930);
        let bytes = app.rewind.bytes;
        let started = Instant::now();
        let sample = app.capture_rewind().unwrap();
        eprintln!(
            "hero {character}: history {} KiB, snapshot {:?}",
            bytes / 1024,
            started.elapsed()
        );
        drop(sample);
        for _ in 0..6 {
            app.rewind_step().unwrap();
            assert_eq!(
                &postcard::to_allocvec(&app.campaign.game).unwrap(),
                &expected[&app.campaign.game.tick]
            );
            assert!(app.paused && app.debug.pause_view);
            assert!(!app.trial.eligible);
            assert_eq!(app.controls.buttons(), 0);
            assert_eq!(
                app.rewind.entries.back().unwrap().tick,
                app.campaign.game.tick
            );
        }
        assert!(app.rewind.bytes < bytes);
        let branch = app.campaign.game.tick;
        app.paused = false;
        for _ in 0..31 {
            assert!(app.game_tick(input::DOWN));
        }
        let changed = postcard::to_allocvec(&app.campaign.game).unwrap();
        app.rewind_step().unwrap();
        assert_eq!(app.campaign.game.tick, branch + 30);
        assert_ne!(
            &postcard::to_allocvec(&app.campaign.game).unwrap(),
            &expected[&app.campaign.game.tick]
        );
        assert_ne!(postcard::to_allocvec(&app.campaign.game).unwrap(), changed);
        let file = app.capture_state().unwrap();
        app.restore_state(&file).unwrap();
        assert_eq!(app.settings.rewind_seconds, 30);
        assert!(app.rewind.entries.is_empty());
        assert!(app.rewind_step().is_err());
        app.record_rewind();
        assert!(!app.rewind.entries.is_empty());
        app.action(Action::Replay);
        assert!(app.rewind.entries.is_empty());
    }
}
