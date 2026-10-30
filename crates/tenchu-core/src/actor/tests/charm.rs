use super::*;
use crate::{
    input::{TRIANGLE, UP},
    items::Processor,
};

#[test]
#[ignore = "requires TENCHU_DISC; both characters cast a protection charm, move during its 450-tick lifetime and recover normally"]
fn charm_casting_and_duration_allow_normal_play() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, world| {
            world.roster.remove(enemy, ctx);
            actor.inventory[12] = 2;
        });
        game.hud.cursor = 12;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_ITEM_KAENGEKI);
        assert_eq!(game.player().inventory[12], 1);
        let id = game
            .items
            .slots
            .iter()
            .position(|i| i.processor() == Some(Processor::Charm))
            .unwrap();
        for _ in 0..120 {
            if game.items.slots[id].source_mode() == 2 {
                break;
            }
            game.step(0).unwrap();
        }
        assert_eq!(game.items.slots[id].source_mode(), 2);
        assert_eq!(game.items.slots[id].charm_remaining, 450);
        assert_eq!(game.player().active_item, 12);
        assert_ne!(game.player().motion.action, MOT_ITEM_KAENGEKI);
        let start = game.player().position;
        for tick in 1..=450 {
            game.step(if tick <= 30 { UP } else { 0 }).unwrap();
            assert_eq!(game.player().active_item, if tick < 450 { 12 } else { 0 });
            assert_eq!(game.items.slots[id].processor().is_some(), tick < 450);
            assert_eq!(game.items.slots[id].charm_remaining, 450 - tick);
        }
        assert_ne!(game.player().position, start);
        assert_eq!(game.player().health, game.player().max_health);
    }
}
