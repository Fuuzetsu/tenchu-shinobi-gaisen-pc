use super::*;
use crate::math::Position;
use crate::{collision, ids::ATTR_SUSPEND};

#[test]
#[ignore = "requires TENCHU_DISC; lightning re-aims on clock-driven boundaries, retains its live hitbox between strikes, and disposes after the zero-count callback"]
fn lightning_reaims_and_keeps_source_postdecrement_lifetime() {
    let mut game = oracle::game(7, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in actors.roster.order.clone() {
            if id != player {
                actors.roster.remove(id, ctx);
            }
        }
        let owner = actors.roster.spawn(0x88, Position::ZERO, 0, ctx).unwrap();
        let actor = actors.get_mut(player);
        actor.position = Position::new(0, 0, -3000);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
        ctx.scene.finish_tick();
        items
            .initialize(
                1,
                Processor::Lightning,
                23,
                Some(owner),
                Position::new(0, -1000, 0),
                ctx,
            )
            .unwrap();
        items.slots[1].lightning = crate::items::lightning::Lightning {
            start: Position::new(0, -1000, 0),
            rotation: [0; 3],
        };
        let model = items.slots[1].model;
        items.lightning(1, actors, player, 1, ctx).unwrap();
        assert_eq!((items.slots[1].mode, items.slots[1].count), (1, 14));
        assert!(ctx.scene.models[model].conflict.is_none());
        assert!(!ctx.effects.sounds.iter().any(|(_, s)| *s == 0x39));
        items.lightning(1, actors, player, 2, ctx).unwrap();
        let first = ctx.scene.models[model].local.translation;
        assert_eq!(
            first,
            ctx.scene
                .world(actors.get(player).parts[0])
                .unwrap()
                .translation
        );
        let slot = ctx.scene.models[model].conflict.unwrap();
        assert_eq!(ctx.collision.slots[slot].size, [100; 3]);
        assert_eq!(ctx.collision.slots[slot].class, collision::HIT);
        let actor = actors.get_mut(player);
        actor.position.z = -6000;
        actor.attribute |= ATTR_SUSPEND;
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
        ctx.scene.finish_tick();
        items.lightning(1, actors, player, 3, ctx).unwrap();
        assert_eq!(items.slots[1].mode, 1);
        assert_eq!(ctx.scene.models[model].local.translation, first);
        items.lightning(1, actors, player, 4, ctx).unwrap();
        let second = ctx
            .scene
            .world(actors.get(player).parts[0])
            .unwrap()
            .translation;
        assert_eq!(ctx.scene.models[model].local.translation, second);
        assert_eq!(items.slots[1].mode, 2);
        for clock in 5..=16 {
            items.lightning(1, actors, player, clock, ctx).unwrap();
        }
        assert!(items.slots[1].processor.is_none());
        assert_eq!(items.slots[1].count, 255);
        assert!(ctx.scene.models[model].conflict.is_none());
        items
            .initialize(
                1,
                Processor::Lightning,
                23,
                Some(player),
                Position::ZERO,
                ctx,
            )
            .unwrap();
        items.slots[1].lightning.start = Position::ZERO;
        items.lightning(1, actors, player, 17, ctx).unwrap();
        assert!(
            ctx.effects
                .sounds
                .iter()
                .any(|&(p, s)| p == [0; 3] && s == 0x39)
        );
        let count = items.slots[1].count;
        let rng = *ctx.random;
        items.dispose(1, actors, ctx).unwrap();
        assert_eq!(items.slots[1].count, count);
        assert_eq!(*ctx.random, rng);
    });
}
