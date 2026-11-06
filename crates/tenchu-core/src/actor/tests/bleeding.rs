use super::*;
use crate::game::bleeding::Settings;

fn enabled() -> Settings {
    Settings {
        enabled: true,
        chance: 100,
        ..Default::default()
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; both heroes select and consume bandages through ordinary item buttons"]
fn bandages_work_as_carried_items_without_replacing_retail_slots() {
    use crate::input::{L2, R2, TRIANGLE};
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        game.set_bleeding(Settings {
            damage: 0,
            ..enabled()
        });
        let player = game.roster.get_mut(game.player_id);
        player.inventory.fill(0);
        player.inventory[0] = 255;
        for _ in 0..10 {
            game.step(0).unwrap();
        }
        game.step(R2).unwrap();
        assert!(game.hud.bandage_selected());
        assert_eq!(game.hud.selected, None);
        assert_eq!(game.hud.bandage_position(game.player()), Some([120, 92]));
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        assert_eq!(
            game.player().bleeding.as_ref().unwrap().bandages,
            3,
            "healthy use must not waste supplies"
        );
        game.step(0).unwrap();
        let player = game.roster.get_mut(game.player_id);
        player.health = 80;
        player.bleeding.as_mut().unwrap().hit();
        game.step(TRIANGLE).unwrap();
        assert!(!game.player().bleeding.as_ref().unwrap().active);
        assert_eq!(game.player().bleeding.as_ref().unwrap().bandages, 2);
        assert_eq!(game.player().health, 80);
        assert_eq!(game.player().inventory[0], 255);
        game.roster
            .get_mut(game.player_id)
            .bleeding
            .as_mut()
            .unwrap()
            .hit();
        for _ in 0..10 {
            game.step(TRIANGLE).unwrap();
        }
        assert!(
            game.player().bleeding.as_ref().unwrap().active,
            "holding use must not treat a new wound"
        );
        assert_eq!(game.player().bleeding.as_ref().unwrap().bandages, 2);
        game.step(0).unwrap();
        game.step(L2).unwrap();
        assert_eq!(game.hud.selected, Some(Kind::Grapple));
        game.step(0).unwrap();
        game.step(R2).unwrap();
        assert!(game.hud.bandage_selected());
        game.set_bleeding(Settings::default());
        game.step(0).unwrap();
        assert_eq!(game.hud.selected, Some(Kind::Grapple));
        assert_eq!(game.hud.bandage_position(game.player()), None);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; real melee wounds must not alter attack timing or consume gameplay RNG"]
fn bleeding_melee_has_independent_rng_and_bounded_damage() {
    let run = |character, chance, invulnerable| {
        let mut game = oracle::game(0, character);
        let guard = arena(&mut game, 0x10);
        let player = game.roster.get_mut(game.player_id);
        player.health = 9999;
        player.max_health = 9999;
        player.invulnerable = invulnerable;
        game.roster.get_mut(guard).setup_think(0x4444).unwrap();
        game.set_bleeding(Settings {
            chance,
            ..enabled()
        });
        let mut frames = Vec::new();
        let mut seen = false;
        for _ in 0..600 {
            game.step(0).unwrap();
            seen |= game.player().bleeding.as_ref().unwrap().active;
            frames.push((
                game.random,
                game.player().position,
                game.player().motion.action,
                game.player().motion.count,
                game.roster.get(guard).motion.action,
            ));
        }
        (frames, game.player().health, seen)
    };
    for character in 0..2 {
        let control = run(character, 0, false);
        let bleeding = run(character, 100, false);
        assert!(!control.2 && bleeding.2);
        assert_eq!(bleeding.0, control.0);
        assert!(bleeding.1 < control.1 && bleeding.1 >= control.1 - 20);
        let immune = run(character, 100, true);
        assert_eq!(immune.1, 9999);
        assert!(!immune.2);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; treatment, refresh, expiry, scripted pauses and 1 HP floor"]
fn bleeding_status_treatment_and_expiry() {
    let mut game = oracle::game(0, 1);
    arena(&mut game, 0x10);
    game.set_bleeding(Settings {
        seconds: 2,
        damage: 3,
        ..enabled()
    });
    let actor = game.roster.get_mut(game.player_id);
    actor.health = 10;
    actor.bleeding.as_mut().unwrap().hit();
    for _ in 0..30 {
        actor.bleeding_tick(false);
    }
    assert_eq!(actor.health, 7);
    actor.bleeding.as_mut().unwrap().hit();
    for _ in 0..60 {
        actor.bleeding_tick(true);
    }
    assert_eq!(actor.bleeding.as_ref().unwrap().remaining, 60);
    for _ in 0..60 {
        actor.bleeding_tick(false);
    }
    assert_eq!(actor.health, 1);
    assert!(!actor.bleeding.as_ref().unwrap().active);
    assert_eq!(actor.bleeding.as_ref().unwrap().bandages, 3);

    actor.bleeding.as_mut().unwrap().hit();
    actor.motion.action = MOT_ATTACK;
    actor.bleeding.as_mut().unwrap().input(true);
    actor.bleeding_tick(false);
    assert!(actor.bleeding.as_ref().unwrap().active);
    actor.motion.action = MOT_NORMAL;
    actor.bleeding_tick(false);
    assert!(
        actor.bleeding.as_ref().unwrap().active,
        "failed use must not queue a later treatment"
    );
    actor.bleeding.as_mut().unwrap().input(false);
    actor.bleeding.as_mut().unwrap().input(true);
    actor.bleeding_tick(false);
    assert!(!actor.bleeding.as_ref().unwrap().active);
    assert_eq!(actor.bleeding.as_ref().unwrap().bandages, 2);
    assert_eq!(actor.health, 1, "bandages must not heal");
    actor.bleeding.as_mut().unwrap().hit();
    actor.bleeding.as_mut().unwrap().input(true);
    actor.bleeding_tick(false);
    assert!(
        actor.bleeding.as_ref().unwrap().active,
        "holding a button must not use multiple bandages"
    );
    game.set_bleeding(Settings {
        seconds: 0,
        bandages: 99,
        ..enabled()
    });
    let actor = game.roster.get_mut(game.player_id);
    assert_eq!(
        actor.bleeding.as_ref().unwrap().bandages,
        2,
        "changing loadout refilled current stock"
    );
    for _ in 0..1000 {
        actor.bleeding_tick(false);
    }
    assert!(actor.bleeding.as_ref().unwrap().active);
    actor.health = 0;
    actor.bleeding_tick(false);
    assert!(!actor.bleeding.as_ref().unwrap().active);
    game.set_bleeding(Settings::default());
    assert!(game.player().bleeding.is_none());
}

#[test]
#[ignore = "requires TENCHU_DISC; potion treatment occurs on recovery, never on consumption or interruption"]
fn bleeding_medicine_cures_only_on_successful_recovery() {
    for (cure, interrupted) in [(true, false), (false, false), (true, true)] {
        let mut game = oracle::game(0, 0);
        game.set_bleeding(Settings {
            medicine: cure,
            seconds: 0,
            damage: 0,
            ..enabled()
        });
        let player = game.roster.get_mut(game.player_id);
        player.health = 25;
        player.inventory[3] = 2;
        player.bleeding.as_mut().unwrap().hit();
        game.hud.cursor = 3;
        game.step(0).unwrap();
        game.step(crate::input::TRIANGLE).unwrap();
        assert_eq!(game.player().motion.action, MOT_ITEM_DRINK);
        for _ in 0..15 {
            game.step(0).unwrap();
        }
        assert!(game.player().bleeding.as_ref().unwrap().active);
        if interrupted {
            let id = game.player_id;
            with_actor(&mut game, id, |a, ctx, _| {
                a.enter(ctx, MOT_DAMAGE, true);
            });
        }
        for _ in 0..100 {
            game.step(0).unwrap();
        }
        assert_eq!(
            game.player().bleeding.as_ref().unwrap().active,
            !cure || interrupted
        );
        assert_eq!(game.player().bleeding.as_ref().unwrap().bandages, 3);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; a real archer arrow starts bleeding while god mode prevents it"]
fn bleeding_from_owned_projectiles() {
    for invulnerable in [false, true] {
        let (mut game, kind) = (0..11)
            .find_map(|stage| {
                let game = oracle::game(stage, 0);
                let kind = game
                    .mission
                    .characters
                    .iter()
                    .find(|c| {
                        c.kind >= 0x10
                            && game.mission.models.contains_key(&c.kind)
                            && c.motions
                                .iter()
                                .any(|m| m.action == MOT_ATTACK && m.clip == 0xaa)
                    })?
                    .kind;
                Some((game, kind))
            })
            .unwrap();
        let shooter = arena(&mut game, kind);
        game.set_bleeding(enabled());
        game.roster.get_mut(game.player_id).invulnerable = invulnerable;
        with_actor(&mut game, shooter, |a, ctx, _| {
            a.attribute = ATTR_WEAPON_DRAWN;
            a.think = Default::default();
            assert!(a.enter(ctx, MOT_ATTACK, true));
        });
        let mut seen = false;
        for _ in 0..220 {
            game.step(0).unwrap();
            seen |= game.player().bleeding.as_ref().unwrap().active;
        }
        assert_eq!(seen, !invulnerable);
    }
}
