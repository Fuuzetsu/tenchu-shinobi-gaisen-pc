use super::*;
use crate::ai::Halt;

#[test]
#[ignore = "requires TENCHU_DISC; Mission 4 corpse discovery during a witnessed stealth kill"]
fn checkpoint_corpse_alarm_is_distinct_from_spotting_the_player() {
    for all_active in [false, true] {
        for discoverable in [false, true] {
            let mut game = oracle::game(9, 1);
            let player = game.player_id;
            let victim = 1;
            let position = Position::new(-26750, -7000, 149250);
            game.roster.keep_all_active = all_active;
            game.teleport_player(position, true).unwrap();
            with_actor(&mut game, player, |actor, ctx, _| {
                actor.rotation = [0; 3];
                actor.enter(ctx, MOT_ENGAGE_STANCE, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            with_actor(&mut game, victim, |actor, ctx, _| {
                actor.position = position.wrapping_sub(Position::new(0, 0, 700));
                actor.rotation = [0; 3];
                actor.attribute = ATTR_CUSTOMAI;
                actor.health = actor.max_health;
                actor.inventory.fill(0);
                actor.enter(ctx, MOT_NORMAL, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            game.step(crate::input::SQUARE).unwrap();
            assert_eq!(game.statistics.criticals, 1);
            assert_eq!(game.player().motion.action, MOT_ATTACK_STEALTH_BACK + 3);
            let mut halted = false;
            let mut discovered_during_animation = false;
            for _ in 0..150 {
                if !discoverable {
                    // Isolate ordinary player sight by marking this body already discovered.
                    game.roster.get_mut(victim).attribute |= ATTR_SEARCH;
                }
                game.step(0).unwrap();
                if game.intelligence.action_halt == Halt::Active {
                    halted = true;
                    discovered_during_animation |= game.intelligence.alert != 0;
                } else if halted {
                    break;
                }
            }
            assert!(halted);
            assert_eq!(game.intelligence.action_halt, Halt::Running);
            assert_eq!(discovered_during_animation, discoverable);
            assert_eq!(game.intelligence.found, 0);
            assert_eq!(game.intelligence.alert, if discoverable { 300 } else { 0 });
            let suspicious = game
                .roster
                .order
                .iter()
                .map(|&id| game.roster.get(id))
                .filter(|actor| {
                    (0x10..0x80).contains(&actor.kind)
                        && actor.id != victim
                        && actor.attribute & ATTR_PHASE == 1
                })
                .count();
            if discoverable {
                assert!(suspicious >= 2, "all-active: {all_active}");
            } else {
                assert_eq!(suspicious, 0, "all-active: {all_active}");
            }
        }
    }
}
