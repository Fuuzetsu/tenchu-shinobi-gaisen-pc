use super::*;
use crate::{
    input::{CIRCLE, TRIANGLE, UP},
    items::Processor,
};

#[test]
#[ignore = "requires TENCHU_DISC; both characters retain their identity while disguised, use changed body offsets, and restore after 600 ticks while crouching"]
fn disguise_cast_expiry_and_render_phase_preserve_player_identity() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        game.camera.snap_pending = true;
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, world| {
            world.roster.remove(enemy, ctx);
            actor.inventory[11] = 2;
        });
        let definition = game.player().definition;
        game.hud.cursor = 11;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_ITEM_KAENGEKI);
        let id = game
            .items
            .slots
            .iter()
            .position(|i| i.processor == Some(Processor::Disguise))
            .unwrap();
        for _ in 0..120 {
            if game.items.slots[id].mode == 3 {
                break;
            }
            game.step(0).unwrap();
        }
        assert_eq!(game.items.slots[id].mode, 3);
        assert_eq!(game.player().inventory[11], 1);
        assert_eq!(game.player().kind, character);
        assert_eq!(game.player().definition, definition);
        assert_eq!(game.player().mesh_kind, game.mission.disguise);
        assert_eq!(game.player().rendered_mesh_kind, game.mission.disguise);
        assert_eq!(game.player().active_item, 11);
        assert_eq!(game.intelligence.alert, -600);
        let target = &game.mission.models[&game.mission.disguise];
        assert_eq!(game.player().motion.pose.root_height, target[0].position[1]);
        for (&model, part) in game.player().parts.iter().zip(target) {
            assert_eq!(
                game.scene.models[model].local.translation,
                part.position.map(i32::from)
            );
        }
        let start = game.player().position;
        for tick in 1..=600 {
            game.step(if tick <= 30 {
                UP
            } else if tick >= 590 {
                CIRCLE
            } else {
                0
            })
            .unwrap();
            assert_eq!(game.items.slots[id].processor.is_some(), tick < 600);
            assert_eq!(game.player().active_item, if tick < 600 { 11 } else { 0 });
            assert_eq!(
                game.player().rendered_mesh_kind,
                if tick < 600 {
                    game.mission.disguise
                } else {
                    character
                }
            );
        }
        assert_eq!(game.player().mesh_kind, character);
        assert_ne!(game.player().status, 11);
        assert_ne!(game.player().position, start);
        assert_eq!(
            game.player().motion.pose.root_height,
            game.mission.models[&character][0].position[1]
        );
        game.step(0).unwrap();
        assert_eq!(game.player().rendered_mesh_kind, character);
        assert_eq!(game.player().health, game.player().max_health);
    }
}
