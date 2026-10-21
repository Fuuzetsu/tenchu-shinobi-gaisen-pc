use super::*;
use crate::{effects::Effect, input::TRIANGLE, items::Processor};

#[test]
#[ignore = "requires TENCHU_DISC; smoke, grenades and mines through input, animation, contact and damage"]
fn both_players_use_smoke_grenades_and_mines() {
    for character in 0..2 {
        for kind in [4, 5, 6] {
            let mut game = oracle::game(0, character);
            let enemy = arena(&mut game, 0x10);
            let player = game.player_id;
            with_actor(&mut game, enemy, |actor, ctx, _| {
                actor.attribute = ATTR_WEAPON_DRAWN;
                actor.think = [0; 4];
                actor.inventory.fill(0);
                actor.position.z = 2200;
                actor.enter(ctx, MOT_ENGAGE_STANCE, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            game.camera.snap_pending = true;
            game.roster.get_mut(player).inventory[kind] = 2;
            game.hud.cursor = kind;
            game.step(0).unwrap();
            game.step(TRIANGLE).unwrap();
            assert_eq!(
                game.player().motion.action,
                if kind == 6 {
                    MOT_ITEM_PLANT
                } else {
                    MOT_ITEM_THROW
                }
            );
            let processor = match kind {
                4 => Processor::Fire,
                5 => Processor::Smoke,
                _ => Processor::Mine,
            };
            let health = game.roster.get(enemy).health;
            let mut spawned = false;
            let mut choked = false;
            let mut smoke = false;
            let mut exploded = false;
            let mut ember = false;
            for _ in 0..360 {
                game.step(0).unwrap();
                if !spawned
                    && let Some(item) = game
                        .items
                        .slots
                        .iter()
                        .find(|i| i.processor == Some(processor))
                {
                    let position = game.scene.models[item.model].local.translation;
                    spawned = true;
                    if kind == 6 {
                        with_actor(&mut game, enemy, |actor, ctx, _| {
                            actor.position = position;
                            actor.velocity = [0; 3];
                            ctx.scene.update(actor.root_model, actor.root(ctx.math));
                        });
                    }
                }
                choked |= game.roster.get(enemy).motion.action == MOT_DAMAGE_CHOKE;
                smoke |= game
                    .effects
                    .slots
                    .iter()
                    .any(|e| matches!(e, Some(Effect::Smoke(_))));
                exploded |= game.effects.sounds.iter().any(|(_, sound)| *sound == 0x25);
                ember |= game
                    .effects
                    .slots
                    .iter()
                    .any(|e| matches!(e, Some(Effect::Ember(_))));
            }
            assert!(spawned && smoke, "character {character} kind {kind}");
            assert_eq!(game.player().inventory[kind], 1);
            assert!(
                !game
                    .items
                    .slots
                    .iter()
                    .any(|i| i.processor == Some(processor))
            );
            if kind == 5 {
                assert!(choked);
                assert_eq!(game.roster.get(enemy).health, health);
                assert!(!exploded && !ember);
                assert_eq!(game.player().health, game.player().max_health);
            } else {
                assert!(exploded && ember, "character {character} kind {kind}");
                assert!(
                    game.roster.get(enemy).health < health,
                    "character {character} kind {kind}"
                );
            }
        }
    }
}
