use super::*;
use tenchu_core::{
    drawing::{Cadence, Step},
    input,
};

fn exact(game: &Game) -> Vec<u8> {
    postcard::to_allocvec(game).unwrap()
}

#[test]
#[ignore = "requires both discs; sandbox saves/rewind retain imported terrain and retry context"]
fn tenchu2_saves_require_the_matching_sandbox_and_preserve_retry() {
    let disc = std::env::var_os("TENCHU2_DISC").expect("Set TENCHU2_DISC");
    let disc = crate::source::open_disc(disc.as_ref()).unwrap();
    let mut app = application(Options::default());
    let normal = app.capture_state().unwrap();
    let sandbox = crate::tenchu2::Sandbox::open(&disc, Some(0), None, None).unwrap();
    app.campaign.game = sandbox.load(&app.assets, app.campaign.options).unwrap();
    app.campaign.sandbox = Some(sandbox);
    app.debug = crate::debug::Debug::new(&app.campaign.game).unwrap();
    for _ in 0..15 {
        app.campaign.game.step(input::UP).unwrap();
    }
    let imported = app.capture_state().unwrap();
    let rewind = app.capture_rewind().unwrap();
    let expected = exact(&app.campaign.game);
    for _ in 0..15 {
        app.campaign.game.step(input::UP).unwrap();
    }
    app.restore_state(&imported).unwrap();
    assert_eq!(exact(&app.campaign.game), expected);
    for _ in 0..15 {
        app.campaign.game.step(input::UP).unwrap();
    }
    app.restore_rewind(&rewind).unwrap();
    assert_eq!(exact(&app.campaign.game), expected);
    app.campaign.action(Action::Replay, &app.assets).unwrap();
    assert!(app.campaign.game.mission.world_vram.is_some());
    app.restore_state(&normal).unwrap();
    assert!(app.campaign.sandbox.is_none());
    assert!(
        app.restore_state(&imported)
            .unwrap_err()
            .contains("same Tenchu 2")
    );
}

