use super::*;
use crate::{settings::styles, visuals::appearance::library::Library};
use tenchu_core::{game::sheathing::Settings as Sheathing, ids::*, input};

fn unarmed_state(game: &Game) -> serde_json::Value {
    let mut state = serde_json::to_value(game.player().sheathing.as_ref().unwrap()).unwrap();
    // The fallback draw/turn/sheath clips follow the selected hero; the unarmed
    // style, imported attacks, held inputs and settings must survive the switch.
    state.as_object_mut().unwrap().remove("motions");
    state
}

#[test]
#[ignore = "requires both discs; selected styles preserve campaign/body/equipment/unarmed state and reject committed actions"]
fn gaisen_combat_style_cycle_preserves_identity_and_safe_transitions() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    for hero in 0..2 {
        for drawn in [false, true] {
            let mut app = application(Options {
                character: hero,
                ..Default::default()
            });
            app.campaign.game.events = Default::default();
            app.set_sheathing(Sheathing {
                enabled: true,
                unarmed: true,
                ..Default::default()
            })
            .unwrap();
            for _ in 0..30 {
                app.campaign.game.step(0).unwrap();
            }
            if !drawn {
                app.campaign
                    .game
                    .roster
                    .get_mut(app.campaign.game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(true);
                for _ in 0..90 {
                    app.campaign.game.step(0).unwrap();
                }
            }
            app.settings.player_model = "disc:0".into();
            app.settings.weapon_model = "tenchu2-jp:kagami".into();
            app.settings.voice_model = "tenchu2-jp:ayame".into();
            app.settings.combat_styles = [
                Some(String::new()),
                Some(format!("disc:{}", 1 - hero)),
                Some("tenchu2-jp:rikimaru".into()),
            ];
            library
                .apply_player(&mut app.campaign.game, &app.settings)
                .unwrap();
            let options = app.campaign.options;
            let preferences = app.settings.clone();
            for index in 0..6 {
                if index == 3 {
                    for _ in 0..4 {
                        app.campaign.game.step(input::UP).unwrap();
                    }
                }
                assert!(styles::gaisen_ready(&app.campaign.game));
                let game = &mut app.campaign.game;
                let rng = game.random;
                let position = game.player().position;
                let health = game.player().health;
                let kind = game.player().kind;
                let unarmed = unarmed_state(game);
                let npcs: Vec<_> = game
                    .roster
                    .actors
                    .iter()
                    .enumerate()
                    .filter(|(id, _)| *id != game.player_id)
                    .map(|(_, a)| postcard::to_allocvec(a).unwrap())
                    .collect();
                let notice = styles::switch_gaisen(game, &mut app.settings, &library).unwrap();
                assert!(notice.starts_with("Combat style: "));
                assert_eq!(game.random, rng);
                assert_eq!(
                    (
                        game.player().kind,
                        game.player().health,
                        game.player().position
                    ),
                    (kind, health, position)
                );
                assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN != 0, drawn);
                assert_eq!(unarmed_state(game), unarmed);
                assert_eq!(
                    game.roster
                        .actors
                        .iter()
                        .enumerate()
                        .filter(|(id, _)| *id != game.player_id)
                        .map(|(_, a)| postcard::to_allocvec(a).unwrap())
                        .collect::<Vec<_>>(),
                    npcs
                );
                assert_eq!(app.settings.player_model, preferences.player_model);
                assert_eq!(app.settings.weapon_model, preferences.weapon_model);
                assert_eq!(app.settings.voice_model, preferences.voice_model);
                assert_eq!(app.settings.sheathing, preferences.sheathing);
                assert_eq!(app.campaign.options, options);
                assert!(!game.collision.slots[..game.collision.len].iter().any(
                    |slot| slot.actor() == Some(game.player_id)
                        && slot.class & tenchu_core::collision::HIT != 0
                        && slot.model.is_some_and(|id| game.scene.models[id].attribute
                            & tenchu_core::scene::COLLIDE
                            != 0)
                ));
            }
            assert!(!app.settings.model_moveset);
            app.campaign.game.step(input::SQUARE).unwrap();
            let original = postcard::to_allocvec(&app.campaign.game).unwrap();
            let settings = app.settings.clone();
            assert!(
                styles::switch_gaisen(&mut app.campaign.game, &mut app.settings, &library).is_err()
            );
            assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), original);
            assert_eq!(app.settings, settings);
            for _ in 0..120 {
                app.campaign.game.step(0).unwrap();
            }
            styles::switch_gaisen(&mut app.campaign.game, &mut app.settings, &library).unwrap();
            let saved = app.capture_state().unwrap();
            let rewind = app.capture_rewind().unwrap();
            let expected = postcard::to_allocvec(&app.campaign.game).unwrap();
            app.settings.combat_styles = Default::default();
            app.restore_state(&saved).unwrap();
            assert_eq!(app.settings.combat_styles, preferences.combat_styles);
            assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), expected);
            app.settings.combat_styles = Default::default();
            app.restore_rewind(&rewind).unwrap();
            assert_eq!(app.settings.combat_styles, preferences.combat_styles);
            assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), expected);
            app.reverse.preview = true;
            assert!(app.switch_combat_style().unwrap_err().contains("rewind"));
        }
    }
}

