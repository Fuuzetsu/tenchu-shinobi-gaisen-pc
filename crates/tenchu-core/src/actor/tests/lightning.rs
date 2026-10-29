use super::*;
use crate::items::Processor;
use crate::math::Position;

#[test]
#[ignore = "requires TENCHU_DISC; all four Mei-Oh attacks launch on frame thirteen and damage both characters through ordinary collision ticks"]
fn mei_oh_lightning_attacks_launch_restrike_damage_and_expire() {
    for (character, imported) in [(0, false), (1, false), (0, true), (1, true)] {
        for action in [
            MOT_ATTACK,
            MOT_ATTACK_SLASH2,
            MOT_ATTACK_SLASH3,
            MOT_ATTACK_SLASH4,
        ] {
            let mut game = oracle::game(if imported { 0 } else { 7 }, character);
            let shooter = if imported {
                let donor = oracle::game(7, character);
                let definition = donor
                    .mission
                    .characters
                    .iter()
                    .find(|c| c.kind == 0x88)
                    .unwrap()
                    .clone();
                let guard = game
                    .roster
                    .order
                    .iter()
                    .map(|&id| game.roster.get(id).kind)
                    .find(|&kind| (0x10..0x80).contains(&kind))
                    .unwrap();
                let shooter = arena(&mut game, guard);
                let source = game
                    .mission
                    .import_actor(
                        definition,
                        donor.mission.models[&0x88].clone(),
                        &donor.mission.clips,
                        std::sync::Arc::new(donor.mission.vram),
                    )
                    .unwrap();
                game.replace_actor(shooter, source, 0).unwrap();
                shooter
            } else {
                arena(&mut game, 0x88)
            };
            game.camera.snap_pending = true;
            with_actor(&mut game, shooter, |actor, ctx, _| {
                actor.attribute = ATTR_WEAPON_DRAWN;
                actor.think = [0; 4];
                actor.position = Position::new(0, 0, 2300);
                assert!(actor.enter(ctx, action, true));
                assert_eq!(actor.motion_id(ctx, MOT_ATTACK), 0xe9);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            let mut started = None;
            let mut active_ticks = 0;
            let mut drawn = false;
            for tick in 0..180 {
                let was = game.roster.get(shooter).motion.count;
                game.step(0).unwrap_or_else(|e| {
                    panic!("character {character} action {action:x} tick {tick}: {e}")
                });
                if game
                    .items
                    .slots
                    .iter()
                    .any(|i| i.processor == Some(Processor::Lightning))
                {
                    if started.is_none() {
                        assert_eq!(was, 13);
                        started = Some(tick);
                    }
                    active_ticks += 1;
                }
                drawn |= !game.effects.lines.is_empty();
            }
            assert!(started.is_some());
            assert_eq!(active_ticks, 15);
            assert!(drawn);
            assert!(
                game.player().health < game.player().max_health,
                "character {character} action {action:x}: lightning missed"
            );
            assert!(
                !game
                    .items
                    .slots
                    .iter()
                    .any(|i| i.processor == Some(Processor::Lightning))
            );
        }
    }
}
