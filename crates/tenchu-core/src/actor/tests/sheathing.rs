use super::*;
use crate::game::sheathing::{Motions, Settings};
use crate::input::*;

fn ready(character: i16, settings: Settings) -> (Game, usize) {
    let mut game = oracle::game(0, character);
    let enemy = arena(&mut game, 0x10);
    let source = oracle::game(1, character);
    let ninja = source
        .mission
        .characters
        .iter()
        .find(|c| c.kind == if character == 0 { 0x20 } else { 0x22 })
        .unwrap();
    let clip = |action| {
        let id = ninja
            .motions
            .iter()
            .find(|m| m.action == action)
            .unwrap()
            .clip;
        source
            .mission
            .clips
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .clone()
    };
    game.set_player_sheathing(
        Settings {
            enabled: true,
            ..settings
        },
        Some(&Motions {
            idle: clip(MOT_NORMAL),
            turn: clip(MOT_NORMAL_TURN_L),
            draw: clip(MOT_STATE_DRAW),
            sheathe: clip(MOT_STATE_SHEATHE),
        }),
    )
    .unwrap();
    let player = game.player_id;
    with_actor(&mut game, player, |a, ctx, _| {
        a.equip(ctx, false);
        a.motion.action = -1;
        a.enter(ctx, MOT_NORMAL, true);
    });
    game.roster.get_mut(enemy).setup_think(0).unwrap();
    (game, enemy)
}

#[test]
#[ignore = "requires TENCHU_DISC; all six paired stealth kills draw the original hero's blades and finish correctly"]
fn sheathing_stealth_kills_equip_before_the_paired_animation() {
    for character in 0..2 {
        for variant in 0..3 {
            let (mut game, enemy) = ready(character, Settings::default());
            with_actor(&mut game, enemy, |a, ctx, _| {
                a.position = if variant == 2 {
                    Position::new(-700, 0, 0)
                } else {
                    Position::new(0, 0, 700)
                };
                a.rotation[1] = if variant == 1 { 0 } else { 2048 };
                a.attribute = 0;
                a.inventory.fill(0);
                a.enter(ctx, MOT_NORMAL, true);
                ctx.scene.update(a.root_model, a.root(ctx.math));
            });
            game.step(SQUARE).unwrap();
            let pair = character * 3 + variant;
            assert_eq!(game.player().motion.action, MOT_ATTACK_STEALTH_BACK + pair);
            assert_eq!(
                game.roster.get(enemy).motion.action,
                MOT_DEAD_STEALTH_BACK + pair
            );
            assert_ne!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
            assert_eq!(game.roster.get(enemy).health, 0);
            assert!(game.player().weapons[0].is_some());
            for _ in 0..300 {
                game.step(0).unwrap();
            }
            assert_eq!(game.player().status, ActorState::Engage);
            assert_eq!(game.statistics.criticals, 1);
            assert_eq!(game.intelligence.action_halt, crate::ai::Halt::Running);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; real incoming melee damage respects sheathed vulnerability and god mode"]
fn sheathing_damage_tradeoffs_apply_only_while_stowed() {
    let run = |character, damage, one_hit, armed, invulnerable| {
        let (mut game, enemy) = ready(
            character,
            Settings {
                damage,
                one_hit,
                ..Default::default()
            },
        );
        let player = game.player_id;
        with_actor(&mut game, player, |a, ctx, _| {
            a.invulnerable = invulnerable;
            if armed {
                a.equip(ctx, true);
                a.enter(ctx, MOT_ENGAGE_STANCE, true);
            }
        });
        game.roster.get_mut(enemy).setup_think(0x4444).unwrap();
        let initial = game.player().health;
        for tick in 0..600 {
            game.step(0).unwrap();
            if game.player().health != initial {
                return (tick, initial - game.player().health);
            }
        }
        (600, 0)
    };
    for character in 0..2 {
        let normal = run(character, 100, false, false, false);
        assert!(normal.1 > 0 && normal.1 < 100);
        assert_eq!(
            run(character, 200, false, false, false),
            (normal.0, normal.1 * 2)
        );
        assert_eq!(run(character, 100, true, false, false), (normal.0, 100));
        assert_eq!(run(character, 200, true, false, true).1, 0);
        assert_eq!(
            run(character, 400, true, true, false),
            run(character, 100, false, true, false)
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; reduced sight range affects a sheathed player, with ordinary armed detection restored"]
fn sheathing_visibility_tradeoff_uses_real_guard_detection() {
    let run = |visibility, armed| {
        let (mut game, enemy) = ready(
            0,
            Settings {
                visibility,
                ..Default::default()
            },
        );
        let player = game.player_id;
        if armed {
            with_actor(&mut game, player, |a, ctx, _| {
                a.equip(ctx, true);
                a.enter(ctx, MOT_ENGAGE_STANCE, true);
            });
        }
        with_actor(&mut game, enemy, |a, ctx, _| {
            a.position.z = 9000;
            a.setup_think(0x4444).unwrap();
            ctx.scene.update(a.root_model, a.root(ctx.math));
        });
        for _ in 0..64 {
            game.step(0).unwrap();
        }
        game.intelligence.found
    };
    assert!(run(100, false) > 0);
    assert_eq!(run(25, false), 0);
    assert!(run(25, true) > 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; optional sheath requests cannot cancel damage, jumps, attacks or scripted motion"]
fn sheathing_respects_committed_actions_and_cinematics() {
    for character in 0..2 {
        for action in [MOT_ATTACK, MOT_JUMP, MOT_DAMAGE_FRONT_MID] {
            let (mut game, _) = ready(character, Settings::default());
            let player = game.player_id;
            with_actor(&mut game, player, |a, ctx, _| {
                a.enter(ctx, action, true);
                a.sheathing.as_mut().unwrap().input(true);
            });
            game.step(0).unwrap();
            assert_eq!(game.player().motion.action, action);
        }
        let (mut game, _) = ready(character, Settings::default());
        let player = game.player_id;
        with_actor(&mut game, player, |a, ctx, _| {
            a.enter_cinematic(ctx, MOT_ATTACK);
        });
        assert_ne!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        assert_eq!(game.player().motion.action, MOT_ATTACK);
        game.roster.get_mut(player).scripted = true;
        game.roster
            .get_mut(player)
            .sheathing
            .as_mut()
            .unwrap()
            .input(true);
        game.step(0).unwrap();
        assert_eq!(game.player().motion.action, MOT_ATTACK);
        game.set_player_sheathing(Default::default(), None).unwrap();
        assert_eq!(game.player().motion.action, MOT_ATTACK);
    }
}
