use super::*;
use crate::{
    collision,
    ids::ATTR_SUSPEND,
    scene::{COLLIDE, CONFLICT},
};

fn prepare(items: &mut Items, id: usize, player: usize, position: [i32; 3], ctx: &mut Context) {
    items
        .initialize(id, Processor::Decoy, 14, Some(player), position, ctx)
        .unwrap();
    let item = &mut items.slots[id];
    item.rolling = Rolling {
        status: 4,
        ..Rolling::default()
    };
    item.decoy = crate::items::decoy::Decoy { health: 3, player };
    item.mode = 2;
    item.count = 1;
    items.decoy_count += 1;
}

#[test]
#[ignore = "requires TENCHU_DISC; decoys retarget only closer existing targets, include suspended/dead actors, exclude bosses and restore the player target"]
fn decoys_retarget_source_eligible_actors_and_restore_them_on_disposal() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        prepare(items, 1, player, [0, 0, 1000], ctx);
        let model = items.slots[1].model;
        let far = ctx.scene.add(
            None,
            Matrix {
                translation: [0, 0, 5000],
                ..Matrix::default()
            },
        );
        let near = ctx.scene.add(
            None,
            Matrix {
                translation: [0, 0, 1500],
                ..Matrix::default()
            },
        );
        let cases = [
            (2000, Some(far), false, false, true),
            (2000, Some(near), false, false, false),
            (11000, Some(far), false, false, false),
            (2000, Some(far), true, false, false),
            (2000, None, false, false, false),
            (2000, Some(far), false, true, true),
        ];
        let mut ids = Vec::new();
        for (z, target, boss, suspended, _) in cases {
            let id = actors.roster.spawn(0x10, [0, 0, z], 0, ctx).unwrap();
            let actor = actors.get_mut(id);
            actor.target = target;
            if boss {
                actor.kind = 0x80;
            }
            if suspended {
                actor.attribute |= ATTR_SUSPEND;
                actor.health = 0;
                actor.status = 17;
            }
            ids.push(id);
        }
        items.decoy(1, actors, ctx).unwrap();
        for (&id, &(_, target, _, _, lure)) in ids.iter().zip(&cases) {
            assert_eq!(
                actors.get(id).target,
                if lure { Some(model) } else { target }
            );
        }
        assert_eq!(items.slots[1].count, 30);
        items.dispose(1, actors, ctx).unwrap();
        let player_root = actors.get(player).root_model;
        for (&id, &(_, target, _, _, lure)) in ids.iter().zip(&cases) {
            assert_eq!(
                actors.get(id).target,
                if lure { Some(player_root) } else { target }
            );
        }
        assert_eq!(items.decoy_count, 0);
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; retarget ticks consume hits without damage, bump thresholds/jitter and breaking only on the fourth attack"]
fn decoy_contacts_keep_original_order_and_fourth_hit_break() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        prepare(items, 1, player, [0; 3], ctx);
        let model = items.slots[1].model;
        items
            .contact(1, 500, collision::STAND | collision::SOFT, ctx)
            .unwrap();
        let own = ctx.scene.models[model].conflict.unwrap();
        let other_model = ctx.scene.add(None, Matrix::default());
        let other = ctx.collision.insert(ctx.scene, other_model).unwrap();
        let hit = |ctx: &mut Context, class, y| {
            ctx.scene.models[model].attribute |= COLLIDE | CONFLICT;
            ctx.collision.slots[own].results[other] = 0x81;
            ctx.collision.slots[own].count = 1;
            ctx.collision.slots[own].position = [0; 3];
            ctx.collision.slots[other].position = [1600, y, -1600];
            ctx.collision.slots[other].class = class;
        };
        hit(ctx, collision::HIT, 0);
        items.decoy(1, actors, ctx).unwrap();
        assert_eq!(items.slots[1].decoy.health, 3);
        assert_eq!(ctx.collision.slots[own].results[other], 0xc1);
        for (class, y) in [
            (collision::SOFT, 0),
            (collision::STAND, -501),
            (collision::STAND, -500),
        ] {
            items.slots[1].rolling = Rolling {
                status: 4,
                ..Rolling::default()
            };
            items.slots[1].count = 10;
            let mut random = *ctx.random;
            hit(ctx, class, y);
            items.decoy(1, actors, ctx).unwrap();
            let expected = if class == collision::SOFT {
                [0; 3]
            } else {
                [
                    (-200 + random.next15() % 20 - 10) as i16,
                    if y >= -500 { -100 } else { 0 },
                    (200 + random.next15() % 20 - 10) as i16,
                ]
            };
            assert_eq!(items.slots[1].rolling.velocity, expected);
            assert_eq!(*ctx.random, random);
        }
        for hit_number in 1..=4 {
            items.slots[1].rolling = Rolling {
                status: 4,
                ..Rolling::default()
            };
            items.slots[1].count = 10;
            hit(ctx, collision::HIT, 0);
            items.decoy(1, actors, ctx).unwrap();
            if hit_number < 4 {
                assert_eq!(items.slots[1].decoy.health, 3 - hit_number);
                assert_eq!(items.slots[1].rolling.velocity, [-100, -100, 100]);
                assert_eq!(ctx.effects.sounds.last().unwrap().1, 0x30);
            } else {
                assert!(items.slots[1].processor.is_none());
                assert!(ctx.scene.models[model].conflict.is_none());
                assert_eq!(items.slots[1].mode, 1);
                assert_eq!(items.decoy_count, 0);
                assert_eq!(ctx.effects.sounds.last().unwrap().1, 0x23);
            }
        }
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; only three dolls grow, excess casts refund, and water cleans up both active and unactivated decoys"]
fn decoy_cap_and_water_cleanup_preserve_active_count() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        for id in 1..=4 {
            prepare(items, id, player, [id as i32 * 1000, 0, 0], ctx);
            items.decoy_count -= 1;
            items.slots[id].mode = 0;
            items.slots[id].decoy.health = 99;
        }
        for id in 1..=4 {
            items.decoy(id, actors, ctx).unwrap();
        }
        assert_eq!(items.decoy_count, 3);
        assert!(items.slots[4].processor.is_none());
        assert_eq!(
            items
                .slots
                .iter()
                .filter(|i| i.processor == Some(Processor::Pickup) && i.kind == 14)
                .count(),
            1
        );
        let root = actors.get(player).root_model;
        actors.get_mut(player).target = Some(items.slots[1].model);
        assert_ne!(actors.get(player).target, Some(root));
    });
    oracle::flat_terrain(&mut game, 0x2004);
    with_items(&mut game, |items, ctx, actors| {
        items.slots[1].rolling = Rolling {
            velocity: [0, 40, 0],
            ..Rolling::default()
        };
        items.decoy(1, actors, ctx).unwrap();
        assert!(items.slots[1].processor.is_none());
        assert_eq!(items.decoy_count, 2);
        assert_eq!(
            actors.get(player).target,
            Some(actors.get(player).root_model)
        );
        prepare(items, 4, player, [0; 3], ctx);
        items.decoy_count -= 1;
        items.slots[4].mode = 0;
        items.slots[4].decoy.health = 99;
        items.slots[4].rolling = Rolling {
            velocity: [0, 40, 0],
            ..Rolling::default()
        };
        items.decoy(4, actors, ctx).unwrap();
        assert!(items.slots[4].processor.is_none());
        assert_eq!(items.decoy_count, 2);
    });
}
