use super::*;
use crate::input::SQUARE;

fn place(game: &mut Game, id: usize, position: Position) {
    with_actor(game, id, |a, ctx, _| {
        a.position = position;
        a.rotation = [0, 2048, 0];
        a.attribute = ATTR_WEAPON_DRAWN;
        a.health = 9999;
        a.max_health = 9999;
        a.setup_think(0).unwrap();
        a.equip(ctx, true);
        a.enter(ctx, MOT_ENGAGE_STANCE, true);
        ctx.scene.update(a.root_model, a.root(ctx.math));
    });
}

fn ready(character: i16) -> (Game, usize, usize) {
    let mut game = oracle::game(0, character);
    let front = arena(&mut game, 0x10);
    let player = game.player_id;
    let rear = with_actor(&mut game, player, |_, ctx, world| {
        world.roster.spawn(0x10, Position::ZERO, 0, ctx).unwrap()
    });
    with_actor(&mut game, player, |a, ctx, _| {
        a.enter(ctx, MOT_ENGAGE_STANCE, true);
    });
    place(&mut game, front, Position::new(800, 0, 1800));
    place(&mut game, rear, Position::new(0, 0, -1000));
    (game, front, rear)
}

#[test]
#[ignore = "requires TENCHU_DISC; forward targeting, smooth wind-up, no added hit frames and no target switches mid-swing"]
fn attack_assist_prefers_front_and_only_turns_during_windup() {
    for character in 0..2 {
        for enabled in [false, true] {
            let (mut game, front, rear) = ready(character);
            game.combat.attack_assist = enabled;
            game.step(SQUARE).unwrap();
            assert_eq!(game.player().motion.action, MOT_ATTACK);
            assert_eq!(
                game.player().target,
                Some(
                    game.roster
                        .get(if enabled { front } else { rear })
                        .root_model
                )
            );
            let player = game.player_id;
            let mut turned = false;
            for _ in 0..45 {
                let before = game.player().rotation[1];
                let count = game.player().motion.count;
                game.step(0).unwrap();
                let a = game.player();
                if a.motion.action != MOT_ATTACK {
                    break;
                }
                let battle = &game.mission.weapons.attacks[a.attack];
                let delta = a.rotation[1].wrapping_sub(before);
                turned |= delta != 0;
                if enabled {
                    assert!(delta.abs() <= 136);
                    if count >= battle.steering.min(battle.start) {
                        assert_eq!(delta, 0, "active/recovery frames must not steer");
                    }
                    for part in [13, 14] {
                        if game.scene.models[a.parts[part]].conflict.is_some() {
                            assert!((battle.start..battle.end).contains(&count));
                        }
                    }
                }
            }
            assert!(turned);
            if enabled {
                place(&mut game, front, Position::new(800, 0, 1800));
                place(&mut game, rear, Position::new(0, 0, 600));
                with_actor(&mut game, player, |a, ctx, world| {
                    a.position = Position::ZERO;
                    a.rotation[1] = 2048;
                    a.map.height = 0;
                    a.target = Some(world.roster.get(front).root_model);
                    assert_eq!(
                        a.select_melee_target(ctx, world.roster, true).unwrap(),
                        a.target
                    );
                    world.roster.get_mut(front).health = 0;
                    assert_eq!(
                        a.select_melee_target(ctx, world.roster, true).unwrap(),
                        Some(world.roster.get(rear).root_model)
                    );
                    world.roster.get_mut(rear).position.z = -1000;
                    let yaw = a.rotation[1];
                    a.steer_melee_target(ctx, world.roster).unwrap();
                    assert_eq!(a.target, None);
                    assert_eq!(a.rotation[1], yaw);
                });
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; assist rejects terrain cover, height differences, dead/suspended/civilian targets"]
fn attack_assist_requires_reachable_visible_actor() {
    let (mut game, front, rear) = ready(1);
    let player = game.player_id;
    place(&mut game, front, Position::new(0, 0, 1500));
    with_actor(&mut game, player, |a, ctx, world| {
        assert_eq!(
            a.select_melee_target(ctx, world.roster, false).unwrap(),
            Some(world.roster.get(front).root_model)
        );
        for case in 0..5 {
            let original = world.roster.get(front).clone();
            let enemy = world.roster.get_mut(front);
            match case {
                0 => enemy.health = 0,
                1 => enemy.position.y = -2000,
                2 => enemy.attribute |= ATTR_SUSPEND,
                3 => enemy.kind = 0x90,
                _ => enemy.scripted = true,
            }
            assert_eq!(
                a.select_melee_target(ctx, world.roster, false).unwrap(),
                None,
                "case {case}"
            );
            *world.roster.get_mut(front) = original;
        }
        assert!(world.roster.get(rear).position.z < 0);
    });
    // Two valid floor regions separated by solid/unwalkable terrain.
    game.mission.terrain = tenchu_assets::terrain::AreaMap::from_nodes(vec![(
        0,
        [[-3000, -3000, 3000, 50], [-3000, 90, 3000, 3000]]
            .into_iter()
            .map(|bounds| tenchu_assets::terrain::Node {
                y: 0,
                dy: 0,
                bounds,
                attributes: 0x2000,
                division: u16::MAX,
            })
            .collect(),
    )])
    .unwrap();
    game.cursor = terrain::Cursor::new(&game.mission.terrain);
    with_actor(&mut game, player, |a, ctx, world| {
        assert_eq!(
            a.select_melee_target(ctx, world.roster, false).unwrap(),
            None
        );
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; assist leaves paired stealth kills and NPC attack steering unchanged"]
fn attack_assist_keeps_stealth_and_enemy_combat() {
    for character in 0..2 {
        for enabled in [false, true] {
            let (mut game, front, _) = ready(character);
            game.combat.attack_assist = enabled;
            place(&mut game, front, Position::new(0, 0, 700));
            with_actor(&mut game, front, |a, ctx, _| {
                a.attribute = 0;
                a.equip(ctx, false);
                a.enter(ctx, MOT_NORMAL, true);
            });
            game.step(SQUARE).unwrap();
            assert_eq!(
                game.player().motion.action,
                MOT_ATTACK_STEALTH_BACK + character * 3
            );
            assert_eq!(game.statistics.criticals, 1);
        }
        let (mut baseline, enemy, _) = ready(character);
        let (mut assisted, _, _) = ready(character);
        assisted.combat.attack_assist = true;
        for game in [&mut baseline, &mut assisted] {
            with_actor(game, enemy, |a, ctx, world| {
                a.target = Some(world.roster.get(world.player).root_model);
                a.enter(ctx, MOT_ATTACK, true);
            });
        }
        for _ in 0..60 {
            baseline.step(0).unwrap();
            assisted.step(0).unwrap();
        }
        assisted.combat.attack_assist = false;
        assert_eq!(
            format!("{:?}", baseline.roster),
            format!("{:?}", assisted.roster)
        );
        assert_eq!(baseline.random, assisted.random);
    }
}
