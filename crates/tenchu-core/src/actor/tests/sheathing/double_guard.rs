use super::*;

fn setup(hero: i16, circle: bool, enabled: bool) -> Game {
    let (mut game, _) = ready(
        hero,
        Settings {
            double_guard: enabled,
            ..Default::default()
        },
    );
    game.intelligence.circle_action = if circle {
        CircleAction::Guard
    } else {
        CircleAction::Crouch
    };
    game.intelligence.guard_controls = !circle;
    game
}

fn transition(game: &Game) -> Vec<u8> {
    let actor = game.player();
    serde_json::to_vec(&(
        &actor.motion,
        actor.status,
        actor.attribute,
        actor.position,
        actor.rotation,
        actor.velocity,
        &actor.weapons,
        &game.collision,
        &game.effects,
        &game.random,
    ))
    .unwrap()
}

#[test]
#[ignore = "requires TENCHU_DISC; both logical guard mappings toggle through identical authored draw/stow transitions without held repeats"]
fn double_guard_matches_explicit_sheathing_and_source_window() {
    for hero in 0..2 {
        for circle in [false, true] {
            for gap in [1, 4, 5] {
                let guard = if circle { CIRCLE } else { R1 };
                let mut shortcut = setup(hero, circle, true);
                let mut explicit = setup(hero, circle, false);
                assert!(
                    shortcut
                        .commands
                        .iter()
                        .all(|command| command.inputs.iter().all(|input| input & R1 == 0))
                );
                for armed in [true, false] {
                    for game in [&mut shortcut, &mut explicit] {
                        for _ in 0..8 {
                            game.step(0).unwrap();
                        }
                        game.step(guard).unwrap();
                        for _ in 0..gap {
                            game.step(0).unwrap();
                        }
                    }
                    if gap < 5 {
                        let player = explicit.player_id;
                        explicit
                            .roster
                            .get_mut(player)
                            .sheathing
                            .as_mut()
                            .unwrap()
                            .input(true);
                    }
                    shortcut.step(guard).unwrap();
                    explicit.step(guard).unwrap();
                    if gap < 5 {
                        assert_eq!(
                            shortcut.player().motion.action,
                            if armed {
                                MOT_STATE_DRAW
                            } else {
                                MOT_STATE_SHEATHE
                            }
                        );
                    } else {
                        assert!(!matches!(
                            shortcut.player().motion.action,
                            MOT_STATE_DRAW | MOT_STATE_SHEATHE
                        ));
                    }
                    for tick in 0..85 {
                        assert_eq!(
                            transition(&shortcut),
                            transition(&explicit),
                            "hero{hero}, circle{circle}, gap{gap}, armed{armed}, tick{tick}"
                        );
                        shortcut.step(guard).unwrap();
                        explicit.step(guard).unwrap();
                    }
                    if gap < 5 {
                        assert_eq!(shortcut.player().attribute & ATTR_WEAPON_DRAWN != 0, armed);
                    }
                    let player = explicit.player_id;
                    explicit
                        .roster
                        .get_mut(player)
                        .sheathing
                        .as_mut()
                        .unwrap()
                        .input(false);
                }
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; unsafe pairs are discarded and optional inputs preserve source directions, attacks and default behavior"]
fn double_guard_discards_unsafe_pairs_and_preserves_other_inputs() {
    for hero in 0..2 {
        let mut game = setup(hero, true, true);
        let player = game.player_id;
        let commands = game.commands.clone();
        for condition in 0..13 {
            with_actor(&mut game, player, |actor, ctx, world| {
                actor.health = 100;
                actor.scripted = false;
                actor.map.height = 0;
                actor.enter(ctx, MOT_NORMAL, true);
                world.intelligence.cinematic = false;
                world.intelligence.action_halt = crate::ai::Halt::Running;
                match condition {
                    0 => {
                        actor.enter(ctx, MOT_ATTACK, true);
                    }
                    1 => {
                        actor.enter(ctx, MOT_JUMP, true);
                    }
                    2 => {
                        actor.enter(ctx, MOT_DAMAGE_FRONT_MID, true);
                    }
                    3 => actor.map.height = 1,
                    4 => actor.health = 0,
                    5 => actor.scripted = true,
                    6 => world.intelligence.cinematic = true,
                    7 => world.intelligence.action_halt = crate::ai::Halt::Active,
                    _ => {
                        let action = [
                            MOT_SQUAT,
                            MOT_SQUAT_WALK_F,
                            MOT_SQUAT_WALK_B,
                            MOT_SQUAT_WALK_L,
                            MOT_SQUAT_WALK_R,
                        ][condition - 8];
                        actor.enter(ctx, action, true);
                        assert_eq!(actor.status, ActorState::Crouch);
                    }
                }
                actor.pad = Default::default();
                for input in [R1, 0, R1] {
                    actor.pad.update(input);
                }
                actor
                    .control(ctx, &commands, actor.position, world)
                    .unwrap();
                assert!(!matches!(
                    actor.motion.action,
                    MOT_STATE_DRAW | MOT_STATE_SHEATHE
                ));
                assert_eq!(actor.pad.stream[0], 0);
                actor.enter(ctx, MOT_NORMAL, true);
                actor.map.height = 0;
                actor.health = 100;
                actor.scripted = false;
                world.intelligence.cinematic = false;
                world.intelligence.action_halt = crate::ai::Halt::Running;
                actor
                    .control(ctx, &commands, actor.position, world)
                    .unwrap();
                assert!(
                    !matches!(actor.motion.action, MOT_STATE_DRAW | MOT_STATE_SHEATHE),
                    "unsafe pair queued for condition{condition}"
                );
            });
        }
        for circle in [false, true] {
            let guard = if circle { CIRCLE } else { R1 };
            let crouch = if circle { R1 } else { CIRCLE };
            for extra in [UP, DOWN, LEFT, RIGHT, crouch | LEFT, SQUARE, CROSS] {
                let mut on = setup(hero, circle, true);
                let mut off = setup(hero, circle, false);
                for buttons in [guard, guard | extra, guard, 0, guard] {
                    on.step(buttons).unwrap();
                    off.step(buttons).unwrap();
                    assert_eq!(transition(&on), transition(&off));
                }
                for _ in 0..80 {
                    on.step(guard | extra).unwrap();
                    off.step(guard | extra).unwrap();
                    assert_eq!(transition(&on), transition(&off));
                }
            }
        }
        let mut imported = setup(hero, true, true);
        imported.intelligence.circle_action = CircleAction::Moveset;
        imported
            .roster
            .get_mut(imported.player_id)
            .imported_circle_guard = true;
        for buttons in [CIRCLE, 0, CIRCLE] {
            imported.step(buttons).unwrap();
        }
        assert_eq!(imported.player().motion.action, MOT_STATE_DRAW);
        let mut auto_draw = setup(hero, false, true);
        let mut auto_draw_off = setup(hero, false, false);
        for game in [&mut auto_draw, &mut auto_draw_off] {
            game.intelligence.guard_draw = true;
        }
        for buttons in [R1, 0, R1] {
            auto_draw.step(buttons).unwrap();
            auto_draw_off.step(buttons).unwrap();
            assert_eq!(transition(&auto_draw), transition(&auto_draw_off));
        }
        assert_eq!(auto_draw.player().motion.action, MOT_STATE_DRAW);
        let mut original = setup(hero, false, true);
        original.intelligence.guard_controls = false;
        for buttons in [R1, 0, R1] {
            original.step(buttons).unwrap();
        }
        assert_eq!(original.player().motion.action, MOT_SQUAT);
        let mut disabled = setup(hero, true, true);
        disabled
            .set_player_sheathing(Settings::default(), None)
            .unwrap();
        for buttons in [CIRCLE, 0, CIRCLE] {
            disabled.step(buttons).unwrap();
        }
        assert_eq!(disabled.player().motion.action, MOT_ENGAGE_GUARD);
        assert!(disabled.player().sheathing.is_none());
    }
}
