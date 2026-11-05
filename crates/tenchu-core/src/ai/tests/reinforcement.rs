use super::*;
use crate::{game::Game, roster::ReinforcementCall};
use tenchu_assets::terrain::{AreaMap, Node};

pub(super) fn corner() -> Game {
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
    actor.equip(&ctx, true);
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
#[ignore = "requires TENCHU_DISC; witnessed-kill requests use covered, bounded calls and restore ordinary AI afterwards"]
fn witnessed_kill_request_finishes_with_nonrecursive_recruits() {
    let mut game = corner();
    let caller = game.roster.order[1];
    game.roster.reinforcement_call.witnessed_kills = true;
    game.roster.reinforcement_call.count = 3;
    game.roster.get_mut(caller).recruitment = Recruitment::Requested;
    game.roster.get_mut(caller).brain.timer = 2;
    for _ in 0..20 {
        game.step(0).unwrap();
    }
    assert_eq!(game.roster.order.len(), 5);
    assert_eq!(game.roster.get(caller).recruitment, Recruitment::Called);
    for &id in &game.roster.order[2..] {
        assert_eq!(game.roster.get(id).recruitment, Recruitment::Reinforcement);
    }
    for _ in 0..300 {
        game.step(0).unwrap();
    }
    assert_eq!(game.roster.order.len(), 5);

    game.roster.get_mut(caller).recruitment = Recruitment::Requested;
    game.roster.reinforcement_call.witnessed_kills = false;
    game.step(0).unwrap();
    assert_ne!(game.roster.get(caller).recruitment, Recruitment::Requested);
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
            game.roster.reinforcement_call = ReinforcementCall {
                count,
                random,
                ..Default::default()
            };
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
    call_at(game, id, timer, Position::new(0, 0, 18000), false);
}

fn call_at(game: &mut Game, id: usize, timer: u8, position: Position, civilian: bool) {
    let mut actor = game.roster.take(id);
    actor.position = position;
    actor.brain.chase = [position.x, position.z];
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
    let mut controller = Controller::new(
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
    );
    if civilian {
        controller.call_aid().unwrap();
        assert!(
            controller.replaced.is_none(),
            "an invalid spawn must keep the caller"
        );
    } else {
        controller.alarm().unwrap();
    }
    game.roster.restore(actor);
}

#[test]
#[ignore = "requires TENCHU_DISC; missing terrain must not qualify as hidden reinforcement ground"]
fn all_active_calls_reject_out_of_bounds_holes_and_missing_floors() {
    for (point, civilian) in [
        Position::new(40000, 0, 18000),
        Position::new(20000, 0, 10000),
        Position::new(0, 10000, 18000),
    ]
    .into_iter()
    .flat_map(|point| [false, true].map(|civilian| (point, civilian)))
    {
        let mut game = corner();
        let caller = game.roster.order[1];
        let mut cursor = terrain::Cursor::new(&game.mission.terrain);
        assert_eq!(
            terrain::level(
                &game.mission.terrain,
                &mut cursor,
                point,
                terrain::STEP_DOWN
            )
            .unwrap(),
            terrain::NONE
        );
        let seed = game.random.seed;
        let population = game.roster.order.len();
        let models = game.scene.models.len();
        call_at(&mut game, caller, 2, point, civilian);
        assert_eq!(
            game.roster.order.len(),
            population,
            "invalid spawn point: {point:?}"
        );
        assert_eq!(game.scene.models.len(), models);
        assert_eq!(game.random.seed, seed);
        assert_eq!(game.roster.get(caller).recruitment, Recruitment::Available);

        call_at(&mut game, caller, 2, Position::new(0, 0, 17500), false);
        assert_eq!(game.roster.order.len(), population + 1);
        let recruit = game.roster.get(*game.roster.order.last().unwrap());
        assert_eq!(recruit.position, Position::new(0, 0, 17500));
    }
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
        ..Default::default()
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
