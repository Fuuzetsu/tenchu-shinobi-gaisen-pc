use super::*;
use crate::game::dismemberment::Settings;

#[test]
#[ignore = "requires TENCHU_DISC; real stealth kills, detached poses, landing and disabled RNG parity"]
fn sa_rear_kills_detach_at_authored_frames_and_keep_the_part_after_landing() {
    for character in 0..2 {
        for enabled in [false, true] {
            let mut game = prepared(character);
            let victim = *game.roster.order.last().unwrap();
            game.combat.dismemberment = Settings {
                enabled,
                stealth_head: 100,
                ..Default::default()
            };
            let mut baseline = prepared(character);
            baseline.combat.dismemberment = Settings::default();
            game.step(crate::input::SQUARE).unwrap();
            baseline.step(crate::input::SQUARE).unwrap();
            assert_eq!(
                game.roster.get(victim).motion.action,
                MOT_DEAD_STEALTH_BACK + character * 3
            );
            let expected_frame = if character == 0 { 68 } else { 36 };
            let models = game.scene.models.len();
            let mut detached_at = None;
            for _ in 0..230 {
                game.step(0).unwrap();
                baseline.step(0).unwrap();
                let actor = game.roster.get(victim);
                if let Some(part) = &actor.severed {
                    detached_at.get_or_insert(actor.motion.count);
                    assert_eq!(part.index, 2);
                    assert_eq!(game.scene.models[actor.parts[2]].parent, Some(0));
                    assert_eq!(actor.motion.mask & ((1 << 2) | 0x6000), 0);
                    let mut inspection = actor.clone();
                    inspection.initialize_rendered(&game.scene, &game.math);
                    assert_eq!(
                        inspection.rendered[2],
                        game.scene.models[actor.parts[2]].local
                    );
                }
                if !enabled {
                    assert_eq!(game.random.seed, baseline.random.seed);
                    assert_eq!(game.player().position, baseline.player().position);
                    assert_eq!(
                        game.roster.get(victim).position,
                        baseline.roster.get(victim).position
                    );
                }
            }
            assert_eq!(detached_at, enabled.then_some(expected_frame + 1));
            assert_eq!(game.scene.models.len(), models);
            if enabled {
                let dead = game.roster.get(victim);
                assert!(!dead.severed.as_ref().unwrap().moving);
                assert!((-19..=0).contains(&game.scene.models[dead.parts[2]].local.translation.y));
                with_actor(&mut game, victim, |actor, ctx, _| {
                    let parent = actor.severed.as_ref().unwrap().parent;
                    actor.restore_severed(ctx.scene);
                    actor.enter(ctx, MOT_NORMAL, true);
                    actor.play_pose(ctx).unwrap();
                    assert_eq!(ctx.scene.models[actor.parts[2]].parent, parent);
                });
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; SA head-before-arm selection, independent sliders and exclusions"]
fn sa_ordinary_kill_selection_and_exclusions() {
    for (action, head, arms, side, expected) in [
        (MOT_ATTACK_DIVE, 100, 100, 100, Some(2)),
        (MOT_ATTACK_DIVE_LAND, 0, 100, 100, Some(4)),
        (MOT_ATTACK, 100, 100, 0, Some(6)),
        (MOT_ATTACK, 100, 0, 100, None),
    ] {
        for kind in [0x10, 0x87, 0xa0] {
            let mut game = oracle::game(0, 0);
            let victim = arena(&mut game, 0x10);
            game.combat.dismemberment = Settings {
                enabled: true,
                dive_head: head,
                arms,
                right_arm: side,
                ..Default::default()
            };
            game.roster.get_mut(game.player_id).motion.action = action;
            game.combat.death_sound = Some(victim);
            with_actor(&mut game, victim, |actor, ctx, world| {
                actor.kind = kind;
                actor.health = 0;
                actor.enter(ctx, MOT_DEAD, true);
                actor.play_pose(ctx).unwrap();
                actor.motion.count = 5;
                actor.dead(ctx, world).unwrap();
                assert_eq!(
                    actor.severed.as_ref().map(|p| p.index),
                    if kind == 0x10 { expected } else { None }
                );
            });
        }
    }
}

fn prepared(character: i16) -> Game {
    let mut game = oracle::game(0, character);
    let victim = arena(&mut game, 0x10);
    let player = game.player_id;
    with_actor(&mut game, player, |actor, ctx, _| {
        actor.rotation = [0; 3];
        actor.enter(ctx, MOT_ENGAGE_STANCE, true);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
    });
    with_actor(&mut game, victim, |actor, ctx, _| {
        actor.position = Position::new(0, 0, -700);
        actor.rotation = [0; 3];
        actor.attribute = 0;
        actor.inventory.fill(0);
        actor.enter(ctx, MOT_NORMAL, true);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
    });
    game
}
