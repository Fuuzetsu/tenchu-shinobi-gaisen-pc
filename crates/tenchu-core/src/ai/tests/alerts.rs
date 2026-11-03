use super::*;

fn castle_guard(position: Position) -> crate::game::Game {
    let mut game = oracle::game(6, 0);
    game.roster.keep_all_active = true;
    game.teleport_player(Position::new(-32900, 12000, 8796), true)
        .unwrap();
    game.roster.get_mut(game.player_id).invulnerable = true;
    assert_eq!(game.roster.get(11).kind, 0x61);
    let patrol = game.roster.get(11).patrol.clone();
    let think = game.roster.get(11).think;
    game.move_actor(11, position, 2132, true).unwrap();
    let guard = game.roster.get_mut(11);
    guard.patrol = patrol;
    guard.think = think;
    guard.attribute |= ATTR_TRACE;
    for &id in &game.roster.order.clone() {
        if id != game.player_id && id != 11 {
            game.roster.get_mut(id).setup_think(0).unwrap();
        }
    }
    game.intelligence.alert = 600;
    game
}

#[test]
#[ignore = "requires TENCHU_DISC; Mission 9 draw motion crosses the all-active alarm radius"]
fn all_active_guard_finishes_one_alarm_across_the_range_boundary() {
    let mut game = castle_guard(Position::new(-24500, 12000, -9250));
    let mut draws = 0;
    let mut sheathes = 0;
    let mut crossed_boundary = false;
    for _ in 0..700 {
        let previous = game.roster.get(11).motion.action;
        game.step(0).unwrap();
        let guard = game.roster.get(11);
        if guard.motion.action != previous {
            draws += usize::from(guard.motion.action == MOT_STATE_DRAW);
            sheathes += usize::from(guard.motion.action == MOT_STATE_SHEATHE);
        }
        if game.intelligence.alert > 1 {
            crossed_boundary |= game
                .math
                .length_wide(guard.position.wrapping_sub(game.player().position))
                >= 20000;
        }
    }
    assert!(
        crossed_boundary,
        "The draw animation must leave the alarm radius"
    );
    assert_eq!(draws, 1, "One alarm must not repeatedly restart drawing");
    assert_eq!(sheathes, 1, "Stand down once, after the alarm expires");
    assert_eq!(game.roster.get(11).attribute & ATTR_PHASE, 0);
    assert!(!game.roster.get(11).heard_alert);
    assert_eq!(game.intelligence.alert, 0);
    assert_eq!(game.intelligence.found, 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; distant all-active patrols do not inherit a local alarm"]
fn distant_all_active_guard_keeps_patrolling_during_an_unheard_alarm() {
    let mut game = castle_guard(Position::new(-24500, 12000, -14000));
    for _ in 0..180 {
        game.step(0).unwrap();
        let guard = game.roster.get(11);
        assert_eq!(guard.attribute & ATTR_PHASE, 0);
        assert_eq!(guard.attribute & ATTR_WEAPON_DRAWN, 0);
        assert!(!guard.heard_alert);
        assert_ne!(guard.motion.action, MOT_STATE_DRAW);
    }
    assert!(game.intelligence.alert > 0);
}