fn continue_identically(original: &mut Application, restored: &mut Application, ticks: usize) {
    for tick in 0..ticks {
        let buttons = match tick % 80 {
            0..20 => input::UP,
            20..30 => input::UP | input::CROSS,
            30..50 => input::RIGHT,
            50..60 => input::SQUARE,
            _ => 0,
        };
        let cadence = Cadence {
            step: if tick % 7 == 0 {
                Step::Skipped
            } else {
                Step::Normal
            },
            display_swap: tick % 7 != 0,
        };
        original
            .campaign
            .game
            .step_recorded(buttons, cadence)
            .unwrap();
        restored
            .campaign
            .game
            .step_recorded(buttons, cadence)
            .unwrap();
        if tick % 60 == 0 {
            assert_eq!(
                exact(&original.campaign.game),
                exact(&restored.campaign.game)
            );
        }
    }
    assert_eq!(
        exact(&original.campaign.game),
        exact(&restored.campaign.game)
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; encounter damage settings and guard memory survive file saves and continue identically"]
fn savestates_preserve_detection_escalation() {
    let mut app = application(Options::default());
    app.apply_item(Item::DetectionDamage, 5).unwrap();
    app.apply_item(Item::GuardControls, 1).unwrap();
    app.apply_item(Item::DetectionDamageLimit, 2).unwrap();
    let options = app.settings.damage_escalation;
    assert_eq!(options.increment, 50);
    assert_eq!(options.maximum, 150);
    for actor in app.campaign.game.roster.actors.iter_mut().flatten() {
        actor.encounters.detections = 4;
    }
    let bytes = app.capture_state().unwrap();
    let mut restored = application(Options::default());
    restored.restore_state(&bytes).unwrap();
    assert_eq!(restored.settings.damage_escalation, options);
    assert!(restored.settings.guard_controls && restored.campaign.game.intelligence.guard_controls);
    assert_eq!(exact(&restored.campaign.game), exact(&app.campaign.game));
    continue_identically(&mut app, &mut restored, 120);
    restored.reset_item(Item::DetectionDamageLimit).unwrap();
    assert_eq!(restored.settings.damage_escalation.increment, 50);
    assert_eq!(restored.settings.damage_escalation.maximum, 100);
    assert_eq!(restored.campaign.game.player().encounters.detections, 4);
    restored.reset_item(Item::DetectionDamage).unwrap();
    assert!(!restored.settings.damage_escalation.enabled());
    assert!(
        restored
            .campaign
            .game
            .roster
            .actors
            .iter()
            .flatten()
            .all(|actor| actor.encounters.detections == 0)
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; restored TRIAL attempts remain editable but cannot award records"]
fn savestates_restore_trial_playtests_as_unranked_drafts() {
    let mut app = application(Options::default());
    app.open_trial();
    assert!(app.trial.open);
    app.trial_action(crate::trial::Event::Play { preview: false });
    assert!(!app.trial.open, "{}", app.trial.notice);
    for _ in 0..70 {
        app.campaign.game.step(0).unwrap();
    }
    let bytes = app.capture_state().unwrap();
    let mut restored = application(Options::default());
    restored.restore_state(&bytes).unwrap();
    assert!(restored.trial_previous.is_some());
    assert!(!restored.trial.eligible && !restored.trial.preview && !restored.trial.open);
    assert_eq!(restored.trial.mission, app.trial.mission);
    continue_identically(&mut app, &mut restored, 120);
}

#[test]
#[ignore = "requires TENCHU_DISC; active items, disguise, imported actors and randomizer context survive a persistent save"]
fn savestates_preserve_live_items_and_imported_actors() {
    use crate::visuals::appearance::library::{Library, import_actor};
    use tenchu_core::{ai::programs::Programs, items::Processor};

    for character in 0..2 {
        for slot in [0, 1, 4, 5, 11] {
            let mut app = application(Options {
                character,
                layout: 0,
                ..Default::default()
            });
            app.campaign.game.events = Default::default();
            for _ in 0..70 {
                app.campaign.game.step(0).unwrap();
            }
            app.campaign.game.set_inventory(slot, 2).unwrap();
            app.campaign.game.hud.cursor = slot;
            app.campaign.game.step(0).unwrap();
            app.campaign.game.step(input::TRIANGLE).unwrap();
            for _ in 0..120 {
                let ready = app.campaign.game.items.slots.iter().any(|item| {
                    if slot == 11 {
                        item.processor() == Some(Processor::Disguise) && item.source_mode() == 3
                    } else {
                        item.processor().is_some()
                    }
                });
                if ready {
                    break;
                }
                app.campaign.game.step(0).unwrap();
            }
            assert!(
                app.campaign
                    .game
                    .items
                    .slots
                    .iter()
                    .any(|i| i.processor().is_some()),
                "hero {character}, item {slot}"
            );
            if slot == 11 {
                assert_eq!(
                    app.campaign.game.player().mesh_kind,
                    app.campaign.game.mission.disguise
                );
            }
            let bytes = app.capture_state().unwrap();
            let mut restored = application(Options::default());
            restored.restore_state(&bytes).unwrap();
            continue_identically(&mut app, &mut restored, 680);
        }
    }

    let mut app = application(Options::default());
    let library = Library::load(&app.assets).unwrap();
    let run = crate::randomizer::Run::new(crate::randomizer::Document::default());
    app.campaign
        .start_randomizer(run, &app.assets, &library)
        .unwrap();
    app.campaign.game.events = Default::default();
    for _ in 0..70 {
        app.campaign.game.step(0).unwrap();
    }
    let source = import_actor(
        &mut app.campaign.game.mission,
        &library,
        "disc:144",
        &mut app.debug.actor_imports,
    )
    .unwrap();
    let position = app.campaign.game.player().position;
    app.campaign
        .game
        .place_actor(
            source.actor,
            Programs::decode(source.think).unwrap(),
            position,
            0,
            true,
        )
        .unwrap();
    app.debug.actors_edited = true;
    app.settings.navigation.camera_relative = true;
    let bytes = app.capture_state().unwrap();
    let path =
        std::env::temp_dir().join(format!("tenchu-state-test-{}.tstate", std::process::id()));
    crate::storage::atomic_write(&path, &bytes).unwrap();
    let mut restored = application(Options::default());
    restored.settings.graphics.blob_shadows = false;
    restored
        .restore_state(&crate::savestate::read(&path).unwrap())
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(restored.debug.actors_edited);
    assert_eq!(
        restored.debug.actor_imports.len(),
        app.debug.actor_imports.len()
    );
    assert!(restored.settings.navigation.camera_relative);
    assert!(!restored.settings.graphics.blob_shadows);
    assert_eq!(
        restored.campaign.randomized.as_ref().unwrap().run.export(),
        app.campaign.randomized.as_ref().unwrap().run.export()
    );
    continue_identically(&mut app, &mut restored, 300);
}

#[test]
#[ignore = "requires TENCHU_DISC; persistent snapshots must reproduce every simulation field and subsequent updates across all missions"]
fn savestates_restore_complete_missions_and_continue_identically() {
    for mission in 0..=10 {
        for character in 0..2 {
            let options = Options {
                mission,
                character,
                ..Default::default()
            };
            let mut original = application(options);
            for _ in 0..70 {
                original.campaign.game.step(0).unwrap();
            }
            let before = exact(&original.campaign.game);
            let started = Instant::now();
            let bytes = original.capture_state().unwrap();
            let encoded = started.elapsed();
            assert_eq!(before, exact(&original.campaign.game));
            let mut restored = application(Options::default());
            let disk_save = restored.campaign.saved.clone();
            let started = Instant::now();
            restored.restore_state(&bytes).unwrap();
            eprintln!(
                "M{mission} hero {character}: {} KiB; save {encoded:?}, load {:?}",
                bytes.len() / 1024,
                started.elapsed()
            );
            assert_eq!(before, exact(&restored.campaign.game));
            assert_eq!(restored.campaign.saved, disk_save);
            assert!(restored.campaign.save_request.is_none());
            assert!(restored.paused);
            assert!(restored.debug.pause_view && !restored.debug.frame_step);
            assert_eq!(restored.controls.buttons(), 0);
            continue_identically(&mut original, &mut restored, 180);
            let before = exact(&restored.campaign.game);
            assert!(restored.restore_state(&bytes[..bytes.len() / 2]).is_err());
            assert_eq!(
                before,
                exact(&restored.campaign.game),
                "failed load must retain the running game"
            );
        }
    }
}
