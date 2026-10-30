use super::*;
use crate::math::Position;
use crate::{camera::Mode, effects::Effect, ids::*, input::*};

#[test]
#[ignore = "requires TENCHU_DISC; speed terrain threshold, post-movement turning, root cache preservation and forced eviction"]
fn speed_movement_preserves_source_cache_and_terrain_boundary() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for y in [-100, 0, 499, 500, 501, 510] {
            items.dispose(1, actors, ctx).unwrap();
            items
                .initialize(
                    1,
                    behavior(Processor::Speed, Some(player), 0),
                    13,
                    Position::ZERO,
                    ctx,
                )
                .unwrap();
            items.slots[1].set_source_mode(2);
            items.slots[1].count = 4;
            items.slots[1].rolling.velocity = [100, 0, 0];
            let owner = actors.get_mut(player);
            owner.enter(ctx, MOT_ITEM_SHINSOKU, true);
            owner.position = Position::new(1000, y, 0);
            owner.rotation = [0; 3];
            owner.pad.held = RIGHT | LEFT;
            owner.pad.trigger = 0;
            let root = owner.root_model;
            ctx.scene.update(root, owner.root(ctx.math));
            let cached = Matrix {
                translation: Position::new(900, 800, 700),
                ..Matrix::default()
            };
            ctx.scene.models[root].cached = cached;
            ctx.scene.models[root].flags = 73;
            let rotation = ctx.scene.models[root].local.rotation;
            ctx.effects.slots.fill(None);
            let random = *ctx.random;
            items
                .process(
                    1,
                    actors,
                    &mut crate::ai::Intelligence::default(),
                    player,
                    0,
                    ctx,
                )
                .unwrap();
            let expected = if y <= 500 {
                [1100, y.min(0), 0]
            } else {
                [1000, y, 0]
            };
            assert_eq!(actors.get(player).position, expected);
            assert_eq!(ctx.scene.models[root].local.translation, expected);
            assert_eq!(ctx.scene.models[root].cached, cached);
            assert_eq!(ctx.scene.models[root].flags, 73);
            assert_eq!(ctx.scene.models[root].local.rotation, rotation);
            assert_eq!(actors.get(player).rotation[1], 64);
            assert_eq!(items.slots[1].rolling.velocity, [99, 0, -10]);
            assert_eq!(items.slots[1].count, 3);
            assert_eq!(*ctx.random, random);
            let trail = ctx
                .effects
                .slots
                .iter()
                .find_map(|s| {
                    if let Some(Effect::Impact(p)) = s {
                        Some(p)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(
                trail.position,
                [expected[0], expected[1] - 300, expected[2]]
            );
            assert_eq!(trail.size, [8192, 20480]);
            assert_eq!(trail.color, [[128; 3], [0; 3]]);
            assert_eq!((trail.sprite, trail.speed, trail.time), (3, -30, 16));
            assert_eq!(ctx.camera.mode, Mode::Crouch);
            items.dispose(1, actors, ctx).unwrap();
            assert_eq!(actors.get(player).motion.action, MOT_ITEM_SHINSOKU);
            assert_eq!(ctx.camera.mode, Mode::Crouch);
        }
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; interrupted casting refunds a rolling scroll, while active interruption does not"]
fn speed_interruption_refunds_only_the_casting_phase() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for mode in [1, 2] {
            for id in 0..CAPACITY {
                items.dispose(id, actors, ctx).unwrap();
            }
            items
                .initialize(
                    1,
                    behavior(Processor::Speed, Some(player), 0),
                    13,
                    Position::new(1000, -1200, 0),
                    ctx,
                )
                .unwrap();
            items.slots[1].set_source_mode(mode);
            actors.get_mut(player).enter(ctx, MOT_DAMAGE, true);
            let random = *ctx.random;
            items
                .process(
                    1,
                    actors,
                    &mut crate::ai::Intelligence::default(),
                    player,
                    0,
                    ctx,
                )
                .unwrap();
            assert!(items.slots[1].processor().is_none());
            assert_eq!(
                ctx.random.calls - random.calls,
                if mode == 1 { 3 } else { 0 }
            );
            let drops: Vec<_> = items
                .slots
                .iter()
                .filter(|i| i.processor() == Some(Processor::Pickup) && i.kind == 13)
                .collect();
            assert_eq!(drops.len(), usize::from(mode == 1));
            if let Some(drop) = drops.first() {
                assert_eq!(drop.owner(), Some(player));
                assert_eq!(
                    ctx.scene.models[drop.model].local.translation,
                    [1000, -1200, 0]
                );
                let mut random = random;
                assert_eq!(
                    drop.rolling.velocity,
                    [
                        (random.next15() % 200 - 100) as i16,
                        (random.next15() % 100 - 200) as i16,
                        (random.next15() % 200 - 100) as i16
                    ]
                );
            }
        }
    });
}
