use super::*;
use crate::actor::State as ActorState;
use crate::ids::*;
use crate::math::Position;

fn prepare(
    items: &mut Items,
    id: usize,
    player: usize,
    ctx: &mut Context,
    actors: &mut Actors,
) -> usize {
    items
        .initialize(
            id,
            behavior(Processor::Dog, Some(player), 0),
            16,
            Position::ZERO,
            ctx,
        )
        .unwrap();
    let item = &mut items.slots[id];
    item.behavior = Behavior::Dog(Used::new(player, state::Dog::Summoning { player }));
    items
        .process(
            id,
            actors,
            &mut crate::ai::Intelligence::default(),
            0,
            0,
            ctx,
        )
        .unwrap();
    assert_eq!(items.slots[id].source_mode(), 2);
    items.slots[id].summoned_dog().unwrap()
}

#[test]
#[ignore = "requires TENCHU_DISC; dog targeting excludes its owner and civilians, retains source scan/status gates and repeated sheathing quirk"]
fn dog_targeting_and_idle_repeated_sheathing_follow_source() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in actors.roster.order.clone() {
            if id != player {
                actors.roster.remove(id, ctx);
            }
        }
        let dog = prepare(items, 1, player, ctx, actors);
        let owner_flags = actors.get(player).attribute;
        let enemy = actors
            .roster
            .spawn(0x10, Position::new(0, 0, 5000), 0, ctx)
            .unwrap();
        let civilian = actors
            .roster
            .spawn(0x10, Position::new(0, 0, 100), 0, ctx)
            .unwrap();
        actors.get_mut(civilian).kind = 0x90;
        let suspended = actors
            .roster
            .spawn(0x10, Position::new(0, 0, 200), 0, ctx)
            .unwrap();
        actors.get_mut(suspended).attribute |= ATTR_SUSPEND;
        actors.get_mut(player).position = Position::new(0, 0, 50);
        actors.get_mut(dog).map.height = 0;
        let owner_root = actors.get(player).root_model;
        for (status, clock) in [(0, 14), (7, 15), (8, 15), (16, 15)] {
            actors.get_mut(dog).status = ActorState::from_source_id(status);
            items
                .process(
                    1,
                    actors,
                    &mut crate::ai::Intelligence::default(),
                    0,
                    clock,
                    ctx,
                )
                .unwrap();
            assert_eq!(actors.get(dog).target, Some(owner_root));
        }
        actors.get_mut(dog).status = ActorState::Normal;
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                15,
                ctx,
            )
            .unwrap();
        assert_eq!(actors.get(dog).target, Some(actors.get(enemy).root_model));
        assert_eq!(actors.get(dog).think.source_ids(), [9, 4, 4, 5]);
        assert_eq!(actors.get(dog).motion.action, MOT_STATE_DRAW);
        assert_ne!(actors.get(dog).attribute & ATTR_WEAPON_DRAWN, 0);
        assert_eq!(actors.get(player).attribute, owner_flags);
        actors.get_mut(enemy).health = -1;
        actors.get_mut(dog).status = ActorState::Normal;
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                30,
                ctx,
            )
            .unwrap();
        assert_eq!(actors.get(dog).target, Some(owner_root));
        assert_eq!(actors.get(dog).think.source_ids(), [0; 4]);
        assert_eq!(actors.get(dog).motion.action, MOT_STATE_SHEATHE);
        assert_eq!(
            actors.get(dog).attribute & (ATTR_WEAPON_DRAWN | 3 | ATTR_CUSTOMAI),
            0
        );
        actors.get_mut(dog).status = ActorState::Normal;
        actors.get_mut(dog).enter(ctx, MOT_NORMAL, true);
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                45,
                ctx,
            )
            .unwrap();
        assert_eq!(actors.get(dog).motion.action, MOT_STATE_SHEATHE);
        actors.get_mut(dog).status = ActorState::Normal;
        actors.get_mut(dog).enter(ctx, MOT_NORMAL, true);
        actors.get_mut(dog).target = Some(items.slots[1].model);
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                60,
                ctx,
            )
            .unwrap();
        assert_eq!(actors.get(dog).motion.action, MOT_NORMAL);
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; occupied dog retries, settled summons refund after disposal, expiry drops inventory and forced disposal only parks"]
fn dog_retries_refunds_and_cleanup_preserve_source_order() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        let dog = prepare(items, 1, player, ctx, actors);
        items
            .initialize(
                2,
                behavior(Processor::Dog, Some(player), 0),
                16,
                Position::ZERO,
                ctx,
            )
            .unwrap();
        items.slots[2].behavior =
            Behavior::Dog(Used::new(player, state::Dog::Summoning { player }));
        items
            .process(
                2,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                1,
                ctx,
            )
            .unwrap();
        assert_eq!(items.slots[2].source_mode(), 0);
        assert!(matches!(
            items.slots[2].behavior,
            Behavior::Dog(Used {
                phase: state::Dog::Thrown { remaining: 15, .. },
                ..
            })
        ));
        items.slots[2].rolling.status = RollingStatus::Settled;
        for id in 0..CAPACITY {
            if id != 1 && id != 2 {
                items
                    .initialize(
                        id,
                        behavior(Processor::Flute, Some(player), 0),
                        18,
                        Position::ZERO,
                        ctx,
                    )
                    .unwrap();
            }
        }
        items.cursor = 1;
        items
            .process(
                2,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                2,
                ctx,
            )
            .unwrap();
        assert_eq!(items.cursor, 2);
        assert_eq!(items.slots[2].processor(), Some(Processor::Pickup));
        assert_eq!(items.slots[2].kind, 16);
        assert_eq!(items.slots[2].owner(), None);
        for id in 2..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        actors.get_mut(dog).inventory[3] = 2;
        items.slots[1].behavior = Behavior::Dog(Used::new(
            player,
            state::Dog::Active {
                slave: dog,
                remaining: 1,
                player,
            },
        ));
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                3,
                ctx,
            )
            .unwrap();
        assert!(items.slots[1].processor().is_none());
        assert_eq!(actors.get(dog).inventory[3], 0);
        assert_eq!(actors.get(dog).position, [999000; 3]);
        assert_eq!(
            items
                .slots
                .iter()
                .filter(|i| i.processor() == Some(Processor::Pickup) && i.kind == 3)
                .count(),
            2
        );
        assert!(ctx.effects.sounds.iter().any(|(_, sound)| *sound == 0x23));
        let reused = prepare(items, 1, player, ctx, actors);
        assert_eq!(reused, dog);
        actors.get_mut(dog).inventory[3] = 2;
        let rng = *ctx.random;
        items.dispose(1, actors, ctx).unwrap();
        assert_eq!(actors.get(dog).inventory[3], 2);
        assert_eq!(*ctx.random, rng);
        assert_eq!(actors.get(dog).position, [999000; 3]);
        assert_ne!(actors.get(dog).attribute & ATTR_SUSPEND, 0);
    });
}
