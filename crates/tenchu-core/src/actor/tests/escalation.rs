use super::*;
use crate::ai::escalation::Settings;

const ESCALATION: Settings = Settings {
    increment: 50,
    maximum: 100,
};

#[test]
#[ignore = "requires TENCHU_DISC; real sight loss and calm transitions delimit repeated detections"]
fn escalation_counts_new_encounters_not_brief_sight_loss() {
    let mut game = oracle::game(0, 0);
    let guard = arena(&mut game, 0x10);
    game.roster.keep_all_active = true;
    game.roster.get_mut(game.player_id).invulnerable = true;
    game.roster.get_mut(guard).setup_think(0x3444).unwrap();
    game.set_damage_escalation(ESCALATION);
    for _ in 0..150 {
        game.step(0).unwrap();
    }
    assert_eq!(game.roster.get(guard).encounters.detections, 1);
    assert_eq!(game.roster.get(guard).encounters.bonus(ESCALATION), 0);
    let away = Position::new(-25000, 0, -25000);
    let return_to_guard = |game: &mut Game| {
        let a = game.roster.get(guard);
        let forward = game.math.velocity(a.rotation[1], 4000, 0);
        let position = Position::new(
            a.position.x + i32::from(forward[0]),
            0,
            a.position.z + i32::from(forward[2]),
        );
        game.teleport_player(position, true).unwrap();
    };
    for _ in 0..3 {
        game.teleport_player(away, true).unwrap();
        for _ in 0..5 {
            game.step(0).unwrap();
        }
        return_to_guard(&mut game);
        for _ in 0..60 {
            game.step(0).unwrap();
        }
        assert_eq!(game.roster.get(guard).encounters.detections, 1);
    }
    for encounter in 2..=4 {
        game.teleport_player(away, true).unwrap();
        let mut calm = false;
        for _ in 0..1800 {
            game.step(0).unwrap();
            if game.intelligence.alert == 0 && game.roster.get(guard).attribute & ATTR_PHASE == 0 {
                calm = true;
                break;
            }
        }
        assert!(
            calm,
            "Guard never calmed down: {:?}",
            game.roster.get(guard).brain
        );
        game.step(0).unwrap();
        return_to_guard(&mut game);
        for _ in 0..120 {
            game.step(0).unwrap();
        }
        assert_eq!(game.roster.get(guard).encounters.detections, encounter);
        assert_eq!(
            game.roster.get(guard).encounters.bonus(ESCALATION),
            ((encounter - 1) * 50).min(100)
        );
    }
    game.set_damage_escalation(Settings::default());
    assert_eq!(game.roster.get(guard).encounters.detections, 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; escalation changes melee health loss without changing combat motion or RNG"]
fn escalation_scales_damage_without_changing_duel_timing() {
    let run = |character, enabled, detections| {
        let mut game = oracle::game(0, character);
        let guard = arena(&mut game, 0x10);
        let player = game.roster.get_mut(game.player_id);
        player.health = 9999;
        player.max_health = 9999;
        game.roster.get_mut(guard).setup_think(0x4444).unwrap();
        if enabled {
            game.set_damage_escalation(ESCALATION);
        }
        let mut frames = Vec::new();
        for _ in 0..600 {
            game.step(0).unwrap();
            if game.roster.get(guard).encounters.detections > 0 {
                game.roster.get_mut(guard).encounters.detections = detections;
            }
            frames.push((
                game.random.seed,
                game.roster
                    .order
                    .iter()
                    .map(|&id| {
                        let a = game.roster.get(id);
                        (
                            a.position,
                            a.status,
                            a.motion.action,
                            a.motion.count,
                            a.brain.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        (frames, 9999 - game.player().health)
    };
    for character in 0..2 {
        let original = run(character, false, 0);
        assert!(original.1 > 0, "Guard never hit player");
        assert_eq!(run(character, true, 1), original);
        let doubled = run(character, true, 3);
        assert_eq!(doubled.0, original.0);
        assert_eq!(doubled.1, original.1 * 2);
        assert_eq!(run(character, true, u16::MAX), doubled);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; an archer's real arrow retains attacker attribution through impact"]
fn escalation_applies_to_owned_projectiles() {
    let run = |enabled, invulnerable| {
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
        game.roster.get_mut(game.player_id).invulnerable = invulnerable;
        if enabled {
            game.set_damage_escalation(ESCALATION);
        }
        with_actor(&mut game, shooter, |actor, ctx, _| {
            actor.attribute = ATTR_WEAPON_DRAWN;
            actor.think = crate::ai::programs::Programs::default();
            actor.encounters.detections = 3;
            assert!(actor.enter(ctx, MOT_ATTACK, true));
        });
        let health = game.player().health;
        let mut launched = false;
        for _ in 0..220 {
            game.step(0).unwrap();
            launched |= game
                .items
                .slots
                .iter()
                .any(|i| i.processor() == Some(crate::items::Processor::Arrow));
        }
        assert!(launched);
        health - game.player().health
    };
    assert_eq!(run(false, false), 10);
    assert_eq!(run(true, false), 20);
    assert_eq!(run(true, true), 0);
}
