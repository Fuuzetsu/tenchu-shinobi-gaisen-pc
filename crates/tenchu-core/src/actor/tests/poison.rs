use super::*;
use crate::{input::TRIANGLE, items::Processor};

#[test]
#[ignore = "requires TENCHU_DISC; both players throw bait, lure a guard and complete eating/poison expiry"]
fn poisoned_food_lures_and_restores_an_enemy() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        let player = game.player_id;
        with_actor(&mut game, enemy, |actor, ctx, _| {
            actor.position[2] = 9000;
            actor.rotation[1] = 2048;
            actor.attribute = 0;
            actor.think = [0; 4];
            actor.inventory.fill(0);
            actor.enter(ctx, MOT_NORMAL, true);
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
        });
        game.camera.snap_pending = true;
        game.roster.get_mut(player).inventory[7] = 2;
        game.hud.cursor = 7;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_ITEM_THROW);
        assert_eq!(game.player().inventory[7], 1);
        let health = game.roster.get(enemy).health;
        let mut lured = false;
        let mut eating = false;
        let mut poisoned = false;
        let mut choked = false;
        for _ in 0..1000 {
            game.step(0).unwrap();
            let actor = game.roster.get(enemy);
            lured |= actor.think[0] == 9;
            eating |= actor.motion.action == MOT_ITEM_DRINK;
            choked |= actor.status == 15 && actor.motion.action == MOT_DAMAGE_CHOKE;
            poisoned |= game
                .items
                .slots
                .iter()
                .any(|i| i.processor == Some(Processor::Poison) && i.mode == 3);
        }
        assert!(
            lured && eating && poisoned && choked,
            "character {character}: lure {lured}, eat {eating}, poison {poisoned}, choke {choked}; enemy {:?}",
            game.roster.get(enemy)
        );
        assert_eq!(game.roster.get(enemy).think[0], 0);
        assert_eq!(
            game.roster.get(enemy).target,
            Some(game.player().root_model)
        );
        assert_eq!(game.roster.get(enemy).health, health);
        assert!(
            !game
                .items
                .slots
                .iter()
                .any(|i| i.processor == Some(Processor::Poison))
        );
    }
}
