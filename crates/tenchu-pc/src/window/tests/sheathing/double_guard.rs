use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; preference defaults/reset, interrupted recordings, TRIAL restrictions and a saved first guard tap"]
fn double_guard_preferences_checkpoint_and_recording() {
    let legacy: Settings = serde_json::from_str("{}").unwrap();
    assert!(!legacy.sheathing.double_guard);
    use crate::settings::runtime::Runtime;
    assert!(Runtime::Gaisen.unavailable(Item::DoubleGuard).is_none());
    assert!(Runtime::Tenchu2.unavailable(Item::DoubleGuard).is_some());
    for hero in 0..2 {
        let mut app = ready(hero);
        app.set_circle_action(input::CircleAction::Guard);
        app.apply_item(Item::DoubleGuard, 1).unwrap();
        assert!(app.settings.sheathing.double_guard);
        assert!(
            app.campaign
                .game
                .player()
                .sheathing
                .as_ref()
                .unwrap()
                .settings
                .double_guard
        );
        assert!(!app.trial_records_eligible());
        for preset in [
            0,
            settings::presets::MODERN_HIGH,
            settings::presets::MODERN_ULTRA,
        ] {
            app.settings.set_visual_preset(preset);
            assert!(app.settings.sheathing.double_guard);
        }
        app.game_tick(input::CIRCLE);
        app.game_tick(0);
        let save = app.capture_state().unwrap();
        let rewind = app.capture_rewind().unwrap();
        let start = postcard::to_allocvec(&app.campaign.game).unwrap();
        app.game_tick(input::CIRCLE);
        assert_eq!(app.campaign.game.player().motion.action, MOT_STATE_SHEATHE);
        for _ in 0..80 {
            app.game_tick(input::CIRCLE);
        }
        assert_eq!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        let end = postcard::to_allocvec(&app.campaign.game).unwrap();
        for checkpoint in [true, false] {
            app.reset_item(Item::DoubleGuard).unwrap();
            if checkpoint {
                app.restore_state(&save).unwrap();
            } else {
                app.restore_rewind(&rewind).unwrap();
            }
            assert!(app.settings.sheathing.double_guard);
            assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), start);
            app.game_tick(input::CIRCLE);
            for _ in 0..80 {
                app.game_tick(input::CIRCLE);
            }
            assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), end);
        }
        app.reset_item(Item::DoubleGuard).unwrap();
        assert!(!app.settings.sheathing.double_guard);
        assert!(
            !app.campaign
                .game
                .player()
                .sheathing
                .as_ref()
                .unwrap()
                .settings
                .double_guard
        );
        app.apply_item(Item::DoubleGuard, 1).unwrap();
        app.set_gameplay_preset(0).unwrap();
        assert!(!app.settings.sheathing.double_guard);
        assert!(app.campaign.game.player().sheathing.is_none());
    }
    let mut app = application(Options::default());
    let directory = std::env::temp_dir().join(format!(
        "tenchu-double-guard-recording-{}",
        std::process::id()
    ));
    app.recording_directory = Some(directory.clone());
    app.campaign.screen = Screen::Launcher;
    app.action(Action::Confirm);
    app.action(Action::Confirm);
    assert!(app.recording.is_some());
    app.record_input(0);
    app.apply_item(Item::DoubleGuard, 1).unwrap();
    assert!(app.recording.is_none());
    assert!(
        app.trial_records_eligible(),
        "inactive preference should not change gameplay"
    );
    app.apply_item(Item::Sheathing, 1).unwrap();
    app.set_circle_action(input::CircleAction::Guard);
    assert!(!app.trial_records_eligible());
    app.campaign.screen = Screen::Launcher;
    app.action(Action::Confirm);
    app.action(Action::Confirm);
    assert!(app.recording.is_none());
    app.apply_item(Item::ResetTweaks, 1).unwrap();
    assert!(!app.settings.sheathing.double_guard);
    assert!(app.trial_records_eligible());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires TENCHU_DISC; both demo input tracks use real guard taps and hold without repeating; optional TENCHU_DOUBLE_GUARD_REPORT captures matched off/on states"]
fn double_guard_demo_tracks_toggle_once_in_both_directions() {
    let output = std::env::var_os("TENCHU_DOUBLE_GUARD_REPORT").map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path).unwrap();
    }
    for (hero, clip) in [(0, "double-guard-rikimaru"), (1, "double-guard-ayame")] {
        let demo = crate::demo::Player::parse(&["--clips".into(), clip.into()]).unwrap();
        assert_eq!(demo.clip().character, hero);
        for enabled in [false, true] {
            let mut app = ready(hero);
            crate::demo::animate(&demo, &mut app.settings, &mut app.debug);
            let mut settings = app.settings.sheathing;
            settings.double_guard = enabled;
            app.set_sheathing(settings).unwrap();
            app.set_circle_action(app.settings.circle_action);
            let mut transitions = Vec::new();
            for tick in 0..300 {
                let previous = app.campaign.game.player().motion.action;
                app.campaign.game.step(demo.buttons(tick)).unwrap();
                let actor = app.campaign.game.player();
                if previous != actor.motion.action
                    && matches!(actor.motion.action, MOT_STATE_DRAW | MOT_STATE_SHEATHE)
                {
                    transitions.push((tick, actor.motion.action));
                }
                if tick == 90 {
                    assert_eq!(actor.attribute & ATTR_WEAPON_DRAWN != 0, !enabled);
                    if let Some(output) = &output {
                        crate::sheathing::tests::capture_pose(
                            &app.campaign.game,
                            &output.join(format!("double-guard-{hero}-{enabled}.png")),
                        );
                    }
                }
                if tick == 210 {
                    assert_ne!(actor.attribute & ATTR_WEAPON_DRAWN, 0);
                }
            }
            if enabled {
                assert_eq!(
                    transitions,
                    [(33, MOT_STATE_SHEATHE), (153, MOT_STATE_DRAW)]
                );
            } else {
                assert!(transitions.is_empty());
            }
        }
    }
}
