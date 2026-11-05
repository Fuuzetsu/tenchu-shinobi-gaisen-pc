use super::*;
use tenchu_core::{ids::*, input};

fn ready(character: i16) -> Application {
    let mut app = application(Options {
        character,
        ..Default::default()
    });
    app.campaign.game.events = Default::default();
    app.set_walking(true).unwrap();
    for _ in 0..20 {
        app.campaign.game.step(0).unwrap();
    }
    app
}

#[test]
#[ignore = "requires TENCHU_DISC; borrowed walk cycles retain both heroes' weapons, combat, collisions and run transitions"]
fn walking_cycles_transition_and_preserve_hero_actions() {
    for character in 0..2 {
        let mut app = ready(character);
        let game = &mut app.campaign.game;
        let walk = game.walking.unwrap().clip;
        let expected_speed = if character == 0 { 38 } else { 31 };
        assert_eq!(game.mission.clips[walk].forward_speed, expected_speed);
        let weapons = game
            .player()
            .weapons
            .each_ref()
            .map(|w| w.as_ref().map(|w| (w.kind, w.model)));
        let definition = game.player().definition;
        game.walking.as_mut().unwrap().held = true;
        let origin = game.player().position;
        for _ in 0..40 {
            game.step(input::UP).unwrap();
        }
        assert_eq!(game.player().motion.clip, Some(walk));
        assert_eq!(game.player().motion.action, MOT_CHASE);
        assert_eq!(
            game.player()
                .weapons
                .each_ref()
                .map(|w| w.as_ref().map(|w| (w.kind, w.model))),
            weapons
        );
        assert_eq!(game.player().definition, definition);
        assert_ne!(game.player().position, origin);
        assert!(game.player().attribute & ATTR_WEAPON_DRAWN != 0);
        let slow = game.player().velocity[0]
            .abs()
            .max(game.player().velocity[2].abs());
        game.walking.as_mut().unwrap().held = false;
        game.step(input::UP).unwrap();
        assert_ne!(game.player().motion.clip, Some(walk));
        assert!(
            game.player().velocity[0]
                .abs()
                .max(game.player().velocity[2].abs())
                > slow * 2
        );
        game.walking.as_mut().unwrap().held = true;
        game.step(input::UP).unwrap();
        assert_eq!(game.player().motion.clip, Some(walk));
        game.set_player_walk(None).unwrap();
        assert_ne!(game.player().motion.clip, Some(walk));
        assert!(
            game.player().velocity[0]
                .abs()
                .max(game.player().velocity[2].abs())
                > slow * 2
        );
        for button in [input::SQUARE, input::CROSS, input::CIRCLE, input::TRIANGLE] {
            let mut app = ready(character);
            let game = &mut app.campaign.game;
            let walk = game.walking.unwrap().clip;
            game.walking.as_mut().unwrap().held = true;
            for _ in 0..15 {
                game.step(input::UP).unwrap();
            }
            game.step(input::UP | button).unwrap();
            assert_ne!(
                game.player().motion.clip,
                Some(walk),
                "hero {character}, button {button:x}"
            );
            for _ in 0..80 {
                game.step(input::UP | button).unwrap();
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; enabling walking without selecting it leaves every original game update unchanged"]
fn unused_walking_preserves_original_simulation() {
    for character in 0..2 {
        let mut off = ready(character);
        let mut on = ready(character);
        off.set_walking(false).unwrap();
        for tick in 0..240 {
            let buttons = match tick % 120 {
                0..40 => input::UP,
                40..50 => input::UP | input::LEFT,
                50..60 => input::CIRCLE,
                60..80 => input::DOWN,
                80..85 => input::SQUARE,
                85..90 => input::CROSS,
                _ => 0,
            };
            off.campaign.game.step(buttons).unwrap();
            on.campaign.game.step(buttons).unwrap();
            if tick % 30 == 0 {
                let walking = on.campaign.game.walking.take();
                assert_eq!(
                    postcard::to_allocvec(&off.campaign.game).unwrap(),
                    postcard::to_allocvec(&on.campaign.game).unwrap(),
                    "hero {character}, tick {tick}"
                );
                on.campaign.game.walking = walking;
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; keyboard modifier and walking state survive file saves and rewind, including clip imports"]
fn walking_input_and_saved_continuation() {
    for character in 0..2 {
        let mut app = ready(character);
        app.controls.key(KeyCode::KeyW, true);
        app.controls.key(KeyCode::AltLeft, true);
        for _ in 0..10 {
            let buttons = app.player_input();
            assert_eq!(buttons, input::UP);
            assert!(app.campaign.game.walking.unwrap().held);
            assert!(app.game_tick(buttons));
        }
        let walk = app.campaign.game.walking.unwrap().clip;
        assert_eq!(app.campaign.game.player().motion.clip, Some(walk));
        let saved = app.capture_state().unwrap();
        let mut restored = application(Options {
            character,
            ..Default::default()
        });
        restored.restore_state(&saved).unwrap();
        assert!(restored.settings.walking.enabled);
        assert!(restored.campaign.game.walking.unwrap().held);
        for tick in 0..120 {
            let held = tick % 60 < 30;
            for game in [&mut app.campaign.game, &mut restored.campaign.game] {
                game.walking.as_mut().unwrap().held = held;
                game.step(input::UP).unwrap();
            }
        }
        assert_eq!(
            postcard::to_allocvec(&app.campaign.game).unwrap(),
            postcard::to_allocvec(&restored.campaign.game).unwrap()
        );
        app.settings.rewind_seconds = 5;
        for _ in 0..10 {
            app.game_tick(input::UP);
        }
        let expected = postcard::to_allocvec(&app.campaign.game).unwrap();
        let index = app.rewind.entries.len() - 1;
        for _ in 0..10 {
            app.game_tick(input::RIGHT);
        }
        app.rewind_to(index).unwrap();
        assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), expected);
        app.controls.clear();
        app.player_input();
        assert!(!app.campaign.game.walking.unwrap().held);
    }
}
