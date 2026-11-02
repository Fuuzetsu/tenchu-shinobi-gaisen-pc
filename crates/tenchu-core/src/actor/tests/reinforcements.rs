use super::*;
use crate::ai::Recruitment;

#[test]
#[ignore = "requires TENCHU_DISC; an actual lethal sword hit requests one visible, eligible witness without extra RNG"]
fn witnessed_combat_kills_require_sight_and_do_not_recruit_recursively() {
    let mut baseline_calls = None;
    for case in [
        "off", "retail", "visible", "wall", "behind", "far", "recruit", "called", "cap", "zero",
    ] {
        let mut game = oracle::game(0, 0);
        let victim = arena(&mut game, 0x10);
        let player = game.player_id;
        let witness = with_actor(&mut game, player, |_, ctx, world| {
            world
                .roster
                .spawn(0x10, Position::new(0, 0, 8000), 0, ctx)
                .unwrap()
        });
        let another = with_actor(&mut game, player, |_, ctx, world| {
            world
                .roster
                .spawn(0x10, Position::new(0, 0, 10000), 0, ctx)
                .unwrap()
        });
        game.roster.keep_all_active = case != "retail";
        game.roster.reinforcement_call.witnessed_kills = case != "off";
        game.roster.reinforcement_call.count = if case == "zero" { 0 } else { 1 };
        game.roster.reinforcement_limit = (case == "cap").then_some(4);
        game.intelligence.difficulty = crate::ai::Difficulty::Easy;
        for id in [witness, another] {
            let actor = game.roster.get_mut(id);
            actor.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | 2;
            if case == "behind" {
                actor.rotation[1] = 2048;
            }
            if case == "far" {
                actor.position.z += 20000;
            }
            actor.recruitment = match case {
                "recruit" => Recruitment::Reinforcement,
                "called" => Recruitment::Called,
                _ => Recruitment::Available,
            };
        }
        let weapon = super::multi_target::swing(&mut game, player, 0);
        game.scene.finish_tick();
        game.collision.compute(&mut game.scene).unwrap();
        let slot = game.scene.models[weapon].conflict.unwrap();
        let target = game.collision.slots[slot].position;
        with_actor(&mut game, victim, |actor, ctx, _| {
            actor.health = 1;
            actor.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | 2;
            actor.position = Position::ZERO;
            actor.rotation = [0; 3];
            actor.enter(ctx, MOT_ENGAGE_STANCE, true);
            actor.play_pose(ctx).unwrap();
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
        });
        for _ in 0..2 {
            game.scene.finish_tick();
            game.collision.compute(&mut game.scene).unwrap();
            with_actor(&mut game, victim, |actor, ctx, _| {
                let center = actor.collision_center(ctx);
                actor.position = Position::from_fn(|i| actor.position[i] + target[i] - center[i]);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
        }
        if case == "wall" {
            use tenchu_assets::terrain::{AreaMap, Node};
            game.mission.terrain = AreaMap::from_nodes(vec![(
                0,
                [[-3000, -3000, 3000, 300], [-3000, 500, 3000, 3000]]
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
        }
        let calls = game.random.calls;
        assert!(super::multi_target::contact(&mut game, victim), "{case}");
        assert_eq!(game.roster.get(victim).health, 0, "{case}");
        let extra = game.random.calls - calls;
        assert_eq!(
            *baseline_calls.get_or_insert(extra),
            extra,
            "{case}: witness search must not consume RNG"
        );
        assert_eq!(
            game.roster.get(witness).recruitment == Recruitment::Requested,
            case == "visible",
            "{case}"
        );
        assert_ne!(
            game.roster.get(another).recruitment,
            Recruitment::Requested,
            "one caller per kill"
        );
        assert_eq!(
            game.roster.order.len(),
            4,
            "calling must not immediately spawn a guard"
        );
    }
}
