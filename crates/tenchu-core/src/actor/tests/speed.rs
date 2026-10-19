use super::*;
use crate::{camera::Mode, input::*, items::Processor};

#[test]
#[ignore = "requires TENCHU_DISC; both characters cast, steer and cancel speed scrolls or run their full 75-tick duration"]
fn speed_scrolls_move_at_the_original_rate_and_return_control() {
    for character in 0..2 {
        for cancel in [false, true] {
            let mut game = oracle::game(0, character);
            let enemy = arena(&mut game, 0x10);
            let player = game.player_id;
            with_actor(&mut game, player, |actor, ctx, world| {
                world.roster.remove(enemy, ctx);
                actor.position[2] = -10000;
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
                actor.inventory[13] = 1;
            });
            game.hud.cursor = 13;
            game.step(0).unwrap();
            game.step(TRIANGLE).unwrap();
            assert_eq!(game.player().motion.action, MOT_ITEM_SHINSOKU);
            assert_eq!(game.player().inventory[13], 0);
            let id = game
                .items
                .slots
                .iter()
                .position(|i| i.processor == Some(Processor::Speed))
                .unwrap();
            for _ in 0..120 {
                if game.items.slots[id].mode == 2 {
                    break;
                }
                game.step(0).unwrap();
            }
            assert_eq!(game.items.slots[id].mode, 2);
            assert_eq!(game.items.slots[id].count, 75);
            let start = game.player().position;
            let movement = game.player().velocity.map(i32::from);
            if cancel {
                let heading = game.player().rotation[1];
                game.step(RIGHT).unwrap();
                assert_eq!(
                    game.player().position,
                    [
                        start[0] + movement[0],
                        start[1] + movement[1],
                        start[2] + movement[2] + 500
                    ]
                );
                assert_eq!(game.player().rotation[1], heading + 64);
                assert_eq!(game.camera.mode, Mode::Crouch);
                let velocity = game.items.slots[id].rolling.velocity;
                assert!(velocity[0] > 0);
                game.step(SQUARE).unwrap();
                assert_eq!(
                    game.player().position,
                    [
                        start[0] + movement[0] * 2 + i32::from(velocity[0]),
                        start[1] + movement[1] * 2 + i32::from(velocity[1]),
                        start[2] + movement[2] * 2 + 500 + i32::from(velocity[2])
                    ]
                );
            } else {
                for tick in 1..=75 {
                    game.step(0).unwrap();
                    assert_eq!(game.items.slots[id].processor.is_some(), tick < 75);
                }
                assert_eq!(
                    game.player().position,
                    [
                        start[0] + movement[0] * 75,
                        start[1] + movement[1] * 75,
                        start[2] + (movement[2] + 500) * 75
                    ]
                );
            }
            assert!(game.items.slots[id].processor.is_none());
            assert_ne!(game.player().motion.action, MOT_ITEM_SHINSOKU);
            assert_eq!(game.camera.mode, Mode::Normal);
            assert_eq!(game.player().health, game.player().max_health);
        }
    }
}
