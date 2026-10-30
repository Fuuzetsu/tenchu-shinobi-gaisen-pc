use super::*;
use crate::{input::TRIANGLE, items::Processor};

#[test]
#[ignore = "requires TENCHU_DISC; both characters fire eight independently trailed darts through arming, damage and disposal"]
fn both_players_fire_eight_darts_with_delayed_hitboxes() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        let player = game.player_id;
        with_actor(&mut game, enemy, |actor, ctx, _| {
            actor.attribute = ATTR_WEAPON_DRAWN;
            actor.think = crate::ai::programs::Programs::default();
            actor.inventory.fill(0);
            actor.enter(ctx, MOT_ENGAGE_STANCE, true);
        });
        game.roster.get_mut(player).inventory[15] = 2;
        game.hud.cursor = 15;
        game.step(0).unwrap();
        let health = game.roster.get(enemy).health;
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().inventory[15], 1);
        assert_eq!(game.player().motion.action, MOT_ITEM_THROW);
        let ids: Vec<_> = game
            .items
            .slots
            .iter()
            .enumerate()
            .filter_map(|(id, item)| (item.processor() == Some(Processor::Darts)).then_some(id))
            .collect();
        assert_eq!(ids.len(), 8);
        for &id in &ids {
            let item = &game.items.slots[id];
            assert_eq!(item.trail.as_ref().unwrap().parent, item.model);
            assert_eq!(item.count, 7);
            assert!(game.scene.models[item.model].conflict.is_none());
        }
        for _ in 0..6 {
            game.step(0).unwrap();
            for &id in &ids {
                assert!(
                    game.scene.models[game.items.slots[id].model]
                        .conflict
                        .is_none()
                );
            }
        }
        game.step(0).unwrap();
        for &id in &ids {
            assert_eq!(game.items.slots[id].count, 0);
            assert!(
                game.scene.models[game.items.slots[id].model]
                    .conflict
                    .is_some()
            );
        }
        for _ in 0..300 {
            game.step(0).unwrap();
        }
        assert!(
            game.roster.get(enemy).health < health,
            "character {character}"
        );
        assert!(
            game.items
                .slots
                .iter()
                .all(|i| i.processor() != Some(Processor::Darts))
        );
    }
}
