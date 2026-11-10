use super::*;

fn ready(character: i16) -> (Game, usize) {
    let mut game = oracle::game(0, character);
    let enemy = arena(&mut game, 0x10);
    game.intelligence.guard_controls = true;
    game.roster.get_mut(enemy).setup_think(0).unwrap();
    for _ in 0..20 {
        game.step(crate::input::R1).unwrap();
    }
    (game, enemy)
}

#[test]
#[ignore = "requires TENCHU_DISC; both heroes dash/roll once per press, retain facing and stop while held"]
fn guard_controls_dash_and_roll_without_repeating() {
    use crate::input::*;
    for character in 0..2 {
        for crouch in [false, true] {
            for (direction, dash, roll) in [
                (UP, MOT_CHASE_DASH_FWD, MOT_SQUAT_ROLL_F),
                (DOWN, MOT_CHASE_DASH_BACK, MOT_SQUAT_ROLL_B),
                (LEFT, MOT_CHASE_DASH_LEFT, MOT_SQUAT_ROLL_L),
                (RIGHT, MOT_CHASE_DASH_RIGHT, MOT_SQUAT_ROLL_R),
            ] {
                let (mut game, _) = ready(character);
                let held = R1 | if crouch { CIRCLE } else { 0 };
                for _ in 0..20 {
                    game.step(held).unwrap();
                }
                let origin = game.player().position;
                let facing = game.player().rotation[1];
                let action = if crouch { roll } else { dash };
                game.step(held | direction).unwrap();
                assert_eq!(
                    game.player().motion.action,
                    action,
                    "character {character}, crouch {crouch}, direction {direction:x}"
                );
                let mut starts = 1;
                for _ in 0..100 {
                    let previous = game.player().motion.action;
                    game.step(held | direction).unwrap();
                    starts +=
                        usize::from(previous != action && game.player().motion.action == action);
                }
                assert_eq!(starts, 1);
                assert_ne!(game.player().position, origin);
                assert_eq!(game.player().rotation[1], facing);
                let stopped = game.player().position;
                for _ in 0..30 {
                    game.step(held | direction).unwrap();
                }
                assert_eq!(game.player().position, stopped);
                game.step(held).unwrap();
                game.step(held | direction).unwrap();
                assert_eq!(game.player().motion.action, action);
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; optional standing guard uses real frontal block/recoil and cannot protect from behind"]
fn guard_controls_block_melee_with_original_recoil() {
    use crate::effects::RumbleAction;
    use crate::input::R1;
    for character in 0..2 {
        for (behind, circle) in [(false, false), (true, false), (false, true), (true, true)] {
            let (mut game, enemy) = ready(character);
            let held = if circle {
                game.intelligence.guard_controls = false;
                game.intelligence.circle_action = crate::input::CircleAction::Guard;
                crate::input::CIRCLE
            } else {
                R1
            };
            game.step(held).unwrap();
            game.set_bleeding(crate::game::bleeding::Settings {
                enabled: true,
                chance: 100,
                ..Default::default()
            });
            if behind {
                game.roster.get_mut(game.player_id).rotation[1] = 0;
            }
            game.roster.get_mut(enemy).setup_think(0x4444).unwrap();
            let mut blocked = false;
            let mut recoiled = false;
            let original = game.player().position;
            let mut health = game.player().health;
            let mut hit = false;
            for _ in 0..500 {
                game.step(held).unwrap();
                blocked |= game
                    .effects
                    .rumble
                    .iter()
                    .any(|r| r.actor == game.player_id && r.action == RumbleAction::Block);
                recoiled |=
                    game.player().motion.action == MOT_ENGAGE && game.player().position != original;
                hit |= game.player().health < health;
                health = game.player().health;
                if blocked || hit {
                    break;
                }
            }
            assert_eq!(game.player().bleeding.as_ref().unwrap().active, hit);
            if behind {
                assert!(hit && !blocked);
            } else {
                assert!(blocked && !hit && recoiled);
                let before = game.player().motion.action;
                game.step(held | crate::input::RIGHT).unwrap();
                assert_eq!(before, MOT_ENGAGE);
                assert_eq!(
                    game.player().motion.action,
                    before,
                    "Dodge must not cancel block recoil"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; guarding cannot cancel attacks, jumping or injury, and disabling it restores original R1 crouch"]
fn guard_controls_respect_uninterruptible_actions() {
    use crate::input::*;
    for character in 0..2 {
        for action in [MOT_ATTACK, MOT_JUMP, MOT_DAMAGE_FRONT_MID] {
            let (mut game, _) = ready(character);
            let id = game.player_id;
            with_actor(&mut game, id, |actor, ctx, _| {
                actor.enter(ctx, action, true);
                actor.pad.update(0);
            });
            game.step(R1 | RIGHT).unwrap();
            assert_eq!(game.player().motion.action, action);
        }
        let (mut game, _) = ready(character);
        game.intelligence.guard_controls = false;
        for _ in 0..30 {
            game.step(R1).unwrap();
        }
        assert_eq!(game.player().motion.action, MOT_SQUAT);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; Circle guard works with both original heroes, repeats steps, preserves facing and releases to crouch or standing"]
fn circle_guard_with_original_movesets_repeats_steps_and_preserves_committed_actions() {
    use crate::input::*;
    for character in 0..2 {
        let setup = || {
            let (mut game, enemy) = ready(character);
            let player = game.player_id;
            with_actor(&mut game, player, |_, ctx, world| {
                world.roster.remove(enemy, ctx)
            });
            game.intelligence.guard_controls = false;
            game.intelligence.circle_action = CircleAction::Guard;
            game
        };
        let mut game = setup();
        for _ in 0..5 {
            game.step(CIRCLE).unwrap();
        }
        assert_eq!(game.player().motion.action, MOT_ENGAGE_GUARD);

        for crouch in [false, true] {
            for (direction, dash, roll) in [
                (UP, MOT_CHASE_DASH_FWD, MOT_SQUAT_ROLL_F),
                (DOWN, MOT_CHASE_DASH_BACK, MOT_SQUAT_ROLL_B),
                (RIGHT, MOT_CHASE_DASH_RIGHT, MOT_SQUAT_ROLL_R),
                (LEFT, MOT_CHASE_DASH_LEFT, MOT_SQUAT_ROLL_L),
            ] {
                let mut game = setup();
                let held = CIRCLE | if crouch { R1 } else { 0 };
                for _ in 0..5 {
                    game.step(held).unwrap();
                }
                let facing = game.player().rotation[1];
                let start = game.player().position;
                let action = if crouch { roll } else { dash };
                let mut starts = 0;
                for tick in 0..160 {
                    let previous = game.player().motion.action;
                    game.step(held | direction).unwrap();
                    starts +=
                        usize::from(previous != action && game.player().motion.action == action);
                    assert_eq!(
                        game.player().rotation[1],
                        facing,
                        "hero {character}, crouch {crouch}, tick {tick}, expected {action:x}, previous {previous:x}, actual {:x}, height {}, pad {:x}",
                        game.player().motion.action,
                        game.player().map.height,
                        game.player().pad.held
                    );
                }
                assert!(
                    starts >= 2,
                    "hero {character}, action {action:x}, starts {starts}"
                );
                assert_ne!(game.player().position, start);
                assert_eq!(game.player().rotation[1], facing);
                for _ in 0..100 {
                    game.step(if crouch { R1 } else { 0 }).unwrap();
                }
                assert_eq!(
                    game.player().motion.action,
                    if crouch { MOT_SQUAT } else { MOT_ENGAGE_STANCE }
                );
            }
        }
        for action in [MOT_ATTACK, MOT_JUMP, MOT_DAMAGE_FRONT_MID] {
            let mut game = setup();
            let id = game.player_id;
            with_actor(&mut game, id, |actor, ctx, _| {
                actor.enter(ctx, action, true);
                actor.pad.update(0);
            });
            game.step(CIRCLE | RIGHT).unwrap();
            assert_eq!(game.player().motion.action, action);
        }
        for (dash, direction, button, expected) in [
            (MOT_CHASE_DASH_FWD, UP, SQUARE, MOT_ATTACK_LUNGE),
            (MOT_CHASE_DASH_BACK, DOWN, SQUARE, MOT_ATTACK_LUNGE_BACK),
            (MOT_CHASE_DASH_FWD, UP, CROSS, MOT_JUMP_RUN),
            (MOT_CHASE_DASH_BACK, DOWN, CROSS, MOT_JUMP_BACK),
        ] {
            let mut game = setup();
            let id = game.player_id;
            with_actor(&mut game, id, |actor, ctx, _| {
                actor.enter(ctx, dash, true);
                actor.motion.count = 12;
                actor.pad.update(0);
            });
            game.step(CIRCLE | direction | button).unwrap();
            assert_eq!(game.player().motion.action, expected);
        }
        game.intelligence.circle_action = CircleAction::Crouch;
        for _ in 0..20 {
            game.step(CIRCLE).unwrap();
        }
        assert_eq!(game.player().motion.action, MOT_SQUAT);
    }
}
