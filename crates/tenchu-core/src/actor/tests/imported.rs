use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; foreign reinforcements preserve their body, HP and voice and attack through ordinary AI ticks"]
fn foreign_reinforcements_spawn_and_land_ai_attacks() {
    let mut game = oracle::game(0, 1);
    let donor = oracle::game(6, 1);
    let ordinary = arena(&mut game, 0x10);
    let definition = donor
        .mission
        .characters
        .iter()
        .find(|c| c.kind == 0x87)
        .unwrap()
        .clone();
    let source = game
        .mission
        .import_actor(
            definition,
            donor.mission.models[&0x87].clone(),
            &donor.mission.clips,
            std::sync::Arc::new(donor.mission.vram),
        )
        .unwrap();
    game.mission.reinforcement_overrides = [Some(tenchu_assets::stage::imports::Reinforcement {
        actor: Some(source),
        voice: Some(128),
        health_percent: 175,
    }); 2];
    let player = game.player_id;
    with_actor(&mut game, player, |actor, ctx, world| {
        world.roster.remove(ordinary, ctx);
        actor.health = 999;
        actor.max_health = 999;
        for slot in 0..2 {
            let id = world
                .roster
                .spawn_reinforcement(slot, Position::new(0, 0, 2000 + slot as i32 * 300), 0, ctx)
                .unwrap();
            let enemy = world.roster.get_mut(id);
            assert_eq!(enemy.kind, 0x87);
            assert_eq!(enemy.script_kind, Some(ctx.mission.reinforcements[slot]));
            assert_eq!((enemy.health, enemy.max_health), (350, 350));
            assert_eq!(enemy.voice_program, Some(128));
            enemy.setup_think(0x4444).unwrap();
            enemy.target = Some(actor.root_model);
        }
    });
    for _ in 0..600 {
        game.step(0).unwrap();
    }
    assert!(
        game.player().health < 999,
        "Imported reinforcement AI never landed a hit"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; imported sword, fist and jaw attacks must retain their authored damage windows and hit both protagonists"]
fn imported_melee_attacks_damage_both_players_like_their_originals() {
    for character in 0..2 {
        for (stage, kind) in [(0, 0x10), (6, 0x87), (2, 0xa4)] {
            let mut damaged = false;
            for distance in [1000, 2000, 3000] {
                let mut remaining = Vec::new();
                for imported in [false, true] {
                    let mut game = oracle::game(if imported { 0 } else { stage }, character);
                    let enemy = arena(&mut game, if imported { 0x10 } else { kind });
                    if imported {
                        let donor = oracle::game(stage, character);
                        let definition = donor
                            .mission
                            .characters
                            .iter()
                            .find(|c| c.kind == kind)
                            .unwrap()
                            .clone();
                        let source = game
                            .mission
                            .import_actor(
                                definition,
                                donor.mission.models[&kind].clone(),
                                &donor.mission.clips,
                                std::sync::Arc::new(donor.mission.vram),
                            )
                            .unwrap();
                        game.replace_actor(enemy, source, 0).unwrap();
                    }
                    with_actor(&mut game, enemy, |actor, ctx, _| {
                        actor.attribute = 0;
                        actor.equip(ctx, true);
                        actor.think = crate::ai::programs::Programs::default();
                        actor.position = Position::new(0, 0, distance);
                        actor.rotation = [0; 3];
                        assert!(actor.enter(ctx, MOT_ATTACK, true));
                        let attack = actor.attack_id(ctx, MOT_ATTACK);
                        assert_eq!(
                            ctx.mission.weapons.attacks[attack].clip,
                            actor.motion_id(ctx, MOT_ATTACK)
                        );
                        ctx.scene.update(actor.root_model, actor.root(ctx.math));
                    });
                    for _ in 0..120 {
                        game.step(0).unwrap();
                    }
                    remaining.push(game.player().health);
                }
                assert_eq!(
                    remaining[0], remaining[1],
                    "character {character}, kind {kind:x}, distance {distance}"
                );
                damaged |= remaining[1] < 100;
            }
            assert!(
                damaged,
                "character {character}, kind {kind:x} never landed a hit"
            );
        }
    }
}
