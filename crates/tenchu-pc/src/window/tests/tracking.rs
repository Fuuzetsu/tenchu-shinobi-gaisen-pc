use super::*;
use tenchu_core::{ids::*, input};

#[test]
#[ignore = "requires TENCHU_DISC; mid-attack saves and rewind restore the assist and its selected actor"]
fn attack_assist_save_and_rewind() {
    let mut app = application(Options::default());
    app.campaign.game.events = Default::default();
    app.apply_item(Item::AttackAssist, 1).unwrap();
    for _ in 0..60 {
        app.game_tick(0);
    }
    let game = &mut app.campaign.game;
    let target = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| game.roster.get(id).kind == 0x10)
        .unwrap();
    let yaw = game.player().rotation[1];
    let at = game
        .player()
        .position
        .wrapping_add(game.math.velocity(yaw, 1800, 300).map(i32::from).into());
    game.move_actor(target, at, yaw, true).unwrap();
    game.roster.get_mut(target).attribute |= ATTR_WEAPON_DRAWN;
    game.roster.get_mut(target).setup_think(0).unwrap();
    app.game_tick(input::SQUARE);
    app.game_tick(0);
    assert_eq!(app.campaign.game.player().motion.action, MOT_ATTACK);
    assert!(app.campaign.game.player().target.is_some());
    let saved = app.capture_state().unwrap();
    let mut restored = application(Options::default());
    restored.restore_state(&saved).unwrap();
    assert!(restored.settings.attack_assist && restored.campaign.game.combat.attack_assist);
    for _ in 0..90 {
        app.game_tick(0);
        restored.game_tick(0);
    }
    assert!(
        postcard::to_allocvec(&app.campaign.game).unwrap()
            == postcard::to_allocvec(&restored.campaign.game).unwrap()
    );
    app.settings.rewind_seconds = 5;
    app.game_tick(input::SQUARE);
    for _ in 0..3 {
        app.game_tick(0);
    }
    let expected = postcard::to_allocvec(&app.campaign.game).unwrap();
    let index = app.rewind.entries.len() - 1;
    for _ in 0..35 {
        app.game_tick(0);
    }
    app.rewind_to(index).unwrap();
    assert!(postcard::to_allocvec(&app.campaign.game).unwrap() == expected);
}
