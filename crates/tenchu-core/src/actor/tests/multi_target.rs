use super::*;

fn swing(game: &mut Game, source: usize, hand: usize) -> ModelId {
    with_actor(game, source, |actor, ctx, world| {
        actor.clear_attack(ctx, 3).unwrap();
        actor.enter(ctx, MOT_ATTACK, true);
        actor.attack = actor.attack_id(ctx, MOT_ATTACK);
        actor.motion.count = ctx.mission.weapons.attacks[actor.attack].start;
        actor.motion.loops = 0;
        actor.target = None;
        actor.pad.trigger = 0;
        actor.attack(ctx, world, 0).unwrap();
        let hands = [actor.parts[13], actor.parts[14]];
        ctx.collision.remove(ctx.scene, hands[1 - hand]);
        assert!(ctx.scene.models[hands[hand]].conflict.is_some());
        hands[hand]
    })
}

fn contact(game: &mut Game, victim: usize) -> bool {
    game.scene.finish_tick();
    game.collision.compute(&mut game.scene).unwrap();
    with_actor(game, victim, |actor, ctx, world| {
        actor.attribute &= !ATTR_HIT;
        actor.hit = None;
        let center = actor.collision_center(ctx);
        actor.body_contact(ctx, center).unwrap();
        let hit = actor.attribute & ATTR_HIT != 0;
        if hit {
            actor.damage_control(ctx, world).unwrap();
        }
        hit
    })
}

#[test]
#[ignore = "requires TENCHU_DISC; authored weapon hitboxes damage overlapping guards once per swing, respecting blocks"]
fn multi_target_melee_consumes_each_body_once_and_resets_for_the_next_swing() {
    for character in 0..2 {
        for hand in 0..=character as usize {
            for enabled in [false, true] {
                let mut game = oracle::game(0, character);
                let first = arena(&mut game, 0x10);
                let player = game.player_id;
                let second = with_actor(&mut game, player, |_, ctx, world| {
                    world.roster.spawn(0x10, Position::ZERO, 0, ctx).unwrap()
                });
                game.intelligence.difficulty = crate::ai::Difficulty::Easy;
                game.combat.multi_target = enabled;
                for blocked in [false, true] {
                    let weapon = swing(&mut game, player, hand);
                    game.scene.finish_tick();
                    game.collision.compute(&mut game.scene).unwrap();
                    let slot = game.scene.models[weapon].conflict.unwrap();
                    let target = game.collision.slots[slot].position;
                    for victim in [first, second] {
                        with_actor(&mut game, victim, |actor, ctx, _| {
                            actor.health = 999;
                            actor.max_health = 999;
                            actor.attribute = ATTR_WEAPON_DRAWN | 2;
                            actor.position = Position::ZERO;
                            actor.rotation = [0; 3];
                            actor.enter(ctx, MOT_ENGAGE_STANCE, true);
                            actor.motion.count = 0;
                            actor.motion.loops = 0;
                            actor.play_pose(ctx).unwrap();
                            ctx.scene.update(actor.root_model, actor.root(ctx.math));
                        });
                        for adjustment in 0..2 {
                            game.scene.finish_tick();
                            game.collision.compute(&mut game.scene).unwrap();
                            with_actor(&mut game, victim, |actor, ctx, _| {
                                let center = actor.collision_center(ctx);
                                actor.position = Position::from_fn(|i| {
                                    actor.position[i] + target[i] - center[i]
                                });
                                if adjustment == 0 {
                                    actor.rotation[1] = ctx
                                        .math
                                        .direction(-actor.position.x, -actor.position.z, 0)
                                        .unwrap();
                                }
                                ctx.scene.update(actor.root_model, actor.root(ctx.math));
                                if blocked && victim == first {
                                    actor.enter(ctx, MOT_CHASE_BACK, true);
                                }
                            });
                        }
                    }
                    assert!(
                        contact(&mut game, first),
                        "character {character}, hand {hand}, enabled {enabled}, blocked {blocked}"
                    );
                    assert_eq!(contact(&mut game, second), enabled);
                    let health = [first, second].map(|id| game.roster.get(id).health);
                    assert_eq!(health[0] == 999, blocked);
                    assert_eq!(health[1] < 999, enabled);
                    let calls = game.random.calls;
                    for _ in 0..10 {
                        assert!(!contact(&mut game, first));
                        assert!(!contact(&mut game, second));
                    }
                    assert_eq!([first, second].map(|id| game.roster.get(id).health), health);
                    assert_eq!(
                        game.random.calls, calls,
                        "repeated contacts emitted effects"
                    );
                }
                let weapon = swing(&mut game, first, 0);
                let slot = game.scene.models[weapon].conflict.unwrap();
                assert!(!game.collision.slots[slot].multi_target);
                assert!(game.collision.slots[slot].hit_targets.is_empty());
            }
        }
    }
}
