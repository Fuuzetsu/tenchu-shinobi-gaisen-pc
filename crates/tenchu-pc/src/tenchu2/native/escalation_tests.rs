use super::{Session, bleeding_tests};
use tenchu_core::{
    ai::escalation::Settings,
    math::Position,
    tenchu2::{input::GUARD, world::Placement},
};

const ESCALATION: Settings = Settings {
    increment: 50,
    maximum: 100,
};

fn decide(s: &mut Session, guard: usize, ticks: u32) {
    for _ in 0..ticks {
        s.world.tick += 1;
        for id in [s.world.player, guard] {
            s.world
                .decide_actor(
                    id,
                    [0; 2],
                    &s.assets,
                    &mut bleeding_tests::Recorded::default(),
                )
                .unwrap();
        }
    }
}

fn sight_fixture() -> (Session, usize) {
    let (mut s, guard) = bleeding_tests::fixture();
    s.world.set_damage_escalation(ESCALATION);
    let agent = &mut s.world.actors[guard];
    agent.actor.flags = 0x10;
    agent.actor.position = Position::new(0, 0, 6000);
    agent.actor.target = Some(s.world.player);
    agent.actor.rotation.y = 0;
    agent
        .behavior
        .assign(0x1015, s.world.map, s.world.difficulty, &mut s.world.random)
        .unwrap();
    (s, guard)
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual native sight, alert expiry and calm transitions delimit encounters"]
fn native_escalation_counts_direct_sight_after_calm_and_restores_encounters() {
    let (mut s, guard) = sight_fixture();
    decide(&mut s, guard, 128);
    assert_eq!(s.world.actors[guard].actor.encounters.detections, 1);
    for _ in 0..3 {
        s.world.actors[s.world.player].actor.position = Position::new(-25000, 0, -25000);
        decide(&mut s, guard, 35);
        assert_ne!(s.world.actors[guard].actor.flags & 3, 0);
        s.world.actors[s.world.player].actor.position = Position::ZERO;
        decide(&mut s, guard, 64);
        assert_eq!(s.world.actors[guard].actor.encounters.detections, 1);
    }
    for encounter in 2..=4 {
        s.world.actors[s.world.player].actor.position = Position::new(-25000, 0, -25000);
        let mut calm = false;
        for _ in 0..2400 {
            decide(&mut s, guard, 1);
            if s.world.ai.global.perception.alert.remaining == 0
                && s.world.actors[guard].actor.flags & 3 == 0
            {
                calm = true;
                break;
            }
        }
        assert!(calm, "guard did not return to calm");
        decide(&mut s, guard, 1);
        s.world.actors[s.world.player].actor.position = Position::ZERO;
        let saved = postcard::to_allocvec(&s.world).unwrap();
        decide(&mut s, guard, 64);
        assert_eq!(s.world.actors[guard].actor.encounters.detections, encounter);
        assert_eq!(
            s.world.actors[guard].actor.encounters.bonus(ESCALATION),
            ((encounter - 1) * 50).min(100)
        );
        let expected = postcard::to_allocvec(&s.world).unwrap();
        s.world = postcard::from_bytes(&saved).unwrap();
        decide(&mut s, guard, 64);
        assert!(postcard::to_allocvec(&s.world).unwrap() == expected);
    }
    s.world.set_damage_escalation(Settings {
        maximum: 200,
        ..ESCALATION
    });
    assert_eq!(s.world.actors[guard].actor.encounters.detections, 4);
    s.world.set_damage_escalation(Settings {
        maximum: 0,
        ..ESCALATION
    });
    assert_eq!(s.world.actors[guard].actor.encounters.detections, 0);
    s.world.set_damage_escalation(ESCALATION);
    decide(&mut s, guard, 64);
    assert_eq!(s.world.actors[guard].actor.encounters.detections, 1);
    s.world.set_damage_escalation(Default::default());
    assert_eq!(s.world.actors[guard].actor.encounters.detections, 0);
}

#[test]
#[ignore = "requires TENCHU2_DISC; shared alerts, hidden player, scripts and other targets do not count as direct encounters"]
fn native_escalation_ignores_alarms_and_ineligible_sight() {
    let (mut s, guard) = sight_fixture();
    let initial = s.world.clone();
    for mode in ["alarm", "hidden", "script", "other", "training"] {
        s.world = initial.clone();
        match mode {
            "alarm" => {
                s.world.ai.global.perception.alert.remaining = 500;
                s.world.actors[guard].actor.rotation.y = 2048;
            }
            "hidden" => s.world.ai.ignore_player = true,
            "script" => s.world.script = 1,
            "other" => s.world.actors[guard].actor.target = Some(guard),
            "training" => {
                s.world.map = 0;
                s.world.actors[guard].actor.kind = 0x60;
            }
            _ => unreachable!(),
        }
        let mut original = s.world.clone();
        original.set_damage_escalation(Default::default());
        decide(&mut s, guard, 128);
        assert_eq!(
            s.world.actors[guard].actor.encounters.detections, 0,
            "{mode}"
        );
        std::mem::swap(&mut s.world, &mut original);
        decide(&mut s, guard, 128);
        let expected = postcard::to_allocvec(&s.world).unwrap();
        std::mem::swap(&mut s.world, &mut original);
        let mut normalized = s.world.clone();
        normalized.set_damage_escalation(Default::default());
        assert!(
            postcard::to_allocvec(&normalized).unwrap() == expected,
            "{mode}: native state changed"
        );
        if mode == "alarm" {
            assert_ne!(s.world.actors[guard].actor.flags & 3, 0);
            s.world.actors[guard].actor.rotation.y = 0;
            decide(&mut s, guard, 64);
            assert_eq!(s.world.actors[guard].actor.encounters.detections, 1);
        }
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; individual and newly placed guards retain independent encounter histories"]
fn native_escalation_tracks_each_guard_separately() {
    let (mut s, first) = sight_fixture();
    decide(&mut s, first, 64);
    s.world.actors[first].actor.encounters.detections = 3;
    let actor = &s.world.actors[first].actor;
    let second = s
        .world
        .place_actor(
            Placement {
                kind: actor.kind,
                definition: actor.definition_kind(),
                behavior: 0x1015,
                position: Position::new(0, 0, -6000),
                yaw: 0,
                snap: true,
            },
            &s.assets,
        )
        .unwrap();
    for yaw in [0, 2048] {
        s.world.actors[second].actor.rotation.y = yaw;
        for _ in 0..128 {
            s.world.tick += 1;
            for id in [s.world.player, first, second] {
                s.world
                    .decide_actor(
                        id,
                        [0; 2],
                        &s.assets,
                        &mut bleeding_tests::Recorded::default(),
                    )
                    .unwrap();
            }
        }
        assert_eq!(s.world.actors[first].actor.encounters.detections, 3);
        assert_eq!(
            s.world.actors[second].actor.encounters.detections,
            u16::from(yaw != 0)
        );
    }
    s.world.set_damage_escalation(Default::default());
    assert!(
        s.world
            .actors
            .iter()
            .all(|a| a.actor.encounters.detections == 0)
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; optional damage changes only player health, with source block, armour, ownership and injury timing"]
fn native_escalation_scales_owned_damage_without_changing_reactions() {
    let (mut s, guard) = bleeding_tests::fixture();
    s.world.actors[s.world.player].actor.health = 10000;
    s.world.actors[s.world.player].actor.max_health = 10000;
    let initial = s.world.clone();
    for (held, invulnerable, armor, object) in [
        (0, false, false, None),
        (GUARD, false, false, None),
        (0, true, false, None),
        (0, false, true, None),
        (0, false, false, Some((4, Some(guard)))),
        (0, false, false, Some((22, Some(guard)))),
        (0, false, false, Some((22, None))),
        (0, false, false, Some((22, Some(s.world.player)))),
        (0, false, false, Some((5, None))),
    ] {
        let mut original = None;
        for encounters in [0, 1, 2, 3, u16::MAX] {
            s.world = initial.clone();
            if encounters > 0 {
                s.world.set_damage_escalation(ESCALATION);
            }
            s.world.actors[guard].actor.encounters.detections = encounters;
            s.world.actors[s.world.player].actor.invulnerable = invulnerable;
            s.world.controls.armor = armor;
            bleeding_tests::strike_with_weapon(&mut s, guard, held, object, true);
            let loss = 10000 - s.world.actors[s.world.player].actor.health;
            let expected_bonus =
                if encounters < 2 || object.is_some_and(|(_, owner)| owner != Some(guard)) {
                    0
                } else {
                    (u32::from(encounters - 1) * 50).min(100)
                };
            s.world.actors[s.world.player].actor.health = 10000;
            s.world.actors[guard].actor.encounters = Default::default();
            s.world.ai.damage_escalation = Default::default();
            let normalized = postcard::to_allocvec(&s.world).unwrap();
            if let Some((base_loss, base_state)) = &original {
                assert_eq!(
                    i32::from(loss),
                    base_loss * (100 + expected_bonus as i32) / 100
                );
                assert!(
                    &normalized == base_state,
                    "injury state changed for {encounters} sightings"
                );
            } else {
                if held == 0 && !invulnerable {
                    assert!(loss > 0);
                }
                original = Some((i32::from(loss), normalized));
            }
        }
    }
    s.world = initial;
    s.world.set_damage_escalation(ESCALATION);
    s.world.actors[guard].actor.encounters.detections = 3;
    let player = &mut s.world.actors[s.world.player].actor;
    player.health = 1;
    player.inventory.resurrections = 1;
    bleeding_tests::strike_with_weapon(&mut s, guard, 0, None, true);
    assert_eq!(
        s.world.actors[s.world.player].actor.inventory.resurrections,
        0
    );
    assert_eq!(s.world.actors[s.world.player].actor.health, 10000);
}
