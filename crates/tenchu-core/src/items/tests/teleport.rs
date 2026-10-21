use super::*;
use crate::math::Position;
use crate::{
    camera::Mode,
    effects::Effect,
    input::{L1, TRIANGLE},
};

#[test]
#[ignore = "requires TENCHU_DISC; debug teleport ray boundaries, no-hit fallback, camera-owner movement and unchanged coordinate caches"]
fn teleport_release_preserves_source_ray_and_raw_position_write() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        let owner = actors.roster.spawn(0x10, Position::ZERO, 0, ctx).unwrap();
        let hit = [29889, -1000, 0];
        let cases = [
            (
                [20000, 0, 0],
                [29000, -1000, 0],
                [31000, -1000, 0],
                hit,
                true,
                false,
            ),
            (
                [hit[0] - 19968, -1000, 0],
                [29000, -1000, 0],
                [31000, -1000, 0],
                hit,
                true,
                true,
            ),
            (
                [hit[0] - 20224, -1000, 0],
                [29000, -1000, 0],
                [31000, -1000, 0],
                hit,
                true,
                false,
            ),
            (
                [hit[0] - 20480, -1000, 0],
                [29000, -1000, 0],
                [31000, -1000, 0],
                hit,
                false,
                false,
            ),
            (
                [5000, 0, 0],
                [0, -1000, 0],
                [2000, -1000, 0],
                [5000, 0, 0],
                true,
                false,
            ),
        ];
        for (position, camera, reference, target, accepted, look) in cases {
            let actor = actors.get_mut(player);
            actor.position = position.into();
            actor.pad.held = if look { L1 } else { 0 };
            let root = actor.root_model;
            ctx.scene.set_translation(root, position.into());
            ctx.scene.models[root].flags = 73;
            let cached = ctx.scene.models[root].cached;
            let rotation = ctx.scene.models[root].local.rotation;
            ctx.camera.position = camera.into();
            ctx.camera.reference = reference.into();
            ctx.camera.mode = Mode::Direction;
            ctx.camera.detail = crate::camera::Detail::Sight;
            *ctx.effects = effects::Effects::default();
            items
                .initialize(1, Processor::Teleport, 24, Some(owner), Position::ZERO, ctx)
                .unwrap();
            let seed = *ctx.random;
            items.teleport(1, actors, player, ctx).unwrap();
            assert_eq!(ctx.camera.target, target);
            let expected = if accepted { target } else { position };
            assert_eq!(actors.get(player).position, expected);
            assert_eq!(ctx.scene.models[root].local.translation, expected);
            assert_eq!(actors.get(owner).position, [0; 3]);
            assert_eq!(ctx.scene.models[root].flags, 73);
            assert_eq!(ctx.scene.models[root].cached, cached);
            assert_eq!(ctx.scene.models[root].local.rotation, rotation);
            assert_eq!(
                ctx.effects
                    .slots
                    .iter()
                    .filter(|s| matches!(s, Some(Effect::Bleed(_))))
                    .count(),
                if accepted { 50 } else { 0 }
            );
            assert_eq!(
                ctx.camera.mode,
                if look { Mode::Direction } else { Mode::Normal }
            );
            assert_eq!(ctx.random.calls != seed.calls, accepted);
            assert!(items.slots[1].processor.is_none());
        }
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; teleport request assigns sight directly, holds the reticle without aimed-item ownership and forced disposal leaves the camera"]
fn teleport_request_hold_and_forced_disposal_keep_original_camera_rules() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        let mut actor = actors.roster.take(player);
        actor.inventory[24] = 3;
        actor.inventory[25] = 9;
        actor.pad.held = TRIANGLE;
        ctx.camera.direction = [11, 22];
        items
            .request_default(24, &mut actor, player, actors.roster, ctx)
            .unwrap();
        actors.roster.restore(actor);
        let id = items.cursor;
        assert_eq!(actors.get(player).inventory[24], 2);
        assert_eq!(actors.get(player).inventory[25], 9);
        assert_eq!(ctx.camera.mode, Mode::Sight);
        assert_eq!(ctx.camera.direction, [11, 22]);
        let bytes = ctx.drawing.bytes;
        let random = *ctx.random;
        items.teleport(id, actors, player, ctx).unwrap();
        assert_eq!(ctx.camera.mode, Mode::Direction);
        assert_eq!(ctx.camera.detail, crate::camera::Detail::Sight);
        assert_eq!(items.sprites.last().unwrap().blend, 2);
        assert_eq!(ctx.drawing.bytes, bytes + 24);
        assert_eq!(*ctx.random, random);
        items.dispose(id, actors, ctx).unwrap();
        assert_eq!(ctx.camera.mode, Mode::Direction);
        assert_eq!(ctx.camera.detail, crate::camera::Detail::Sight);
        assert_eq!(actors.get(player).inventory[25], 9);
    });
}
