use super::*;
use std::time::Duration;

fn application(options: Options) -> Application {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.screen = Screen::Playing;
    let debug = crate::debug::Debug::new(&campaign.game).unwrap();
    Application {
        assets,
        campaign,
        storage: Storage::new().unwrap(),
        view: None,
        clock: GameClock::default(),
        last_frame: Instant::now(),
        controls: Controls::new(),
        paused: false,
        faulted: false,
        fault: None,
        focused: true,
        error: None,
        audio: None,
        recording_directory: None,
        recording: None,
        settings: Settings::default(),
        settings_store: settings::Store {
            path: std::env::temp_dir().join("tenchu-unused-test-settings.json"),
        },
        menu: Menu::default(),
        pointer: [0.0; 2],
        fps: 0.0,
        fps_elapsed: 0.0,
        fps_frames: 0,
        debug,
        screenshot: false,
        modifiers: ModifiersState::empty(),
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; unlimited host pacing executes every original tick and respects menu, focus and single-step"]
fn unlimited_simulation_keeps_the_same_state_sequence() {
    let options = Options {
        mission: 2,
        ..Default::default()
    };
    let mut app = application(options);
    let mut reference = application(options);
    app.debug.speed = 0;
    for _ in 0..8 {
        let tick = app.campaign.game.tick;
        app.advance_simulation(Duration::ZERO);
        assert!(app.campaign.game.tick > tick);
        assert!(app.campaign.game.tick - tick <= 2048);
        while reference.campaign.game.tick < app.campaign.game.tick {
            reference.campaign.tick(0).unwrap();
        }
        assert_eq!(
            crate::session::tests::state(&app.campaign.game),
            crate::session::tests::state(&reference.campaign.game)
        );
    }
    let tick = app.campaign.game.tick;
    app.menu.open = true;
    app.advance_simulation(Duration::from_secs(1));
    assert_eq!(app.campaign.game.tick, tick);
    app.menu.open = false;
    app.focused = false;
    app.advance_simulation(Duration::from_secs(1));
    assert_eq!(app.campaign.game.tick, tick);
    app.focused = true;
    app.apply_item(Item::Step, 1).unwrap();
    app.advance_simulation(Duration::ZERO);
    app.advance_simulation(Duration::ZERO);
    assert_eq!(app.campaign.game.tick, tick + 1);
    app.reset_item(Item::Speed).unwrap();
    assert_eq!(app.debug.speed, 100);
    app.set_paused(false);
    app.advance_simulation(Duration::ZERO);
    assert_eq!(app.campaign.game.tick, tick + 1);
}

#[test]
#[ignore = "requires TENCHU_DISC; resetting a selected setting leaves other preferences and live game state alone"]
fn per_value_reset_does_not_execute_actions_or_reset_other_options() {
    let mut app = application(Options::default());
    let state = crate::session::tests::state(&app.campaign.game);
    app.settings.graphics.fog_distance = 200;
    app.settings.graphics.exposure = 150;
    app.reset_item(Item::FogDistance).unwrap();
    assert_eq!(app.settings.graphics.fog_distance, 100);
    assert_eq!(app.settings.graphics.exposure, 150);
    app.debug.target[0] += 5000;
    assert!(app.reset_item(Item::Teleport).is_err());
    app.reset_item(Item::Target(0)).unwrap();
    assert_eq!(app.debug.target, app.campaign.game.player().position);
    app.debug.toggle_camera(&app.campaign.game);
    app.reset_item(Item::FreeCamera).unwrap();
    assert!(app.debug.camera.is_none());
    assert_eq!(state, crate::session::tests::state(&app.campaign.game));
}

#[test]
#[ignore = "requires TENCHU_DISC; executes the real host scheduler through cutscene acceleration and single-tick stepping"]
fn fast_forward_preserves_updates_and_stops_at_gameplay() {
    for character in 0..2 {
        let options = Options {
            mission: 2,
            character,
            ..Default::default()
        };
        let mut reference = application(options);
        reference.campaign.tick(0).unwrap();
        assert!(reference.campaign.game.cinematic.active);
        while reference.campaign.game.cinematic.active {
            reference.campaign.tick(0).unwrap();
            assert!(reference.campaign.game.tick < 5000);
        }
        let tick = reference.campaign.game.tick;
        let expected = crate::session::tests::state(&reference.campaign.game);
        for (speed, held) in [(2, false), (16, false), (0, false), (1, true)] {
            let mut app = application(options);
            app.settings.cutscene_speed = speed;
            if held {
                app.controls.key(KeyCode::Backspace, true);
            }
            app.advance_simulation(Duration::from_secs(1));
            assert!(app.campaign.game.cinematic.active);
            for _ in 0..5000 {
                app.advance_simulation(Duration::from_millis(16));
                if !app.campaign.game.cinematic.active {
                    break;
                }
            }
            assert!(!app.faulted, "{:?}", app.fault);
            assert_eq!(app.campaign.game.tick, tick, "speed {speed}, hold {held}");
            assert_eq!(crate::session::tests::state(&app.campaign.game), expected);
            app.advance_simulation(Duration::ZERO);
            assert_eq!(app.campaign.game.tick, tick, "no accelerated backlog");

            app.set_paused(true);
            app.advance_simulation(Duration::from_secs(10));
            assert_eq!(app.campaign.game.tick, tick);
            app.apply_item(Item::Step, 1).unwrap();
            app.advance_simulation(Duration::from_secs(10));
            assert_eq!(app.campaign.game.tick, tick + 1);
            app.advance_simulation(Duration::from_secs(10));
            assert_eq!(app.campaign.game.tick, tick + 1);
        }
    }
}
