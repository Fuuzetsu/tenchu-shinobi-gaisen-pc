use super::*;
use crate::ai::Halt;

#[test]
#[ignore = "requires TENCHU_DISC; keep retail's nearby wake-up without waking distant all-active sleepers"]
fn stealth_kill_turning_does_not_wake_sleepers_across_the_map() {
    for all_active in [false, true] {
        for nearby in [false, true] {
            let mut game = oracle::game(0, 1);
            game.roster.keep_all_active = all_active;
            let player = game.player_id;
            let sleeper = 5;
            let victim = 19;
            let position = if nearby {
                Position::new(28000, 0, 18000)
            } else {
                Position::new(18521, 0, 27572)
            };
            with_actor(&mut game, player, |_, ctx, world| {
                for id in world.roster.order.clone() {
                    if ![player, sleeper, victim].contains(&id) {
                        world.roster.remove(id, ctx);
                    }
                }
            });
            with_actor(&mut game, victim, |actor, ctx, _| {
                actor.position = Position::new(-40000, 0, -30000);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            game.teleport_player(position, true).unwrap();
            for _ in 0..90 {
                game.step(crate::input::CIRCLE).unwrap();
            }
            if nearby || all_active {
                assert_eq!(game.roster.get(sleeper).motion.action, MOT_ACTION);
                assert_eq!(game.roster.get(sleeper).motion.loops, -1);
            } else {
                assert_ne!(game.roster.get(sleeper).attribute & ATTR_SUSPEND, 0);
            }
            with_actor(&mut game, player, |actor, ctx, _| {
                actor.position = position;
                actor.rotation = [0, 3072, 0];
                actor.velocity = [0; 3];
                actor.enter(ctx, MOT_ENGAGE_STANCE, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            with_actor(&mut game, victim, |actor, ctx, _| {
                actor.position = position.wrapping_add(Position::new(700, 0, 0));
                actor.rotation = [0, 3072, 0];
                actor.velocity = [0; 3];
                actor.attribute = ATTR_CUSTOMAI;
                actor.inventory.fill(0);
                actor.enter(ctx, MOT_NORMAL, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            game.step(crate::input::SQUARE).unwrap();
            assert_eq!(game.statistics.criticals, 1);
            let mut woke = false;
            let mut halted = false;
            for _ in 0..150 {
                // Isolate the turn/get-up sequence from the separate corpse-discovery alarm.
                game.roster.get_mut(victim).attribute |= ATTR_SEARCH;
                game.step(crate::input::CIRCLE).unwrap();
                woke |= game.roster.get(sleeper).motion.action == MOT_DAMAGE_GETUP;
                if game.intelligence.action_halt == Halt::Active {
                    halted = true;
                } else if halted {
                    break;
                }
            }
            assert!(halted);
            assert_eq!(game.intelligence.action_halt, Halt::Running);
            assert_eq!(woke, nearby, "all-active: {all_active}, nearby: {nearby}");
            assert_eq!(game.intelligence.found, 0);
            assert_eq!(game.intelligence.alert, 0);
            if !nearby {
                assert_eq!(game.roster.get(sleeper).attribute & ATTR_PHASE, 0);
            }
        }
    }
}

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

#[test]
#[ignore = "requires TENCHU_DISC; Mission 4 cover must hide bodies without suppressing open-ground discovery"]
fn strict_corpse_sight_requires_an_eye_to_body_passage() {
    for strict in [false, true] {
        for covered in [false, true] {
            let mut game = oracle::game(9, 1);
            let victim = 1;
            let witness = 15;
            let (body, observer) = if covered {
                (
                    Position::new(-26882, -7000, 148810),
                    Position::new(-22894, -7000, 155067),
                )
            } else {
                oracle::flat_ground(&mut game);
                (Position::ZERO, Position::new(0, 0, 7000))
            };
            game.intelligence.strict_corpse_sight = strict;
            game.intelligence.alert = 0;
            game.intelligence.found = 0;
            let index = game
                .roster
                .order
                .iter()
                .position(|&id| id == witness)
                .unwrap();
            game.intelligence.death_index =
                (index + game.roster.order.len() - 1) % game.roster.order.len();
            with_actor(&mut game, witness, |actor, ctx, _| {
                actor.position = observer;
                let delta = body.wrapping_sub(observer);
                actor.rotation[1] = ctx.math.atan2(-delta.x, -delta.z).unwrap();
                actor.attribute = ATTR_CUSTOMAI;
                actor.enter(ctx, MOT_NORMAL, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            with_actor(&mut game, victim, |actor, ctx, world| {
                actor.position = body;
                actor.rotation = [0; 3];
                actor.velocity = [0; 3];
                actor.attribute = ATTR_CUSTOMAI;
                actor.health = 0;
                actor.enter(ctx, MOT_DEAD, true);
                actor.motion.count = ctx.mission.clips[actor.motion.clip.unwrap()].duration;
                actor.play_pose(ctx).unwrap();
                actor.motion.loops = 0;
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
                actor.corpse(ctx, world).unwrap();
            });
            let discovered = !strict || !covered;
            assert_eq!(
                game.roster.get(victim).attribute & ATTR_SEARCH != 0,
                discovered,
                "strict {strict}, covered {covered}"
            );
            assert_eq!(game.intelligence.alert, if discovered { 300 } else { 0 });
            assert_eq!(game.intelligence.found, 0);
            assert_eq!(
                game.roster.get(witness).motion.action == MOT_STATE_DRAW,
                discovered
            );
        }
    }
}
