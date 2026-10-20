use super::*;
use crate::{input::TRIANGLE, items::Processor};

#[test]
#[ignore = "requires TENCHU_DISC; a dog requested by ordinary controller input selects and damages a nearby enemy through the existing AI and combat"]
fn summoned_dog_attacks_a_nearby_enemy() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        game.camera.snap_pending = true;
        game.roster.get_mut(game.player_id).inventory[16] = 1;
        game.hud.cursor = 16;
        let health = game.roster.get(enemy).health;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        let mut selected = false;
        let mut attacked = false;
        for tick in 0..900 {
            game.step(0)
                .unwrap_or_else(|e| panic!("character {character} tick {tick}: {e}"));
            if let Some(dog) = game.roster.find(0xa9) {
                let actor = game.roster.get(dog);
                selected |= actor.target == Some(game.roster.get(enemy).root_model);
                attacked |= actor.status == 7;
            }
            if game.roster.get(enemy).health < health {
                break;
            }
        }
        assert!(selected);
        assert!(attacked);
        assert!(
            game.roster.get(enemy).health < health,
            "character {character}: dog never landed an attack"
        );
        assert_eq!(game.player().health, game.player().max_health);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; both characters summon through ordinary input, keep the dog for 1800 ticks, then reuse the parked actor"]
fn dog_summon_expires_and_reuses_the_parked_actor() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        game.camera.snap_pending = true;
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, world| {
            world.roster.remove(enemy, ctx);
            actor.inventory[16] = 2;
        });
        game.hud.cursor = 16;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_ITEM_THROW);
        assert_eq!(game.player().inventory[16], 1);
        let id = game
            .items
            .slots
            .iter()
            .position(|i| i.processor == Some(Processor::Dog))
            .unwrap();
        for _ in 0..180 {
            if game.items.slots[id].mode == 2 {
                break;
            }
            game.step(0).unwrap();
        }
        assert_eq!(game.items.slots[id].mode, 2);
        assert_eq!(game.items.slots[id].dog.remaining, 1800);
        let dog = game.items.slots[id].dog.slave.unwrap();
        assert_eq!(game.roster.get(dog).kind, 0xa9);
        assert_eq!(game.roster.get(dog).health, game.roster.get(dog).max_health);
        assert_eq!(game.roster.get(dog).target, Some(game.player().root_model));
        assert_eq!(game.roster.get(dog).attribute, 0);
        for tick in 1..=1800 {
            game.step(0).unwrap();
            assert_eq!(
                game.items.slots[id].processor.is_some(),
                tick < 1800,
                "character {character} tick {tick}"
            );
        }
        assert_eq!(game.roster.get(dog).position, [999000; 3]);
        assert_ne!(game.roster.get(dog).attribute & ATTR_SUSPEND, 0);
        assert_eq!(game.player().health, game.player().max_health);
        let count = game.roster.order.len();
        game.step(TRIANGLE).unwrap();
        let id = game
            .items
            .slots
            .iter()
            .position(|i| i.processor == Some(Processor::Dog))
            .unwrap();
        for _ in 0..180 {
            if game.items.slots[id].mode == 2 {
                break;
            }
            game.step(0).unwrap();
        }
        assert_eq!(game.items.slots[id].mode, 2);
        assert_eq!(game.items.slots[id].dog.slave, Some(dog));
        assert_eq!(game.roster.order.len(), count);
        assert_eq!(game.player().inventory[16], 0);
    }
}
