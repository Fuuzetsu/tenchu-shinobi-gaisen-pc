use super::*;

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; actual NPC melee contacts retain source skill/difficulty, ignore player-only modifiers and preserve blocks"]
fn native_full_enemy_melee_damage_preserves_source_modifiers() {
    let (mut s, [attacker, victim]) = fixture(0);
    s.world.actors[attacker].actor.position = Position::ZERO;
    s.world.actors[attacker].actor.previous = Position::ZERO;
    let weapon = swing(&mut s, attacker, 0);
    place(&mut s, victim, weapon, false);
    s.world
        .set_damage_escalation(tenchu_core::ai::escalation::Settings {
            increment: 100,
            maximum: 400,
        });
    s.world.actors[attacker].actor.encounters.detections = 5;
    s.world
        .set_player_sheathing(
            tenchu_core::game::sheathing::Settings {
                enabled: true,
                damage: 400,
                one_hit: true,
                ..Default::default()
            },
            &s.assets,
        )
        .unwrap();
    let attack = s.world.actors[attacker].actor.attack.index.unwrap();
    let original = s.world.clone();
    let mut hits = 0;
    for power in [1, 2, 3, 7, 23, 24, 31, 100] {
        for skill in [0, 4] {
            for difficulty in [1, 3] {
                for armor in [false, true] {
                    let mut recoil = None;
                    for full in [false, true] {
                        s.world = original.clone();
                        s.world.difficulty = difficulty;
                        s.world.controls.armor = armor;
                        s.world.controls.full_enemy_melee_damage = full;
                        s.world.actors[victim].actor.skill = skill;
                        s.assets.resources.weapons.attacks[attack].power = power;
                        s.world.random = Random::new(1);
                        assert!(contact(&mut s, victim));
                        let damage = if difficulty == 3 {
                            i16::from(power)
                        } else {
                            i16::from(power) - i16::from(power) * skill / 10
                        };
                        let expected = if full { damage } else { damage / 3 };
                        assert_eq!(
                            30000 - s.world.actors[victim].actor.health,
                            expected,
                            "power {power}, skill {skill}, difficulty {difficulty}, armor {armor}, full {full}"
                        );
                        let actual = s.world.actors[attacker].actor.motion.loops;
                        if let Some(before) = recoil {
                            assert_eq!(actual, before);
                        }
                        recoil = Some(actual);
                        hits += 1;
                    }
                }
            }
        }
    }
    assert_eq!(hits, 128);
    s.world = original;
    place(&mut s, victim, weapon, true);
    let blocked = s.world.clone();
    let mut reference = None;
    for full in [false, true] {
        s.world = blocked.clone();
        s.world.random = Random::new(1);
        s.world.controls.full_enemy_melee_damage = full;
        assert!(contact(&mut s, victim));
        assert_eq!(s.world.actors[victim].actor.health, 30000);
        s.world.controls.full_enemy_melee_damage = false;
        let bytes = postcard::to_allocvec(&s.world).unwrap();
        if let Some(before) = &reference {
            assert!(before == &bytes);
        }
        reference = Some(bytes);
    }
}
