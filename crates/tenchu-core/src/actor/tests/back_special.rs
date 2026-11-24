use super::*;
use crate::input::{DOWN, Pad, SQUARE};

fn ready(character: i16) -> Game {
    let mut game = oracle::game(0, character);
    let enemy = arena(&mut game, 0x10);
    let player = game.player_id;
    with_actor(&mut game, player, |actor, ctx, world| {
        world.roster.remove(enemy, ctx);
        actor.equip(ctx, true);
        actor.enter(ctx, MOT_ENGAGE_STANCE, true);
        actor.motion.count = 5;
        actor.pad = Pad::default();
        actor.target = None;
    });
    game.events = Default::default();
    game
}

#[test]
#[ignore = "requires TENCHU_DISC; the optional shortcut and original Up–Down–Square command select identical authored attacks and continue identically"]
fn back_special_matches_original_command_and_authored_contacts() {
    for character in 0..2 {
        let mut original = ready(character);
        let mut shortcut = ready(character);
        let commands = original.commands.clone();
        let held = original
            .roster
            .get_mut(original.player_id)
            .pad
            .issue(&commands, 0x22);
        original.step(held).unwrap();
        shortcut.combat.back_special = true;
        shortcut.step(DOWN | SQUARE).unwrap();
        assert_eq!(original.player().motion.action, MOT_ATTACK_LUNGE_BACK);
        assert_eq!(shortcut.player().motion.action, MOT_ATTACK_LUNGE_BACK);
        original.roster.get_mut(original.player_id).pad = Pad::default();
        shortcut.roster.get_mut(shortcut.player_id).pad = Pad::default();
        shortcut.combat.back_special = false;
        let mut contacts = 0;
        assert_eq!(state(&shortcut), state(&original), "hero {character}");
        for _ in 0..90 {
            contacts += shortcut.collision.slots[..shortcut.collision.len]
                .iter()
                .filter(|slot| {
                    slot.actor() == Some(shortcut.player_id)
                        && slot.class & crate::collision::HIT != 0
                        && slot.model.is_some_and(|model| {
                            shortcut.scene.models[model].attribute & crate::scene::COLLIDE != 0
                        })
                })
                .count();
            shortcut.step(0).unwrap();
            original.step(0).unwrap();
        }
        assert!(contacts > 0);
        assert_eq!(
            state(&shortcut),
            state(&original),
            "continued hero {character}"
        );
        for enabled in [false, true] {
            let mut game = ready(character);
            game.combat.back_special = enabled;
            let has_back = game.mission.characters[game.player().definition]
                .motions
                .iter()
                .any(|r| r.action == MOT_ATTACK_BACK && r.clip >= 0);
            for _ in 0..5 {
                game.step(DOWN).unwrap();
            }
            assert_eq!(game.player().motion.action, MOT_CHASE_BACK);
            game.step(DOWN | SQUARE).unwrap();
            assert_eq!(
                game.player().motion.action,
                if enabled {
                    MOT_ATTACK_LUNGE_BACK
                } else if has_back {
                    MOT_ATTACK_BACK
                } else {
                    MOT_CHASE_BACK
                }
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; unsupported specials and NPC/crouch/guard/dash/air/script contexts keep their original attack selection"]
fn back_special_retains_original_selection_outside_supported_stance() {
    for scenario in 0..10 {
        let mut game = ready(0);
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, world| {
            match scenario {
                0 => ctx.mission.characters[actor.definition]
                    .motions
                    .retain(|r| r.action != MOT_ATTACK_LUNGE_BACK),
                1 => world.player = usize::MAX,
                2 => {
                    actor.enter(ctx, MOT_SQUAT, true);
                }
                3 => {
                    actor.enter(ctx, MOT_CHASE_DASH_FWD, true);
                }
                4 => {
                    world.intelligence.circle_action = crate::input::CircleAction::Guard;
                    actor.enter(ctx, MOT_CHASE_DASH_FWD, true);
                }
                5 => actor.map.height = 1,
                6 => actor.scripted = true,
                7 => world.intelligence.cinematic = true,
                8 => world.intelligence.action_halt = crate::ai::Halt::StageEnd,
                9 => actor.equip(ctx, false),
                _ => unreachable!(),
            }
            let before = actor.clone();
            let mut expected = None;
            for enabled in [false, true] {
                *actor = before.clone();
                world.combat.back_special = enabled;
                let selected = actor.attack_input(ctx, world, DOWN | SQUARE, None).unwrap();
                if enabled {
                    assert_eq!(selected, expected, "scenario {scenario}");
                }
                expected = selected;
            }
        });
    }
    for action in [MOT_ATTACK, MOT_JUMP, MOT_DAMAGE_FRONT_MID] {
        let mut games = [ready(0), ready(0)];
        for (enabled, game) in games.iter_mut().enumerate() {
            let player = game.player_id;
            with_actor(game, player, |actor, ctx, _| {
                assert!(actor.enter(ctx, action, true));
            });
            game.combat.back_special = enabled == 1;
            game.step(DOWN | SQUARE).unwrap();
        }
        assert_eq!(
            state(&games[0]),
            state(&games[1]),
            "committed action {action:x}"
        );
    }
}

fn state(game: &Game) -> serde_json::Value {
    serde_json::json!({
        "actor": game.player(),
        "scene": &game.scene,
        "collision": &game.collision,
        "effects": &game.effects,
        "random": game.random.seed,
    })
}
