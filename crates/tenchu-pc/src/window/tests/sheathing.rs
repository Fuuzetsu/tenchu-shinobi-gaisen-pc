use super::*;
use tenchu_core::{game::sheathing::Settings as Sheathing, ids::*, input};

fn ready(character: i16) -> Application {
    let mut app = application(Options {
        character,
        ..Default::default()
    });
    app.campaign.game.events = Default::default();
    app.set_sheathing(Sheathing {
        enabled: true,
        ..Default::default()
    })
    .unwrap();
    for _ in 0..20 {
        app.campaign.game.step(0).unwrap();
    }
    app
}

fn press_sheathe(app: &mut Application) {
    app.controls.key(KeyCode::KeyH, true);
    for _ in 0..80 {
        let buttons = app.player_input();
        assert_eq!(buttons, 0);
        assert!(app.game_tick(buttons));
    }
    app.controls.key(KeyCode::KeyH, false);
    app.player_input();
}

#[test]
#[ignore = "requires TENCHU_DISC; real draw/sheath cycles, original locomotion and single-toggle input for both heroes"]
fn sheathing_controls_and_ground_attacks() {
    for character in 0..2 {
        let mut app = ready(character);
        let armed = app
            .campaign
            .game
            .player()
            .weapons
            .each_ref()
            .map(|w| w.as_ref().map(|w| w.model));
        press_sheathe(&mut app);
        assert_eq!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        assert_eq!(app.campaign.game.player().motion.action, MOT_NORMAL);
        assert_ne!(
            app.campaign
                .game
                .player()
                .weapons
                .each_ref()
                .map(|w| w.as_ref().map(|w| w.model)),
            armed
        );
        for _ in 0..20 {
            app.campaign.game.step(input::UP).unwrap();
        }
        assert_eq!(app.campaign.game.player().motion.action, MOT_CHASE);
        let base = app.campaign.game.player().velocity;
        app.set_sheathing(Sheathing {
            speed: 150,
            ..app.settings.sheathing
        })
        .unwrap();
        app.campaign.game.step(input::UP).unwrap();
        assert!(i32::from(app.campaign.game.player().velocity[0]).abs() > i32::from(base[0]).abs());
        app.set_walking(true).unwrap();
        app.campaign.game.walking.as_mut().unwrap().held = true;
        app.campaign.game.step(input::UP).unwrap();
        assert_eq!(
            app.campaign.game.player().motion.clip,
            Some(app.campaign.game.walking.unwrap().clip)
        );
        let slow = app.campaign.game.player().velocity;
        app.set_walking(false).unwrap();
        assert_ne!(app.campaign.game.player().velocity, slow);
        app.campaign.game.step(input::SQUARE).unwrap();
        assert_eq!(app.campaign.game.player().motion.action, MOT_STATE_DRAW);
        for _ in 0..30 {
            app.campaign.game.step(0).unwrap();
        }
        assert_ne!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        assert_eq!(
            app.campaign
                .game
                .player()
                .weapons
                .each_ref()
                .map(|w| w.as_ref().map(|w| w.model)),
            armed
        );
        app.campaign.game.step(input::SQUARE).unwrap();
        assert_eq!(app.campaign.game.player().motion.action, MOT_ATTACK);
        // A tap during a committed attack is discarded, not queued for recovery.
        app.controls.key(KeyCode::KeyH, true);
        let buttons = app.player_input();
        app.game_tick(buttons);
        assert_eq!(app.campaign.game.player().motion.action, MOT_ATTACK);
        for _ in 0..90 {
            let buttons = app.player_input();
            app.game_tick(buttons);
        }
        assert_ne!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        app.controls.clear();
        app.player_input();
        press_sheathe(&mut app);
        assert_eq!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        app.set_sheathing(Default::default()).unwrap();
        assert_ne!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        assert!(app.campaign.game.player().sheathing.is_none());
        for _ in 0..30 {
            app.campaign.game.step(input::UP).unwrap();
        }
        assert_eq!(app.campaign.game.player().motion.action, MOT_CHASE);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; optional sheathing leaves armed gameplay unchanged and savestates retain a live sheathed stance"]
fn sheathing_armed_parity_and_saved_state() {
    for character in 0..2 {
        let mut on = ready(character);
        let mut off = ready(character);
        off.set_sheathing(Default::default()).unwrap();
        for tick in 0..180 {
            let buttons = match tick % 90 {
                0..20 => input::UP,
                20..30 => input::DOWN,
                30..45 => input::CIRCLE,
                45..60 => input::SQUARE,
                _ => 0,
            };
            on.campaign.game.step(buttons).unwrap();
            off.campaign.game.step(buttons).unwrap();
            if tick % 30 == 0 {
                let state = on
                    .campaign
                    .game
                    .roster
                    .get_mut(on.campaign.game.player_id)
                    .sheathing
                    .take();
                assert!(
                    postcard::to_allocvec(&on.campaign.game).unwrap()
                        == postcard::to_allocvec(&off.campaign.game).unwrap(),
                    "hero {character}, tick {tick}"
                );
                on.campaign
                    .game
                    .roster
                    .get_mut(on.campaign.game.player_id)
                    .sheathing = state;
            }
        }
        for _ in 0..90 {
            on.campaign.game.step(0).unwrap();
        }
        press_sheathe(&mut on);
        assert_eq!(on.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        let bytes = on.capture_state().unwrap();
        let mut restored = application(Options {
            character,
            ..Default::default()
        });
        restored.restore_state(&bytes).unwrap();
        assert!(restored.settings.sheathing.enabled);
        for tick in 0..150 {
            let buttons = if tick < 60 {
                input::UP
            } else if tick == 80 {
                input::SQUARE
            } else {
                0
            };
            on.campaign.game.step(buttons).unwrap();
            restored.campaign.game.step(buttons).unwrap();
        }
        assert!(
            postcard::to_allocvec(&on.campaign.game).unwrap()
                == postcard::to_allocvec(&restored.campaign.game).unwrap()
        );
        on.settings.rewind_seconds = 5;
        for _ in 0..10 {
            on.game_tick(0);
        }
        let expected = postcard::to_allocvec(&on.campaign.game).unwrap();
        let index = on.rewind.entries.len() - 1;
        press_sheathe(&mut on);
        on.rewind_to(index).unwrap();
        assert!(postcard::to_allocvec(&on.campaign.game).unwrap() == expected);
    }
}
