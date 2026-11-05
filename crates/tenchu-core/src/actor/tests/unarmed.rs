use super::*;
use crate::game::sheathing::Settings;
use crate::input::*;

fn ready(character: i16) -> (Game, usize) {
    let (mut game, enemy) = super::sheathing::ready(
        character,
        Settings {
            unarmed: true,
            ..Default::default()
        },
    );
    let source = (0..11)
        .map(|stage| oracle::game(stage, character))
        .find(|g| {
            [454, 455, 456, 458, 459, 460, 461]
                .iter()
                .all(|id| g.mission.clips.iter().any(|c| c.id == *id))
        })
        .unwrap();
    let ninja = source
        .mission
        .characters
        .iter()
        .find(|c| c.kind == 0x87)
        .unwrap();
    let attacks: Vec<_> = ninja
        .motions
        .iter()
        .filter(|m| matches!(m.action, MOT_ATTACK..=MOT_ATTACK_LUNGE))
        .map(|m| {
            (
                m.action,
                source
                    .mission
                    .clips
                    .iter()
                    .find(|c| c.id == m.clip)
                    .unwrap()
                    .clone(),
            )
        })
        .collect();
    game.set_player_unarmed(&attacks).unwrap();
    with_actor(&mut game, enemy, |a, ctx, _| {
        a.position.z = 900;
        a.rotation[1] = 2048;
        a.health = 9999;
        a.max_health = 9999;
        a.equip(ctx, true);
        a.enter(ctx, MOT_ENGAGE_STANCE, true);
        ctx.scene.update(a.root_model, a.root(ctx.math));
    });
    (game, enemy)
}

#[test]
#[ignore = "requires TENCHU_DISC; both heroes land Onikage's three-hit combo using limb hitboxes while their blades stay stowed"]
fn unarmed_combo_hits_with_original_timings_and_retains_weapons() {
    for character in 0..2 {
        let (mut game, enemy) = ready(character);
        let weapons = game
            .player()
            .weapons
            .each_ref()
            .map(|w| w.as_ref().map(|w| (w.model, w.kind)));
        let hitboxes = game.player().weapon_hitbox;
        game.step(SQUARE).unwrap();
        assert!(game.player().unarmed_attack());
        assert_eq!(
            game.mission.clips[game.player().motion.clip.unwrap()].id,
            454
        );
        let mut actions = std::collections::BTreeSet::new();
        let mut hit = false;
        let mut limbs = false;
        for _ in 0..160 {
            let a = game.player();
            let button = if matches!(a.motion.action, MOT_ATTACK | MOT_ATTACK_SLASH2)
                && a.motion.count == game.mission.weapons.attacks[a.attack].combo
            {
                SQUARE
            } else {
                0
            };
            game.step(button).unwrap();
            let a = game.player();
            if a.unarmed_attack() {
                actions.insert(a.motion.action);
                for part in [8, 11] {
                    if let Some(slot) = game.scene.models[a.parts[part]].conflict {
                        assert_eq!(game.collision.slots[slot].class, crate::collision::HIT);
                        limbs = true;
                    }
                }
            }
            assert_eq!(a.attribute & ATTR_WEAPON_DRAWN, 0);
            assert_eq!(a.weapon_hitbox, hitboxes);
            assert_eq!(
                a.weapons
                    .each_ref()
                    .map(|w| w.as_ref().map(|w| (w.model, w.kind))),
                weapons
            );
            hit |= game.roster.get(enemy).health < 9999;
        }
        assert_eq!(
            actions,
            [MOT_ATTACK, MOT_ATTACK_SLASH2, MOT_ATTACK_SLASH3].into()
        );
        assert!(
            hit && limbs,
            "hero {character}: hit={hit}, limb collision={limbs}"
        );
        assert_eq!(game.player().motion.action, MOT_NORMAL);
        for part in [8, 11, 13, 14] {
            assert!(
                game.scene.models[game.player().parts[part]]
                    .conflict
                    .is_none()
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; disabling unarmed combat during a kick finishes it, disabling all sheathing clears hits and draws blades"]
fn unarmed_settings_changes_do_not_leave_attack_collisions() {
    for character in 0..2 {
        for disable_sheathing in [false, true] {
            let (mut game, _) = ready(character);
            game.step(SQUARE).unwrap();
            for _ in 0..10 {
                game.step(0).unwrap();
            }
            assert!(game.player().unarmed_attack());
            let settings = Settings {
                enabled: !disable_sheathing,
                ..Default::default()
            };
            game.set_player_sheathing(settings, None).unwrap();
            for _ in 0..80 {
                game.step(0).unwrap();
            }
            for part in [8, 11, 13, 14] {
                assert!(
                    game.scene.models[game.player().parts[part]]
                        .conflict
                        .is_none()
                );
            }
            assert_eq!(
                game.player().attribute & ATTR_WEAPON_DRAWN != 0,
                disable_sheathing
            );
            game.step(SQUARE).unwrap();
            assert!(!game.player().unarmed_attack());
            assert_eq!(
                game.player().motion.action,
                if disable_sheathing {
                    MOT_ATTACK
                } else {
                    MOT_STATE_DRAW
                }
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; unarmed mode retains stealth kills and restores the original sword moves after drawing"]
fn unarmed_mode_draws_for_stealth_and_explicit_weapon_changes() {
    for character in 0..2 {
        let (mut game, enemy) = ready(character);
        let player = game.player_id;
        with_actor(&mut game, enemy, |a, ctx, _| {
            a.equip(ctx, false);
            a.attribute &= !ATTR_PHASE;
            a.enter(ctx, MOT_NORMAL, true);
        });
        game.step(SQUARE).unwrap();
        assert_eq!(
            game.player().motion.action,
            MOT_ATTACK_STEALTH_BACK + character * 3
        );
        assert_ne!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        for _ in 0..300 {
            game.step(0).unwrap();
        }
        assert_eq!(game.statistics.criticals, 1);
        let (mut game, _) = ready(character);
        game.roster
            .get_mut(player)
            .sheathing
            .as_mut()
            .unwrap()
            .input(true);
        game.step(0).unwrap();
        game.roster
            .get_mut(player)
            .sheathing
            .as_mut()
            .unwrap()
            .input(false);
        for _ in 0..80 {
            game.step(0).unwrap();
        }
        assert_ne!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        game.step(SQUARE).unwrap();
        assert!(!game.player().unarmed_attack());
        let original = game.mission.characters[game.player().definition]
            .motions
            .iter()
            .find(|m| m.action == MOT_ATTACK)
            .unwrap()
            .clip;
        assert_eq!(
            game.mission.clips[game.player().motion.clip.unwrap()].id,
            original
        );
        let (mut game, _) = ready(character);
        game.step(CROSS).unwrap();
        let mut dive = false;
        for _ in 0..100 {
            game.step(SQUARE).unwrap();
            if game.player().motion.action == MOT_ATTACK_DIVE {
                assert_ne!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
                assert!(!game.player().unarmed_attack());
                dive = true;
            }
        }
        assert!(
            dive,
            "hero {character}: armed dive should work from a sheathed jump"
        );
    }
}
