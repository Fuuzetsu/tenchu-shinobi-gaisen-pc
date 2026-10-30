use super::*;
use crate::actor::State as ActorState;
use crate::math::Position;
use crate::{ids::*, input::*};

#[test]
#[ignore = "requires TENCHU_DISC; both characters plant rice, retain its map marker and collect it without emitting a mine"]
fn rice_planting_map_and_pickup_use_ordinary_ticks() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let player = game.player_id;
        game.roster.get_mut(player).inventory[8] = 2;
        game.hud.cursor = 8;
        game.step(0).unwrap();
        let velocity = Matrix {
            rotation: game.math.rotate_yxz(game.player().rotation),
            ..Matrix::default()
        }
        .apply_large(Position::new(0, 0, -100))
        .map(|v| v as i16);
        game.step(TRIANGLE | SELECT).unwrap();
        assert_eq!(game.player().inventory[8], 1);
        assert_eq!(
            (game.player().motion.action, game.player().status),
            (MOT_ITEM_PLANT, ActorState::Transition)
        );
        let cast = game
            .items
            .slots
            .iter()
            .find(|i| i.processor() == Some(Processor::Rice))
            .unwrap();
        assert_eq!(cast.rolling.velocity, velocity.to_array());
        let mut dropped = false;
        for _ in 0..80 {
            game.step(SELECT).unwrap();
            assert!(
                game.items
                    .slots
                    .iter()
                    .all(|i| i.processor() != Some(Processor::Mine))
            );
            assert_eq!(game.player().inventory[6], 0);
            dropped |=
                game.items.slots.iter().any(|i| {
                    i.kind == Kind::ColoredRice && i.processor() == Some(Processor::Pickup)
                });
        }
        assert!(dropped);
        assert_eq!(game.hud.map.lines.len(), 4);
        let rice = game
            .items
            .slots
            .iter()
            .find(|i| i.kind == Kind::ColoredRice && i.processor() == Some(Processor::Pickup))
            .unwrap();
        assert_eq!(rice.source_mode(), 1);
        assert_eq!(rice.owner(), Some(player));
        for _ in 0..40 {
            game.step(UP | SELECT).unwrap();
            if game.player().inventory[8] == 2 {
                break;
            }
        }
        assert_eq!(game.player().inventory[8], 2, "character {character}");
        game.step(SELECT).unwrap();
        assert_eq!(game.hud.map.lines.len(), 2);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; interruption restarts rice, and hand release frees its slot before allocating the drop"]
fn rice_restarts_and_reuses_its_slot_before_other_items() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
            items.initialize(
                id,
                behavior(Processor::Pickup, None, 0),
                Kind::Shuriken,
                Position::ZERO,
                ctx,
            );
        }
        let owner = actors.get_mut(player);
        owner.enter(ctx, MOT_DAMAGE, true);
        let action = owner.motion.action;
        let model = items.slots[1].model;
        let cached = Matrix {
            translation: Position::new(800, 900, 1000),
            ..Matrix::default()
        };
        ctx.scene.models[model].cached = cached;
        ctx.scene.models[model].flags = 59;
        items.slots[1].rotation = [20, 600, -30];
        items.initialize(
            1,
            behavior(Processor::Rice, Some(player), 0),
            Kind::ColoredRice,
            Position::ZERO,
            ctx,
        );
        assert_eq!(
            ctx.scene.models[model].local.rotation,
            ctx.math.rotate_yxz([20, 600, -30])
        );
        assert_eq!(ctx.scene.models[model].cached, cached);
        assert_eq!(ctx.scene.models[model].flags, 0);
        items.slots[1].set_source_mode(1);
        items.slots[1].rolling.velocity = [-71, -3, 68];
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence {
                    action_halt: crate::ai::Halt::Running,
                    ..Default::default()
                },
                player,
                0,
                ctx,
            )
            .unwrap();
        assert_eq!(items.slots[1].source_mode(), 0);
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence {
                    action_halt: crate::ai::Halt::Active,
                    ..Default::default()
                },
                player,
                0,
                ctx,
            )
            .unwrap();
        assert_eq!(actors.get(player).motion.action, action);
        assert_eq!(items.slots[1].source_mode(), 1);
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence {
                    action_halt: crate::ai::Halt::Running,
                    ..Default::default()
                },
                player,
                0,
                ctx,
            )
            .unwrap();
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence {
                    action_halt: crate::ai::Halt::Running,
                    ..Default::default()
                },
                player,
                0,
                ctx,
            )
            .unwrap();
        assert_eq!(
            (actors.get(player).motion.action, actors.get(player).status),
            (MOT_ITEM_PLANT, ActorState::Transition)
        );
        actors.get_mut(player).motion.count = 15;
        let position = actors.get(player).position;
        let hand_position = [position[0] + 50, position[1] - 800, position[2] + 70];
        let hand = actors.get(player).parts[13];
        ctx.scene.models[hand].parent = None;
        ctx.scene.models[hand].local.translation = hand_position.into();
        ctx.scene.models[hand].flags = 0;
        items.cursor = 20;
        let random = *ctx.random;
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence {
                    action_halt: crate::ai::Halt::Running,
                    ..Default::default()
                },
                player,
                0,
                ctx,
            )
            .unwrap();
        assert_eq!(items.cursor, 1);
        let drop = &items.slots[1];
        assert_eq!(drop.processor(), Some(Processor::Pickup));
        assert_eq!(drop.owner(), Some(player));
        assert_eq!(drop.rolling.velocity, [-71, -3, 68]);
        assert_eq!(
            ctx.scene.models[drop.model].local.translation,
            hand_position
        );
        assert_eq!(
            items
                .slots
                .iter()
                .filter(|i| i.processor().is_some())
                .count(),
            CAPACITY
        );
        assert!(
            items
                .slots
                .iter()
                .enumerate()
                .all(|(id, item)| id == 1 || item.kind == Kind::Shuriken)
        );
        assert_eq!(*ctx.random, random);
        assert_ne!(actors.get(player).motion.action, MOT_ITEM_PLANT);
    });
}
