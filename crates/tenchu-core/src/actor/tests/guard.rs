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
        for behind in [false, true] {
            let (mut game, enemy) = ready(character);
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
                game.step(R1).unwrap();
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
            if behind {
                assert!(hit && !blocked);
            } else {
                assert!(blocked && !hit && recoiled);
                let before = game.player().motion.action;
                game.step(R1 | crate::input::RIGHT).unwrap();
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
