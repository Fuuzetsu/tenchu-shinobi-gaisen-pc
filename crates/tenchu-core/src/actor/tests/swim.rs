use super::*;
use crate::math::Position;
use crate::{camera::Mode, effects::Effect, input::*};

fn hidden_body(game: &Game, hidden: bool) {
    for (i, &part) in game.player().parts.iter().enumerate() {
        assert_eq!(
            game.scene.models[part].attribute & 1 != 0,
            hidden && (i == 0 || (7..=12).contains(&i)),
            "part {i}"
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; water entry cleans attacks once and preserves splash RNG order"]
fn water_entry_cleans_attack_and_emits_one_splash_batch() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    arena(&mut game, 0x10);
    game.effects = crate::effects::Effects::default();
    let mut random = game.random;
    with_actor(&mut game, player, |actor, ctx, world| {
        actor.enter(ctx, MOT_ATTACK, true);
        actor.map.attribute = 4;
        actor.map.height = 1;
        assert!(!actor.swim_check(ctx, world).unwrap());
        actor.map.height = 0;
        actor.map.level = 300;
        actor.position = Position::new(100, 300, 100);
        let waist = ctx.scene.models[actor.parts[0]].conflict.unwrap();
        ctx.collision.slots[waist].position = Position::new(200, 0, -200);
        for hand in [actor.parts[13], actor.parts[14]] {
            ctx.collision.insert(ctx.scene, hand).unwrap();
        }
        world.intelligence.action_halt = crate::ai::Halt::Active;
        assert!(actor.swim_check(ctx, world).unwrap());
        assert_eq!(actor.position, [200, 300, -200]);
        assert_eq!(actor.motion.action, MOT_SWIM);
        assert_eq!(actor.motion.mask, 0x7fff);
        assert_eq!(world.intelligence.action_halt, crate::ai::Halt::Running);
        assert_eq!(world.intelligence.alert, 300);
        for hand in [actor.parts[13], actor.parts[14]] {
            assert!(ctx.scene.models[hand].conflict.is_none());
        }
        let width = i32::from(ctx.mission.characters[actor.definition].width);
        for effect in ctx.effects.slots.iter().flatten() {
            let Effect::Splash(splash) = effect else {
                panic!("Missing entry splash")
            };
            assert_eq!(
                splash.position,
                [
                    200 + random.next15() % width * 2 - width,
                    300,
                    -200 + random.next15() % width * 2 - width,
                ]
            );
            assert_eq!(
                splash.size,
                std::array::from_fn(|_| ((random.next15() & 7) << 12) as i16)
            );
            assert_eq!(splash.speed, 6);
        }
        assert!(actor.swim_check(ctx, world).unwrap());
        assert_eq!(*ctx.random, random);
        assert_eq!(ctx.effects.slots.iter().flatten().count(), 20);
        assert_eq!(ctx.effects.sounds, [(Position::new(200, 300, -200), 0x16)]);
    });
    assert_eq!(game.camera.mode, Mode::Swim);
    hidden_body(&game, true);
}

#[test]
#[ignore = "requires TENCHU_DISC; both players swim, reverse, cancel a hook and leave the water through ordinary ticks"]
fn swimming_input_hook_and_shore_exit() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        let player = game.player_id;
        with_actor(&mut game, player, |_, ctx, world| {
            world.roster.remove(enemy, ctx)
        });
        oracle::flat_terrain(&mut game, 0x2004);
        game.camera.snap_pending = true;
        game.roster.get_mut(player).inventory[0] = 255;
        game.roster.get_mut(player).inventory[3] = 2;
        game.hud.cursor = 3;
        game.step(0).unwrap();
        assert_eq!(game.player().motion.action, MOT_SWIM);
        hidden_body(&game, true);
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_SWIM);
        assert_eq!(game.player().inventory[3], 2);
        for _ in 0..24 {
            game.step(UP).unwrap();
        }
        assert_eq!(game.player().motion.action, MOT_SWIM_STROKE);
        assert_eq!(game.player().velocity, [0, 0, 60]);
        let yaw = game.player().rotation[1];
        let turn = game.mission.characters[game.player().definition].turn;
        for _ in 0..8 {
            game.step(DOWN | RIGHT).unwrap();
        }
        assert_eq!(game.player().rotation[1], yaw - 8 * turn);
        assert!(game.player().velocity[2] < 0);
        game.step(L1 | UP).unwrap();
        assert_eq!(game.player().motion.action, MOT_SWIM);
        assert_eq!(game.player().velocity, [0; 3]);
        game.hud.cursor = 0;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_KAGI);
        assert_eq!(game.player().motion.mask, !1);
        hidden_body(&game, false);
        game.step(TRIANGLE).unwrap();
        assert!(game.player().aiming);
        game.step(TRIANGLE | CIRCLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_SWIM);
        assert_eq!(game.player().motion.mask, 0x7fff);
        hidden_body(&game, true);
        game.step(0).unwrap();
        oracle::flat_ground(&mut game);
        game.step(UP).unwrap();
        assert_eq!(game.player().motion.action, MOT_SWIM_EXIT);
        game.step(0).unwrap();
        hidden_body(&game, false);
        for _ in 0..100 {
            game.step(0).unwrap();
            if game.player().status != 3 {
                break;
            }
        }
        assert!(matches!(
            game.player().motion.action,
            MOT_NORMAL | MOT_ENGAGE_STANCE
        ));
        assert_eq!(game.camera.mode, Mode::Normal);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; an actor without swimming motion drowns and loses body collision"]
fn non_swimmer_finishes_drowning() {
    let mut game = oracle::game(0, 0);
    let enemy = arena(&mut game, 0x10);
    oracle::flat_terrain(&mut game, 0x2004);
    game.camera.snap_pending = true;
    game.step(0).unwrap();
    assert_eq!(game.roster.get(enemy).motion.action, MOT_DEAD_DROWN);
    assert_eq!(game.roster.get(enemy).health, 0);
    for _ in 0..240 {
        game.step(0).unwrap();
    }
    let actor = game.roster.get(enemy);
    assert_eq!(actor.motion.loops, -1);
    assert!(game.scene.models[actor.parts[0]].conflict.is_none());
    assert_ne!(game.scene.models[actor.root_model].attribute & 1, 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; leaving an attack over a drop removes its live hand collisions"]
fn falling_clears_attack_hitboxes() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    arena(&mut game, 0x10);
    with_actor(&mut game, player, |actor, ctx, world| {
        actor.enter(ctx, MOT_ATTACK, true);
        actor.map.height = 2000;
        actor.attribute &= !ATTR_HIT;
        for hand in [actor.parts[13], actor.parts[14]] {
            ctx.collision.insert(ctx.scene, hand).unwrap();
        }
        actor.control(ctx, &[], Position::ZERO, world).unwrap();
        assert_eq!(actor.motion.action, MOT_STATE_FALL);
        for hand in [actor.parts[13], actor.parts[14]] {
            assert!(ctx.scene.models[hand].conflict.is_none());
        }
    });
}
