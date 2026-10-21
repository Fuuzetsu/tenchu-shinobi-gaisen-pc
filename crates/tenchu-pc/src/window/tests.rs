use super::*;
use std::time::Duration;

fn application(options: Options) -> Application {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.screen = Screen::Playing;
    let debug = crate::debug::Debug::new(&campaign.game).unwrap();
    Application {
        frontend: None,
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
        performance: Default::default(),
        fps_elapsed: 0.0,
        fps_frames: 0,
        debug,
        screenshot: false,
        demo: None,
        demo_exit: false,
        video: None,
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
    assert_eq!(
        app.debug.target,
        app.campaign.game.player().position.to_array()
    );
    app.debug.toggle_camera(&app.campaign.game);
    app.reset_item(Item::FreeCamera).unwrap();
    assert!(app.debug.camera.is_none());
    app.settings.graphics.asset_pack = false;
    app.settings.pause_on_defocus = false;
    app.debug.speed = 200;
    app.reset_item(Item::Preset).unwrap();
    app.apply_item(Item::Preset, 1).unwrap();
    assert_eq!(app.settings.graphics.preset(), 1);
    app.apply_item(Item::Preset, 1).unwrap();
    assert_eq!(app.settings.graphics.preset(), 2);
    app.apply_item(Item::Bloom, 1).unwrap();
    assert_eq!(app.settings.graphics.preset(), settings::presets::CUSTOM);
    app.apply_item(Item::PhotoMode, 1).unwrap();
    assert!(app.debug.photo_mode && app.debug.active());
    let mut effects = crate::render_effects::build(&app.campaign.game);
    let world: Vec<_> = effects
        .vertices
        .iter()
        .filter(|v| v.flags & crate::render_mesh::UI == 0)
        .copied()
        .collect();
    effects.hide_ui();
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&effects.vertices),
        bytemuck::cast_slice::<_, u8>(&world)
    );
    assert_eq!(
        effects.batches.iter().map(|b| b.range.len()).sum::<usize>(),
        world.len()
    );
    app.reset_item(Item::Preset).unwrap();
    assert_eq!(app.settings.graphics.preset(), 0);
    assert!(!app.settings.graphics.asset_pack && !app.settings.pause_on_defocus);
    assert_eq!(app.debug.speed, 200);
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

#[test]
#[ignore = "requires TENCHU_DISC; a live settings overlay must not feed menu input into gameplay or bypass explicit pauses"]
fn live_settings_keep_simulating_without_menu_input() {
    let options = Options {
        mission: 2,
        ..Default::default()
    };
    let mut app = application(options);
    let mut reference = application(options);
    app.menu.open = true;
    app.apply_item(Item::MenuPause, 1).unwrap();
    app.controls.key(KeyCode::KeyW, true);
    let tick = app.campaign.game.tick;
    app.advance_simulation(Duration::from_millis(34));
    assert_eq!(app.campaign.game.tick, tick + 1);
    reference.campaign.tick(0).unwrap();
    assert_eq!(
        crate::session::tests::state(&app.campaign.game),
        crate::session::tests::state(&reference.campaign.game)
    );
    app.set_paused(true);
    app.advance_simulation(Duration::from_secs(1));
    assert_eq!(app.campaign.game.tick, tick + 1);
    app.set_paused(false);
    app.focused = false;
    app.advance_simulation(Duration::from_secs(1));
    assert_eq!(app.campaign.game.tick, tick + 1);
    app.focused = true;
    app.reset_item(Item::MenuPause).unwrap();
    app.advance_simulation(Duration::from_secs(1));
    assert_eq!(app.campaign.game.tick, tick + 1);
}

