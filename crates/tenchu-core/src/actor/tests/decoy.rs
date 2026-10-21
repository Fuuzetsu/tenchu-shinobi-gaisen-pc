use super::*;
use crate::{input::TRIANGLE, items::Processor};

#[test]
#[ignore = "requires TENCHU_DISC; both characters throw a decoy through its ninety-tick wait, sixteen-tick growth and persistent collision"]
fn decoy_throw_grows_into_the_original_model_and_hitbox() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        game.camera.snap_pending = true;
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, world| {
            world.roster.remove(enemy, ctx);
            actor.inventory[14] = 2;
        });
        game.hud.cursor = 14;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().inventory[14], 1);
        assert_eq!(game.player().motion.action, MOT_ITEM_THROW);
        let id = game
            .items
            .slots
            .iter()
            .position(|i| i.processor == Some(Processor::Decoy))
            .unwrap();
        assert_eq!(game.items.slots[id].count, 89);
        assert_eq!(game.items.slots[id].decoy.health, 99);
        for _ in 0..89 {
            game.step(0).unwrap();
        }
        assert_eq!(game.items.slots[id].mode, 1);
        assert_eq!(game.items.slots[id].count, 0);
        assert_eq!(game.items.slots[id].decoy.health, 3);
        let model = game.items.slots[id].model;
        for tick in 1..=16 {
            game.step(0).unwrap();
            assert_eq!(game.scene.models[model].conflict.is_some(), tick == 16);
            let rotation = game
                .math
                .rotate_yxz(game.items.slots[id].rotation)
                .map(|row| row.map(|v| ((i32::from(v) * tick * 256) >> 12) as i16));
            assert_eq!(game.scene.models[model].local.rotation, rotation);
        }
        assert_eq!(game.items.slots[id].mode, 2);
        assert_eq!(game.items.slots[id].count, 3);
        let contact = &game.collision.slots[game.scene.models[model].conflict.unwrap()];
        assert_eq!(contact.size, [500; 3]);
        assert_eq!(contact.offset[1], -250);
        assert_eq!(
            contact.class,
            crate::collision::STAND | crate::collision::SOFT
        );
        assert!(game.items.models.iter().any(|(asset, _)| *asset == 27));
        for _ in 0..180 {
            game.step(0).unwrap();
        }
        assert_eq!(game.items.slots[id].processor, Some(Processor::Decoy));
        assert_eq!(game.items.slots[id].decoy.health, 3);
        assert_eq!(game.player().health, game.player().max_health);
    }
}
