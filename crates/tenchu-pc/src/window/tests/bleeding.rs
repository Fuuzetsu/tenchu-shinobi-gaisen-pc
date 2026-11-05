use super::*;
use tenchu_core::{game::bleeding::Settings as Bleeding, input};

#[test]
#[ignore = "requires TENCHU_DISC; disabled settings preserve exact gameplay and treatment survives save/rewind"]
fn bleeding_options_input_and_saved_continuation() {
    let mut app = application(Options::default());
    let mut original = application(Options::default());
    for a in [&mut app, &mut original] {
        a.campaign.game.events = Default::default();
    }
    app.set_bleeding(Bleeding {
        chance: 100,
        damage: 20,
        ..Default::default()
    });
    for tick in 0..120 {
        let buttons = if tick < 60 { input::UP } else { input::RIGHT };
        assert!(app.game_tick(buttons) && original.game_tick(buttons));
        assert_eq!(
            postcard::to_allocvec(&app.campaign.game).unwrap(),
            postcard::to_allocvec(&original.campaign.game).unwrap()
        );
    }
    let previous = app.settings.clone();
    app.apply_item(Item::Bleeding, 1).unwrap();
    app.reset_item(Item::BleedDamage).unwrap();
    app.apply_settings(&previous);
    assert!(app.campaign.game.player().bleeding.is_some());
    assert_eq!(app.settings.bleeding.damage, 1);
    assert!(!app.trial.eligible);
    app.settings.rewind_seconds = 5;
    let player = app
        .campaign
        .game
        .roster
        .get_mut(app.campaign.game.player_id);
    player.health = 80;
    let state = player.bleeding.as_mut().unwrap();
    state.active = true;
    state.remaining = 300;
    for _ in 0..17 {
        assert!(app.game_tick(0));
    }
    let saved = app.capture_state().unwrap();
    if let Some(path) = std::env::var_os("TENCHU_BLEED_REVIEW") {
        std::fs::write(path, &saved).unwrap();
    }
    let expected = postcard::to_allocvec(&app.campaign.game).unwrap();
    let rewind_tick = app.campaign.game.tick;
    let mut restored = application(Options::default());
    restored.restore_state(&saved).unwrap();
    assert_eq!(restored.settings.bleeding, app.settings.bleeding);
    for _ in 0..35 {
        assert!(app.game_tick(0) && restored.game_tick(0));
    }
    assert_eq!(
        postcard::to_allocvec(&app.campaign.game).unwrap(),
        postcard::to_allocvec(&restored.campaign.game).unwrap()
    );
    assert_eq!(app.campaign.game.player().health, 79);
    app.seek_rewind(rewind_tick).unwrap();
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), expected);
    app.set_paused(false);
    app.controls.key(KeyCode::KeyB, true);
    app.controls.key(KeyCode::KeyB, false);
    let buttons = app.player_input();
    assert_eq!(
        buttons, 0,
        "bandage binding leaked into original pad buttons"
    );
    assert!(app.game_tick(buttons));
    let state = app.campaign.game.player().bleeding.as_ref().unwrap();
    assert!(!state.active);
    assert_eq!(state.bandages, 2);
    assert_eq!(app.campaign.game.player().health, 80);
    assert!(
        app.rewind
            .entries
            .back()
            .unwrap()
            .input
            .as_ref()
            .unwrap()
            .bandage
    );
    let previous = app.settings.clone();
    app.apply_item(Item::BandageLoadout, 9).unwrap();
    app.apply_settings(&previous);
    assert_eq!(
        app.campaign
            .game
            .player()
            .bleeding
            .as_ref()
            .unwrap()
            .bandages,
        2
    );
    let previous = app.settings.clone();
    app.reset_item(Item::Bleeding).unwrap();
    app.apply_settings(&previous);
    assert!(app.campaign.game.player().bleeding.is_none());
    app.set_bleeding(Bleeding {
        enabled: true,
        ..Default::default()
    });
    app.set_gameplay_preset(0).unwrap();
    assert!(!app.settings.bleeding.enabled && app.campaign.game.player().bleeding.is_none());
}