#[test]
#[ignore = "requires both discs; unavailable/duplicate aliases do not create a cycle, and busy input leaves source state unchanged"]
fn gaisen_combat_style_unavailable_sources_and_busy_input_are_unchanged() {
    let mut app = application(Default::default());
    app.campaign.game.events = Default::default();
    for _ in 0..30 {
        app.campaign.game.step(0).unwrap();
    }
    let mut library = Library::load(&app.assets).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let ayame = library.get("disc:1").unwrap().id.clone();
    app.settings.combat_styles = [
        Some("disc:1".into()),
        Some(ayame),
        Some("missing:source".into()),
    ];
    let original = postcard::to_allocvec(&app.campaign.game).unwrap();
    assert!(styles::switch_gaisen(&mut app.campaign.game, &mut app.settings, &library).is_err());
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), original);
    app.settings.combat_styles[2] = Some(String::new());
    assert!(styles::next(&app.settings, &library, |_| true).is_ok());
    assert!(app.trial_records_eligible());
    let original_settings = app.settings.clone();
    styles::switch_gaisen(&mut app.campaign.game, &mut app.settings, &library).unwrap();
    assert!(!app.trial_records_eligible());
    app.settings = original_settings;
    app.campaign.game = postcard::from_bytes(&original).unwrap();
    for action in [
        MOT_ATTACK,
        MOT_JUMP,
        MOT_DAMAGE_FRONT_MID,
        MOT_ENGAGE_GUARD,
        MOT_SQUAT,
        MOT_STATE_DRAW,
    ] {
        let game = &mut app.campaign.game;
        game.roster.get_mut(game.player_id).motion.action = action;
        game.roster.get_mut(game.player_id).status = tenchu_core::actor::State::from_motion(action);
        let before = postcard::to_allocvec(game).unwrap();
        assert!(
            styles::switch_gaisen(game, &mut app.settings, &library).is_err(),
            "{action:x}"
        );
        assert_eq!(postcard::to_allocvec(game).unwrap(), before);
    }
    app.campaign.game = postcard::from_bytes(&original).unwrap();
    app.recording_directory =
        Some(std::env::temp_dir().join(format!("tenchu-style-record-{}", std::process::id())));
    app.campaign.screen = Screen::Launcher;
    app.action(Action::Confirm);
    app.action(Action::Confirm);
    assert!(app.recording.is_some());
    let original = postcard::to_allocvec(&app.campaign.game).unwrap();
    assert!(app.switch_combat_style().unwrap_err().contains("recording"));
    assert!(app.recording.is_some());
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), original);
    app.finish_recording();
}