#[test]
#[ignore = "requires TENCHU_DISC; all mission/layout rosters run with activation limits disabled"]
fn all_enemy_activation_survives_layouts_and_mission_reloads() {
    let mut app = application(Options::default());
    app.apply_item(Item::KeepEnemiesActive, 1).unwrap();
    assert!(app.campaign.game.activation.keep_all_active);
    app.reset_item(Item::KeepEnemiesActive).unwrap();
    assert!(!app.campaign.game.activation.keep_all_active);
    let saved: Settings = serde_json::from_str(r#"{"keep_enemies_active":true}"#).unwrap();
    assert!(saved.keep_enemies_active);
    for mission in 0..=10 {
        for character in 0..2 {
            for layout in 1..=3 {
                let options = Options {
                    mission,
                    character,
                    layout,
                    ..Default::default()
                };
                let mut game = options.load(&app.assets).unwrap();
                game.activation.keep_all_active = true;
                game.events = Default::default();
                for _ in 0..300 {
                    game.step(0).unwrap_or_else(|e| {
                        panic!("mission {mission}, character {character}, layout {layout}: {e}")
                    });
                }
                for &id in &game.roster.order {
                    let actor = game.roster.get(id);
                    if id != game.player_id && actor.kind != 0xa9 && actor.health >= 0 {
                        assert_eq!(actor.attribute & tenchu_core::ids::ATTR_SUSPEND, 0);
                    }
                }
            }
        }
    }
    app.keep_enemies_active(true);
    app.campaign.screen = Screen::Launcher;
    app.action(Action::Confirm);
    app.action(Action::Confirm);
    assert!(app.campaign.game.activation.keep_all_active);
    app.apply_item(Item::ResetTweaks, 1).unwrap();
    assert!(!app.settings.keep_enemies_active && !app.campaign.game.activation.keep_all_active);
}

#[test]
#[ignore = "requires TENCHU_DISC; the same scripted showcase inputs produce the same game state at different display rates"]
fn showcase_simulation_is_independent_of_display_cadence() {
    let run = |fps: Option<u64>| {
        let mut app = application(Options::default());
        app.campaign.game.events = Default::default();
        app.demo = Some(crate::demo::Player::parse(&["--clips".into(), "motion".into()]).unwrap());
        let frames = fps.unwrap_or(60) * 6;
        for frame in 0..frames {
            if let Some(fps) = fps {
                app.advance_demo(Duration::from_nanos(
                    (frame + 1) * 1_000_000_000 / fps - frame * 1_000_000_000 / fps,
                ));
            } else {
                app.advance_demo_frame();
            }
            assert!(!app.faulted, "{:?}", app.fault);
        }
        assert_eq!(app.campaign.game.tick, 180);
        let state = crate::session::tests::state(&app.campaign.game);
        app.demo.as_mut().unwrap().library = true;
        app.advance_demo(Duration::from_secs(1));
        assert_eq!(state, crate::session::tests::state(&app.campaign.game));
        state
    };
    let expected = run(Some(60));
    for fps in [None, Some(24), Some(30), Some(144)] {
        assert_eq!(run(fps), expected);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; F1 edits in showcase mode must not overwrite normal preferences"]
fn showcase_settings_are_temporary_even_when_edited_through_f1() {
    let mut app = application(Options::default());
    let directory =
        std::env::temp_dir().join(format!("tenchu-demo-settings-{}", std::process::id()));
    app.settings_store.path = directory.join("settings.json");
    app.settings_store.write(&app.settings).unwrap();
    let bytes = std::fs::read(&app.settings_store.path).unwrap();
    let original = app.settings.clone();
    app.demo = Some(crate::demo::Player::parse(&[]).unwrap());
    app.apply_item(Item::Fps, 1).unwrap();
    app.apply_item(Item::Preset, 1).unwrap();
    app.apply_settings(&original);
    assert_eq!(std::fs::read(&app.settings_store.path).unwrap(), bytes);
    assert_ne!(app.settings, original);
    app.demo = None;
    app.apply_settings(&original);
    assert_eq!(app.settings_store.load().unwrap(), app.settings);
    std::fs::remove_dir_all(directory).unwrap();
}
