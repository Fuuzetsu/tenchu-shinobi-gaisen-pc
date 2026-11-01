use super::*;
use crate::{game::Game, roster::ReinforcementCall};
use tenchu_assets::terrain::{AreaMap, Node};

fn corner() -> Game {
    let mut game = oracle::game(7, 0);
    let guard = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| (0x10..0x80).contains(&game.roster.get(id).kind))
        .unwrap();
    let mut ctx = Context {
        mission: &mut game.mission,
        math: &game.math,
        body: &game.body,
        cursor: &mut game.cursor,
        random: &mut game.random,
        effects: &mut game.effects,
        scene: &mut game.scene,
        collision: &mut game.collision,
        camera: &mut game.camera,
        drawing: &mut game.drawing,
    };
    for id in game.roster.order.clone() {
        if id != game.player_id && id != guard {
            game.roster.remove(id, &mut ctx);
        }
    }
    let player = game.roster.get_mut(game.player_id);
    player.position = Position::ZERO;
    player.invulnerable = true;
    let target = player.root_model;
    ctx.scene.set_translation(target, Position::ZERO);
    let actor = game.roster.get_mut(guard);
    actor.position = Position::new(0, 0, 18000);
    actor.home = [0, 18000];
    actor.target = Some(target);
    actor.attribute = ATTR_CUSTOMAI | ATTR_SEARCH | ATTR_WEAPON_DRAWN | 1;
    actor.setup_think(0x4444).unwrap();
    actor.brain.chase = [0, 18000];
    actor.equip(&mut ctx, true);
    actor.enter(&ctx, MOT_ENGAGE_STANCE, true);
    ctx.scene.models[actor.parts[0]].attribute |= crate::scene::COLLIDE;
    ctx.scene.set_translation(actor.root_model, actor.position);
    game.mission.terrain = AreaMap::from_nodes(vec![(
        0,
        [[-3000, -3000, 3000, 800], [-3000, 1200, 3000, 1800]]
            .map(|bounds| Node {
                y: 0,
                dy: 0,
                bounds,
                attributes: 0x2000,
                division: u16::MAX,
            })
            .to_vec(),
    )])
    .unwrap();
    game.cursor = terrain::Cursor::new(&game.mission.terrain);
    for &id in &game.roster.order {
        game.roster.actors[id].as_mut().unwrap().map = terrain::Probe {
            area: game.cursor.area,
            attribute: 0x2000,
            ..Default::default()
        };
    }
    game.roster.keep_all_active = true;
    game.intelligence.difficulty = Difficulty::Hard;
    game.intelligence.reset_alert();
    game.clock = 31;
    game
}

#[test]
#[ignore = "requires TENCHU_DISC; a blocked caller and recruits must not recursively populate an unlimited roster"]
fn all_active_alarm_calls_are_bounded() {
    for (count, random, limit) in [
        (0, false, None),
        (1, false, None),
        (4, false, None),
        (32, false, Some(5)),
        (5, true, None),
    ] {
        let mut outcomes = std::collections::BTreeSet::new();
        for seed in [1, 19, 73] {
            let mut game = corner();
            game.random = Random::new(seed);
            game.roster.reinforcement_call = ReinforcementCall { count, random };
            game.roster.reinforcement_limit = limit;
            for _ in 0..20 {
                game.step(0).unwrap();
            }
            let population = game.roster.order.len();
            if random {
                assert!((3..=2 + usize::from(count)).contains(&population));
                outcomes.insert(population);
            } else {
                assert_eq!(
                    population,
                    (2 + usize::from(count)).min(limit.unwrap_or(usize::MAX))
                );
            }
            game.roster.reinforcement_limit = None;
            for _ in 0..900 {
                game.step(0).unwrap();
                assert_eq!(
                    game.roster.order.len(),
                    population,
                    "count={count}, random={random}, seed={seed}"
                );
            }
        }
        if random {
            assert!(
                outcomes.len() > 1,
                "Random calls always produced the same group size"
            );
        }
    }
}

fn alarm(game: &mut Game, id: usize, timer: u8) {
    let mut actor = game.roster.take(id);
    actor.position = Position::new(0, 0, 18000);
    actor.brain.chase = [0, 18000];
    actor.brain.timer = timer;
    let mut ctx = Context {
        mission: &mut game.mission,
        math: &game.math,
        body: &game.body,
        cursor: &mut game.cursor,
        random: &mut game.random,
        effects: &mut game.effects,
        scene: &mut game.scene,
        collision: &mut game.collision,
        camera: &mut game.camera,
        drawing: &mut game.drawing,
    };
    let player = Player::new(game.roster.get(game.player_id), &ctx);
    game.intelligence.sight = Sight {
        distance: 18000,
        direction: 0,
    };
    Controller::new(
        &mut actor,
        &mut game.roster,
        &mut game.items,
        &mut game.intelligence,
        Input {
            player,
            clock: game.clock,
            buttons: 0,
            commands: &game.commands,
        },
        &mut ctx,
    )
    .alarm()
    .unwrap();
    game.roster.restore(actor);
}

#[test]
#[ignore = "requires TENCHU_DISC; only placed guards can call again after a fully ended alert, and retail behavior stays independent"]
fn recruitment_permissions_survive_new_alerts_and_settings_changes() {
    let mut game = corner();
    let caller = game.roster.order[1];
    alarm(&mut game, caller, 2);
    assert_eq!(game.roster.order.len(), 3);
    let recruit = *game.roster.order.last().unwrap();
    for id in [caller, recruit] {
        alarm(&mut game, id, 0);
        assert_eq!(game.roster.get(id).brain.timer, 1);
        alarm(&mut game, id, 2);
        assert_eq!(game.roster.order.len(), 3);
    }
    for id in [caller, recruit] {
        game.roster.get_mut(id).attribute &= !(ATTR_PHASE | ATTR_SEARCH);
    }
    game.intelligence.alert = 100;
    game.step(0).unwrap();
    assert_eq!(game.roster.get(caller).recruitment, Recruitment::Called);
    game.intelligence.alert = 0;
    for _ in 0..120 {
        game.step(0).unwrap();
        if game.roster.get(caller).recruitment == Recruitment::Available {
            break;
        }
    }
    assert_eq!(game.roster.get(caller).recruitment, Recruitment::Available);
    assert_eq!(
        game.roster.get(recruit).recruitment,
        Recruitment::Reinforcement
    );
    alarm(&mut game, recruit, 2);
    assert_eq!(game.roster.order.len(), 3);
    alarm(&mut game, caller, 2);
    assert_eq!(game.roster.order.len(), 4);

    game.roster.keep_all_active = false;
    game.roster.reinforcement_call = ReinforcementCall {
        count: 0,
        random: true,
    };
    alarm(&mut game, recruit, 2);
    assert_eq!(
        game.roster.order.len(),
        5,
        "Original mode retains recursive calls"
    );
    assert_eq!(
        game.roster.get(recruit).recruitment,
        Recruitment::Reinforcement
    );
    game.roster.keep_all_active = true;
    game.roster.reinforcement_call = Default::default();
    alarm(&mut game, caller, 2);
    alarm(&mut game, recruit, 2);
    assert_eq!(
        game.roster.order.len(),
        5,
        "Changing settings must not reset eligibility"
    );
}
