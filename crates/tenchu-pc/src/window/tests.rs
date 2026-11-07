use super::*;
use std::time::Duration;

mod actors;
mod behavior;
mod bleeding;
mod camera;
mod death;
mod rewind;
mod savestate;
mod sheathing;
mod tracking;
mod walking;

pub(super) fn application(options: Options) -> Application {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.screen = Screen::Playing;
    let debug = crate::debug::Debug::new(&campaign.game).unwrap();
    Application {
        tenchu2: Default::default(),
        frontend: None,
        trial: crate::trial::Editor::new(
            std::env::temp_dir().join(format!("tenchu-trial-test-{}.json", std::process::id())),
        ),
        trial_previous: None,
        assets,
        campaign,
        storage: Storage::temporary(),
        view: None,
        clock: GameClock::default(),
        last_frame: Instant::now(),
        controls: Controls::default(),
        touch: Default::default(),
        orbit: Default::default(),
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
        recent_inputs: Default::default(),
        rewind: Default::default(),
        reverse: Default::default(),
        benchmarks: Default::default(),
        presets: settings::library::Manager::new(std::env::temp_dir().join(format!(
            "tenchu-unused-test-presets-{}.json",
            std::process::id()
        ))),
        randomizer: crate::randomizer::Editor::new(std::env::temp_dir().join(format!(
            "tenchu-unused-test-randomizer-{}.json",
            std::process::id()
        ))),
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
#[ignore = "requires TENCHU_DISC; a real quick-launch action must not retrigger a held touch stick"]
fn quick_launcher_touch_drag_moves_one_row_until_recentered() {
    use winit::event::TouchPhase;
    let mut app = application(Default::default());
    app.settings.frontend.auto_save = false;
    app.campaign.screen = Screen::Launcher;
    app.campaign.cursor = 0;
    app.touch.layout(
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 540.0)),
        crate::touch::Settings {
            visibility: crate::touch::Visibility::Always,
            ..Default::default()
        },
        crate::touch::Mode::Menus,
        false,
        false,
    );
    for (i, y) in std::iter::once(490.0)
        .chain((0..150).map(|i| 490.0 + (i % 3) as f32))
        .enumerate()
    {
        app.touch.event(
            1,
            if i == 0 {
                TouchPhase::Started
            } else {
                TouchPhase::Moved
            },
            egui::pos2(95.0, y),
        );
        app.controls.touch(app.touch.sample());
        let update = app.controls.poll(true);
        if let Some(Command::Menu(action)) =
            controls::gamepad_command(update.menu_pressed, false, false, false)
        {
            app.action(action);
        }
        assert_eq!(app.campaign.cursor, 1, "drag event {i}");
    }
    app.touch
        .event(1, TouchPhase::Moved, egui::pos2(95.0, 440.0));
    app.controls.touch(app.touch.sample());
    app.touch
        .event(1, TouchPhase::Moved, egui::pos2(95.0, 390.0));
    app.controls.touch(app.touch.sample());
    let update = app.controls.poll(true);
    let Some(Command::Menu(action)) =
        controls::gamepad_command(update.menu_pressed, false, false, false)
    else {
        panic!("recentered stick must select again")
    };
    app.action(action);
    assert_eq!(app.campaign.cursor, 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; results keyboard/controller retry must preserve the mission and awarded progress"]
fn results_start_retries_while_confirm_advances() {
    use tenchu_core::input;

    for mission in [1, 10] {
        for (command, retry) in [
            (controls::keyboard_command(KeyCode::Space, false), true),
            (
                controls::gamepad_command(input::START, false, false, false),
                true,
            ),
            (controls::keyboard_command(KeyCode::F5, false), true),
            (controls::keyboard_command(KeyCode::Enter, false), false),
            (
                controls::gamepad_command(input::START, false, false, true),
                true,
            ),
            (
                controls::gamepad_command(input::CIRCLE, false, false, true),
                false,
            ),
            (
                controls::gamepad_command(input::CROSS, false, false, false),
                false,
            ),
        ] {
            let mut app = application(Options {
                mission,
                ..Default::default()
            });
            app.settings.frontend.auto_save = false;
            app.campaign.game.events.outcome = tenchu_core::game::events::Outcome::Complete;
            app.campaign.tick(0).unwrap();
            assert!(matches!(app.campaign.screen, Screen::Results(_)));
            let progress = app.campaign.progress.clone();
            let seed = app.campaign.seed;
            let Some(Command::Menu(action)) = command else {
                panic!("Expected a menu command");
            };
            app.action(action);
            assert!(!app.faulted, "{:?}", app.fault);
            assert_eq!(app.campaign.progress, progress);
            assert_eq!(app.campaign.seed, seed);
            if retry {
                assert_eq!(app.campaign.options.mission, mission);
                assert!(matches!(app.campaign.screen, Screen::Inventory(_)));
            } else if mission == 10 {
                assert!(matches!(app.campaign.screen, Screen::Complete));
            } else {
                assert_eq!(app.campaign.options.mission, mission + 1);
                assert!(matches!(app.campaign.screen, Screen::Inventory(_)));
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; optional death hold preserves retail death timing and freezes gameplay until confirmation"]
fn death_hold_preserves_the_final_scene_and_requires_a_new_confirmation() {
    for mission in [1, 4, 9] {
        for character in 0..2 {
            let options = Options {
                character,
                mission,
                ..Default::default()
            };
            let mut app = application(options);
            let mut reference = application(options);
            app.settings.wait_after_death = true;
            app.settings.pause_in_menu = character == 0;
            app.settings.frontend.auto_save = false;
            let mut blood = crate::visuals::blood::Blood::default();
            let mut deaths = crate::visuals::death::Deaths::load(&app.assets).unwrap();
            let graphics = settings::Graphics {
                blood: settings::Blood::modern(),
                ..Default::default()
            };
            let models = crate::visuals::models::Models::build(
                &app.campaign.game.mission,
                &Default::default(),
                &graphics,
            )
            .unwrap();
            let surfaces = crate::visuals::surfaces::Surfaces::new(&render_mesh::stage(
                &app.campaign.game.mission,
                &app.campaign.game.math,
                Some(&models),
            ));
            for game in [&mut app.campaign.game, &mut reference.campaign.game] {
                let events = std::mem::take(&mut game.events);
                for _ in 0..30 {
                    game.step(0).unwrap();
                }
                game.events = events;
                let player = game.roster.get_mut(game.player_id);
                player.health = 0;
                player.status = tenchu_core::actor::State::Dead;
                player.motion.change(
                    tenchu_core::ids::MOT_DEAD,
                    &game.mission.characters[player.definition].motions,
                    &game.mission.common_motions,
                    &game.mission.clips,
                );
            }
            app.controls.key(KeyCode::Space, true);
            for _ in 0..3000 {
                reference.campaign.tick(0).unwrap();
                let running = app.game_tick(0);
                deaths.tick(&app.campaign.game, 100).unwrap();
                blood.tick(
                    &app.campaign.game,
                    &models,
                    &surfaces,
                    &graphics.blood,
                    deaths.frame.as_ref(),
                );
                if !running {
                    break;
                }
            }
            assert!(reference.campaign.playing());
            assert!(deaths.frame.is_some(), "M{mission} hero {character}");
            assert_eq!(app.campaign.game.death_wait_ticks(), Some(1));
            assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
            assert!(app.campaign.scene_visible() && !app.campaign.playing());
            assert_eq!(app.controls.buttons(), 0);
            assert!(
                blood.pages().any(|p| !p.is_empty()),
                "pool must finish before holding"
            );
            let held_state = crate::session::tests::state(&reference.campaign.game);
            let effects = app.campaign.game.effects.billboards.len();
            assert!(
                effects > 0,
                "hold the complete scene, including its flames and effects"
            );
            for _ in 0..60 {
                app.advance_simulation(Duration::from_secs(1));
            }
            app.debug.frame_step = true;
            app.advance_simulation(Duration::ZERO);
            app.action(Action::Up);
            assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
            assert!(crate::session::tests::state(&app.campaign.game) == held_state);
            assert_eq!(app.campaign.game.effects.billboards.len(), effects);
            while reference.campaign.playing() {
                reference.campaign.tick(0).unwrap();
            }
            let final_state = crate::session::tests::state(&reference.campaign.game);
            app.action(Action::Start);
            assert!(matches!(app.campaign.screen, Screen::GameOver));
            assert!(crate::session::tests::state(&app.campaign.game) == final_state);
            assert_eq!(app.campaign.seed, reference.campaign.seed);
            app.action(Action::Confirm);
            assert!(matches!(app.campaign.screen, Screen::Inventory(_)));
            assert_eq!(
                app.campaign.progress.stock,
                reference.campaign.progress.stock
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; post-death patrols and effects continue without altering failure/retry progress or RNG"]
fn live_death_scene_keeps_patrols_running_without_changing_retry() {
    use tenchu_core::{actor::State, ai::Halt, game::events::Outcome, ids::MOT_DEAD};

    for mission in [1, 4, 9] {
        for character in 0..2 {
            let options = Options {
                mission,
                character,
                ..Default::default()
            };
            let mut app = application(options);
            let mut reference = application(options);
            app.apply_item(Item::LiveDeathScene, 1).unwrap();
            assert!(app.settings.wait_after_death);
            let saved: Settings =
                serde_json::from_str(&serde_json::to_string(&app.settings).unwrap()).unwrap();
            assert!(saved.live_death_scene && saved.wait_after_death);
            app.settings.frontend.auto_save = false;
            for game in [&mut app.campaign.game, &mut reference.campaign.game] {
                game.roster.keep_all_active = true;
                let player = game.roster.get_mut(game.player_id);
                player.health = 0;
                player.status = State::Dead;
                player.motion.change(
                    MOT_DEAD,
                    &game.mission.characters[player.definition].motions,
                    &game.mission.common_motions,
                    &game.mission.clips,
                );
            }
            for _ in 0..3000 {
                reference.campaign.tick(0).unwrap();
                if !app.game_tick(0) {
                    break;
                }
            }
            assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
            assert_eq!(app.campaign.game.death_wait_ticks(), Some(1));
            let positions: Vec<_> = app
                .campaign
                .game
                .roster
                .actors
                .iter()
                .map(|a| {
                    a.as_ref()
                        .filter(|a| a.health > 0 && a.patrol.is_some())
                        .map(|a| a.position)
                })
                .collect();
            for _ in 0..600 {
                assert!(app.game_tick(tenchu_core::input::SQUARE));
            }
            let game = &app.campaign.game;
            assert!(
                positions.iter().enumerate().any(|(id, position)| {
                    position.is_some_and(|p| game.roster.get(id).position != p)
                }),
                "No patrol moved after death in mission {mission}, character {character}"
            );
            assert!(
                !game.effects.billboards.is_empty(),
                "Flames/effects disappeared"
            );
            assert_eq!(game.player().status, State::Dead);
            assert_eq!(game.player().health, 0);
            assert_eq!(game.player().pad.held, 0);
            assert_eq!(game.events.outcome, Outcome::Running);
            assert_eq!(game.intelligence.action_halt, Halt::StageEnd);
            assert!(!game.intelligence.ignore_player);
            assert_eq!(game.death_wait_ticks(), Some(1));
            assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
            let tick = game.tick;
            app.reset_item(Item::LiveDeathScene).unwrap();
            app.advance_simulation(Duration::from_secs(1));
            assert_eq!(app.campaign.game.tick, tick);
            app.apply_item(Item::LiveDeathScene, 1).unwrap();
            app.apply_item(Item::Pause, 1).unwrap();
            app.advance_simulation(Duration::from_secs(1));
            assert_eq!(app.campaign.game.tick, tick);
            app.apply_item(Item::Step, 1).unwrap();
            app.advance_simulation(Duration::ZERO);
            assert_eq!(app.campaign.game.tick, tick + 1);
            app.apply_item(Item::Pause, 1).unwrap();
            app.advance_simulation(Duration::from_secs(1));
            assert!(app.campaign.game.tick > tick);
            while reference.campaign.playing() {
                reference.campaign.tick(0).unwrap();
            }
            app.action(Action::Start);
            assert!(matches!(app.campaign.screen, Screen::GameOver));
            assert_eq!(app.campaign.seed, reference.campaign.seed);
            assert_eq!(
                app.campaign.game.random.seed,
                reference.campaign.game.random.seed
            );
            assert_eq!(app.campaign.progress, reference.campaign.progress);
            for application in [&mut app, &mut reference] {
                application.action(Action::Confirm);
                assert!(matches!(application.campaign.screen, Screen::Inventory(_)));
                application.action(Action::Confirm);
                assert!(application.campaign.playing());
            }
            assert!(
                crate::session::tests::state(&app.campaign.game)
                    == crate::session::tests::state(&reference.campaign.game)
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; camera-relative input must leave actual scripted sequences unchanged and resume afterward"]
fn camera_relative_controls_resume_after_original_cutscenes() {
    for character in 0..2 {
        let options = Options {
            character,
            mission: 2,
            ..Default::default()
        };
        let mut app = application(options);
        let mut reference = application(options);
        app.settings.navigation.camera_relative = true;
        app.campaign.tick(0).unwrap();
        reference.campaign.tick(0).unwrap();
        assert!(app.campaign.game.cinematic.active);
        for _ in 0..5000 {
            let input = tenchu_core::input::UP | tenchu_core::input::LEFT;
            let buttons = app.navigation_input(input);
            app.campaign.tick(buttons).unwrap();
            reference.campaign.tick(input).unwrap();
            assert!(
                crate::session::tests::state(&app.campaign.game)
                    == crate::session::tests::state(&reference.campaign.game),
                "hero {character}, cinematic update {}",
                app.campaign.game.tick
            );
            if app.campaign.game.cinematic.active {
                assert!(!app.campaign.game.player_can_orbit());
            } else {
                break;
            }
        }
        assert!(!app.campaign.game.cinematic.active);
        assert!(app.campaign.game.player_can_orbit());
        for _ in 0..30 {
            if app.campaign.game.player_can_steer() {
                break;
            }
            app.campaign.tick(0).unwrap();
        }
        assert_eq!(
            app.navigation_input(tenchu_core::input::LEFT),
            tenchu_core::input::UP
        );
        assert!(app.campaign.game.navigation.heading.is_some());
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
#[ignore = "requires TENCHU_DISC; reinforcement settings persist through reloads and graphics changes, and gameplay presets restore safe defaults"]
fn reinforcement_call_settings_apply_and_reset() {
    use tenchu_core::roster::ReinforcementCall;

    let mut app = application(Options::default());
    app.apply_item(Item::KeepEnemiesActive, 1).unwrap();
    app.apply_item(Item::ReinforcementsPerCall, 3).unwrap();
    app.apply_item(Item::RandomReinforcements, 1).unwrap();
    app.apply_item(Item::WitnessedKillReinforcements, 1)
        .unwrap();
    let call = ReinforcementCall {
        count: 4,
        random: true,
        witnessed_kills: true,
    };
    assert_eq!(app.campaign.game.roster.reinforcement_call, call);
    let saved: Settings =
        serde_json::from_str(&serde_json::to_string(&app.settings).unwrap()).unwrap();
    assert_eq!(saved.reinforcement_call, call);
    app.settings
        .set_visual_preset(settings::presets::MODERN_HIGH);
    assert_eq!(app.settings.reinforcement_call, call);
    app.campaign.screen = Screen::Launcher;
    app.action(Action::Confirm);
    app.action(Action::Confirm);
    assert_eq!(app.campaign.game.roster.reinforcement_call, call);
    assert!(settings::gameplay::current(&app.settings, &app.debug).is_none());
    app.reset_item(Item::ReinforcementsPerCall).unwrap();
    assert_eq!(
        app.campaign.game.roster.reinforcement_call,
        ReinforcementCall { count: 1, ..call }
    );
    app.reset_item(Item::RandomReinforcements).unwrap();
    assert!(app.campaign.game.roster.reinforcement_call.witnessed_kills);
    app.reset_item(Item::WitnessedKillReinforcements).unwrap();
    assert_eq!(
        app.campaign.game.roster.reinforcement_call,
        Default::default()
    );
    app.set_reinforcement_call(call);
    app.set_gameplay_preset(1).unwrap();
    assert_eq!(
        app.campaign.game.roster.reinforcement_call,
        Default::default()
    );
    assert_eq!(
        settings::gameplay::current(&app.settings, &app.debug),
        Some(1)
    );
    app.set_reinforcement_call(call);
    app.apply_item(Item::ResetTweaks, 1).unwrap();
    assert_eq!(
        app.campaign.game.roster.reinforcement_call,
        Default::default()
    );
    assert!(!app.settings.keep_enemies_active);
}

#[test]
#[ignore = "requires TENCHU_DISC; all mission/layout rosters run with activation limits disabled"]
fn all_enemy_activation_survives_layouts_and_mission_reloads() {
    let mut app = application(Options::default());
    app.apply_item(Item::KeepEnemiesActive, 1).unwrap();
    assert!(app.campaign.game.roster.keep_all_active);
    app.reset_item(Item::KeepEnemiesActive).unwrap();
    assert!(!app.campaign.game.roster.keep_all_active);
    let saved: Settings = serde_json::from_str(r#"{"keep_enemies_active":true}"#).unwrap();
    assert!(saved.keep_enemies_active);
    assert_eq!(saved.reinforcement_limit, 0);
    app.apply_item(Item::ReinforcementLimit, 6).unwrap();
    assert_eq!(app.settings.reinforcement_limit, 60);
    assert_eq!(app.campaign.game.roster.reinforcement_limit, Some(60));
    let saved: Settings =
        serde_json::from_str(&serde_json::to_string(&app.settings).unwrap()).unwrap();
    assert_eq!(saved.reinforcement_limit, 60);
    app.settings.set_visual_preset(10);
    assert_eq!(app.settings.reinforcement_limit, 60);
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
                game.roster.keep_all_active = true;
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
    assert!(app.campaign.game.roster.keep_all_active);
    assert_eq!(app.campaign.game.roster.reinforcement_limit, Some(60));
    app.reset_item(Item::ReinforcementLimit).unwrap();
    assert_eq!(app.campaign.game.roster.reinforcement_limit, None);
    app.set_reinforcement_limit(100);
    app.set_gameplay_preset(1).unwrap();
    assert_eq!(app.campaign.game.roster.reinforcement_limit, None);
    app.apply_item(Item::ResetTweaks, 1).unwrap();
    assert!(!app.settings.keep_enemies_active && !app.campaign.game.roster.keep_all_active);
}

#[test]
#[ignore = "requires TENCHU_DISC; combat options persist, apply on reload and reset without being changed by graphics"]
fn combat_settings_apply_and_reset() {
    for (index, item) in [
        Item::IndependentAttacks,
        Item::MultiTargetAttacks,
        Item::AttackAssist,
        Item::BehaviorProfiles,
        Item::QuickShurikenDodge,
        Item::StrictCorpseSight,
        Item::GuardControls,
        Item::AnalogWalking,
        Item::Sheathing,
        Item::DetectionDamage,
    ]
    .into_iter()
    .enumerate()
    {
        let enabled = |app: &Application| match item {
            Item::IndependentAttacks => app.campaign.game.intelligence.independent_attacks,
            Item::AttackAssist => app.campaign.game.combat.attack_assist,
            Item::BehaviorProfiles => app.campaign.game.intelligence.profiles.enabled,
            Item::QuickShurikenDodge => app.campaign.game.intelligence.quick_shuriken_dodge,
            Item::StrictCorpseSight => app.campaign.game.intelligence.strict_corpse_sight,
            Item::GuardControls => app.campaign.game.intelligence.guard_controls,
            Item::AnalogWalking => app.campaign.game.walking.is_some(),
            Item::Sheathing => app.campaign.game.player().sheathing.is_some(),
            Item::DetectionDamage => app.campaign.game.intelligence.damage_escalation.enabled(),
            _ => app.campaign.game.combat.multi_target,
        };
        let selected = |settings: &Settings| match item {
            Item::IndependentAttacks => settings.independent_attacks,
            Item::AttackAssist => settings.attack_assist,
            Item::BehaviorProfiles => settings.behavior_profiles.enabled,
            Item::QuickShurikenDodge => settings.quick_shuriken_dodge,
            Item::StrictCorpseSight => settings.strict_corpse_sight,
            Item::GuardControls => settings.guard_controls,
            Item::AnalogWalking => settings.walking.enabled,
            Item::Sheathing => settings.sheathing.enabled,
            Item::DetectionDamage => settings.damage_escalation.enabled(),
            _ => settings.multi_target_attacks,
        };
        let mut app = application(Options::default());
        let directory = std::env::temp_dir().join(format!(
            "tenchu-combat-options-recording-{}-{index}",
            std::process::id()
        ));
        app.recording_directory = Some(directory.clone());
        let original: Settings = serde_json::from_str("{}").unwrap();
        assert!(!selected(&original));
        app.apply_item(item, 1).unwrap();
        assert!(enabled(&app));
        let saved: Settings =
            serde_json::from_str(&serde_json::to_string(&app.settings).unwrap()).unwrap();
        assert!(selected(&saved));
        app.settings
            .set_visual_preset(settings::presets::MODERN_HIGH);
        app.settings.retro = true;
        assert!(selected(&app.settings.rendering()));
        app.campaign.screen = Screen::Launcher;
        app.action(Action::Confirm);
        app.action(Action::Confirm);
        assert!(enabled(&app));
        assert!(app.recording.is_none());
        assert!(settings::gameplay::current(&app.settings, &app.debug).is_none());
        app.reset_item(item).unwrap();
        assert!(!enabled(&app));
        app.apply_item(item, 1).unwrap();
        app.set_gameplay_preset(1).unwrap();
        assert!(!enabled(&app));
        app.apply_item(item, 1).unwrap();
        app.apply_item(Item::ResetTweaks, 1).unwrap();
        assert!(!enabled(&app));
        app.campaign.screen = Screen::Launcher;
        app.action(Action::Confirm);
        app.action(Action::Confirm);
        assert!(app.recording.is_some());
        app.record_input(0);
        app.apply_item(item, 1).unwrap();
        assert!(app.recording.is_none());
        let recordings: Vec<_> = std::fs::read_dir(&directory).unwrap().collect();
        assert_eq!(recordings.len(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
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

#[test]
#[ignore = "requires TENCHU_DISC; graphics comparison and gameplay profiles remain independent"]
fn remake_presets_keep_graphics_inventory_and_gameplay_separate() {
    let mut app = application(Options::default());
    for _ in 0..40 {
        app.campaign.tick(0).unwrap();
    }
    app.settings
        .set_visual_preset(settings::presets::MODERN_HIGH);
    let graphics = app.settings.graphics.clone();
    app.set_inventory(1, 17).unwrap();
    let inventory = app.debug.inventory;
    for index in 0..settings::gameplay::PRESETS.len() {
        app.set_gameplay_preset(index).unwrap();
        assert_eq!(
            settings::gameplay::current(&app.settings, &app.debug),
            Some(index)
        );
        assert_eq!(app.debug.inventory, inventory);
        assert_eq!(app.settings.graphics, graphics);
        assert_eq!(
            app.campaign.game.intelligence.sight_percent,
            app.settings.sight_percent
        );
        let state = crate::session::tests::state(&app.campaign.game);
        let selected = app.settings.clone();
        app.apply_item(Item::Retro, 1).unwrap();
        let _ = app.settings.rendering();
        assert_eq!(crate::session::tests::state(&app.campaign.game), state);
        app.apply_item(Item::Retro, 1).unwrap();
        assert_eq!(app.settings, selected);
    }
    app.set_gameplay_preset(0).unwrap();
    assert!(!app.campaign.game.player().invulnerable);
    assert!(!app.campaign.game.intelligence.ignore_player);
    assert!(!app.campaign.game.roster.keep_all_active);
    assert!(!app.debug.click_teleport);
    assert_eq!(app.settings.sight_percent, 100);
    assert_eq!(app.debug.inventory, inventory);
}
