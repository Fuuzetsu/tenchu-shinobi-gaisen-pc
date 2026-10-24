use super::*;

fn guard(game: &mut Game) -> usize {
    let enemy = arena(game, 0x10);
    with_actor(game, enemy, |actor, ctx, _| {
        actor.position.z = 1800;
        actor.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | 2;
        actor.think = [4; 4];
        actor.enter(ctx, MOT_ENGAGE_STANCE, true);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
    });
    game.camera.snap_pending = true;
    enemy
}

#[test]
#[ignore = "requires TENCHU_DISC; god mode preserves health under real combat without suppressing hit reactions"]
fn god_mode_prevents_lethal_hits_and_can_be_disabled() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        guard(&mut game);
        let player = game.player_id;
        game.roster.get_mut(player).invulnerable = true;
        game.roster.get_mut(player).health = 1;
        let mut hit = false;
        for _ in 0..600 {
            game.step(0).unwrap();
            hit |= (MOT_DAMAGE..=MOT_DAMAGE_GETUP).contains(&game.player().motion.action);
            assert_eq!(game.player().health, 1);
        }
        assert!(hit, "guard never struck character {character}");
        game.roster.get_mut(player).invulnerable = false;
        for _ in 0..600 {
            game.step(0).unwrap();
            if game.player().health == 0 {
                break;
            }
        }
        assert_eq!(game.player().health, 0, "disabling must restore damage");
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; stealth cancels an alert guard, prevents detection and restores sensing when disabled"]
fn stealth_cancels_alert_and_restores_detection() {
    let mut game = oracle::game(0, 0);
    let enemy = guard(&mut game);
    game.intelligence.ignore_player = true;
    let found = game.intelligence.found;
    for _ in 0..180 {
        game.step(0).unwrap();
        assert_eq!(game.roster.get(enemy).attribute & ATTR_PHASE, 0);
        assert_eq!(game.intelligence.found, found);
        assert_eq!(game.intelligence.alert, 0);
    }
    assert_eq!(game.player().health, game.player().max_health);
    game.intelligence.ignore_player = false;
    for _ in 0..180 {
        game.step(0).unwrap();
        if game.intelligence.found != found {
            break;
        }
    }
    assert!(
        game.intelligence.found > found,
        "disabling stealth must restore detection"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; inventory edits preserve active-item state and edited equipment is usable"]
fn inventory_edits_preserve_timers_and_allow_item_use() {
    let mut game = oracle::game(0, 0);
    arena(&mut game, 0x10);
    let player = game.player_id;
    for slot in 0..19 {
        game.set_inventory(slot, 0).unwrap();
    }
    game.roster.get_mut(player).inventory[21] = 71;
    let internal = game.player().inventory[19..].to_vec();
    let random = game.random;
    assert!(game.set_inventory(21, 0).is_err());
    game.set_inventory(4, 255).unwrap();
    assert_eq!(&game.player().inventory[19..], internal);
    assert_eq!(game.random, random);
    assert_eq!(game.hud.selected, Some(4));
    game.step(0).unwrap();
    game.step(crate::input::TRIANGLE).unwrap();
    assert_eq!(game.player().motion.action, MOT_ITEM_THROW);
    for _ in 0..90 {
        game.step(0).unwrap();
    }
    assert_eq!(game.player().inventory[4], 255);
    assert!(
        game.items
            .slots
            .iter()
            .any(|i| i.processor == Some(crate::items::Processor::Fire))
    );
    game.set_inventory(4, 0).unwrap();
    assert_eq!(game.hud.selected, None);
}
