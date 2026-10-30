use super::*;
use crate::math::Position;
use crate::{ai::Intelligence, ids::*};

fn food(game: &mut Game, owner_first: bool, owner_distance: i32) -> (usize, usize) {
    oracle::flat_ground(game);
    let player = game.player_id;
    with_items(game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        for id in actors.roster.order.clone() {
            if id != player {
                actors.roster.remove(id, ctx);
            }
        }
        let owner = actors.get_mut(player);
        owner.position = Position::new(0, 0, -owner_distance);
        ctx.scene.update(owner.root_model, owner.root(ctx.math));
        let root = owner.root_model;
        let enemy = actors
            .roster
            .spawn(0x10, Position::new(0, 0, 100), 2048, ctx)
            .unwrap();
        let actor = actors.get_mut(enemy);
        actor.attribute = 0;
        actor.think = [0; 4];
        actor.target = Some(root);
        if !owner_first {
            actors.roster.order.reverse();
        }
        let id = items.allocate(actors, ctx).unwrap();
        items
            .initialize(id, Processor::Poison, 7, Some(player), Position::ZERO, ctx)
            .unwrap();
        items.slots[id].mode = 1;
        items.slots[id].rolling.status = RollingStatus::Settled;
        (id, enemy)
    })
}

#[test]
#[ignore = "requires TENCHU_DISC; interrupted eating drops the same item and disposal restores an idle AI callback"]
fn interrupted_eating_keeps_bait_until_cleanup() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    let (id, enemy) = food(&mut game, true, 2000);
    let mut intelligence = Intelligence::default();
    with_items(&mut game, |items, ctx, actors| {
        items
            .poison(id, actors, &mut intelligence, player, 2, ctx)
            .unwrap();
        assert_eq!(items.slots[id].mode, 2);
        assert_eq!(items.slots[id].poison.original_think, Some(0));
        assert_eq!(actors.get(enemy).think[0], 9);
        assert_eq!(actors.get(enemy).motion.action, MOT_ITEM_DRINK);
        let model = items.slots[id].model;
        assert_eq!(
            ctx.scene.models[model].parent,
            Some(actors.get(enemy).parts[14])
        );
        actors.get_mut(enemy).enter(ctx, MOT_DAMAGE, true);
        let position = ctx.scene.world(model).unwrap().translation;
        let mut random = *ctx.random;
        let velocity = [
            (random.next15() % 200 - 100) as i16,
            (random.next15() % 100 - 200) as i16,
            (random.next15() % 200 - 100) as i16,
        ];
        items
            .poison(id, actors, &mut intelligence, player, 4, ctx)
            .unwrap();
        assert_eq!(items.slots[id].mode, 0);
        assert_eq!(items.slots[id].poison.remaining, 30);
        assert_eq!(items.slots[id].rolling.velocity, velocity);
        assert_eq!(*ctx.random, random);
        assert_eq!(ctx.scene.models[model].parent, None);
        assert_eq!(ctx.scene.models[model].local.translation, position);
        assert_eq!(actors.get(enemy).target, Some(model));
        items.dispose(id, actors, ctx).unwrap();
        assert_eq!(actors.get(enemy).think[0], 0);
        assert_eq!(
            actors.get(enemy).target,
            Some(actors.get(player).root_model)
        );
        assert!(items.slots[id].processor.is_none());
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; a closer earlier enemy can suppress owner pickup despite the owner being in range"]
fn owner_recovery_keeps_target_search_order() {
    for owner_first in [true, false] {
        let mut game = oracle::game(0, 0);
        let player = game.player_id;
        let (id, enemy) = food(&mut game, owner_first, 200);
        with_items(&mut game, |items, ctx, actors| {
            items
                .poison(id, actors, &mut Intelligence::default(), player, 2, ctx)
                .unwrap();
            if owner_first {
                assert!(
                    !items
                        .slots
                        .iter()
                        .any(|i| i.processor == Some(Processor::Poison))
                );
                assert_eq!(
                    items
                        .slots
                        .iter()
                        .filter(|i| i.processor == Some(Processor::Pickup) && i.kind == 7)
                        .count(),
                    1
                );
                assert_eq!(actors.get(enemy).think[0], 0);
            } else {
                assert_eq!(items.slots[id].mode, 2);
                assert_eq!(items.slots[id].poison.eater, Some(enemy));
            }
        });
    }
}
