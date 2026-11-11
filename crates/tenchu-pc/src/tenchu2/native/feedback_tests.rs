use super::*;
use tenchu_core::{effects::RumbleAction, tenchu2::feedback::Feedback};

#[test]
#[ignore = "requires TENCHU2_FEEDBACK_ORACLE; original JP motor conversion and input-poll countdown"]
fn native_feedback_matches_original_motor_instructions() {
    #[derive(serde::Deserialize)]
    struct Case {
        level: i16,
        duration: i16,
        player: bool,
        before: [i16; 3],
        after: Vec<[i16; 3]>,
        #[serde(default)]
        direct: bool,
    }
    let text =
        std::fs::read_to_string(std::env::var_os("TENCHU2_FEEDBACK_ORACLE").unwrap()).unwrap();
    let mut count = 0;
    for line in text.lines() {
        let case: Case = serde_json::from_str(line).unwrap();
        let mut feedback = Feedback {
            motors: [case.before[0] as u8, case.before[1] as u8],
            remaining: case.before[2],
            action: Some(RumbleAction::Attack),
        };
        let mut pending = BTreeSet::new();
        let voices = BTreeMap::new();
        let mut output = Output {
            player: 0,
            feedback: &mut feedback,
            voice: None,
            weapon_voice: None,
            actor_voices: &voices,
            listener: Listener {
                actor: 0,
                position: Position::default(),
                yaw: 0,
                distance: 0,
            },
            requests: Vec::new(),
            wounds: Vec::new(),
            melee_hits: BTreeSet::new(),
            pending: &mut pending,
        };
        if case.direct {
            output.feedback.drowning(case.level);
        } else {
            environment::Events::emit(
                &mut output,
                environment::Event::Rumble {
                    actor: usize::from(!case.player),
                    action: RumbleAction::Attack,
                    kind: case.level,
                    frames: case.duration,
                },
                &mut Random::new(1),
            )
            .unwrap();
        }
        for (tick, expected) in case.after.iter().enumerate() {
            if tick != 0 {
                feedback.poll();
            }
            assert_eq!(
                [
                    i16::from(feedback.motors[0]),
                    i16::from(feedback.motors[1]),
                    feedback.remaining
                ],
                *expected,
                "case {count}, poll {tick}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 1713);
}

#[test]
#[ignore = "requires TENCHU2_DISC; native attacks, paired kills, drowning and scenery reach the player motors"]
fn native_feedback_routes_gameplay_events_and_preserves_rng() {
    use tenchu_core::tenchu2::{
        actor::Velocity,
        input::ATTACK,
        physics::{Depth, Water},
    };
    let (mut world, mut assets, guard) = crate::tenchu2::tests::native_world::fixture();
    let original = world.clone();
    let mut feedback = Feedback::default();
    let mut pending = BTreeSet::new();
    let voices = BTreeMap::new();
    let mut output = Output {
        player: world.player,
        feedback: &mut feedback,
        voice: None,
        weapon_voice: None,
        actor_voices: &voices,
        listener: world.listener(),
        requests: Vec::new(),
        wounds: Vec::new(),
        melee_hits: BTreeSet::new(),
        pending: &mut pending,
    };

    world.actors[0].actor.pad.update(ATTACK);
    world.control_actor(0, &mut assets, &mut output).unwrap();
    assert_eq!(world.actors[guard].actor.health, 0);
    let paired = world.clone();
    let mut pulses = Vec::new();
    for _ in 0..180 {
        output.feedback.clear();
        for id in [0, guard] {
            world.move_actor(id, &assets, &mut output).unwrap();
            world.actors[id].actor.pad.update(0);
            world.control_actor(id, &mut assets, &mut output).unwrap();
            world.animate_actor(id, &assets).unwrap();
        }
        if output.feedback.action == Some(RumbleAction::StealthKill) {
            pulses.push((output.feedback.motors, output.feedback.remaining));
        }
        world.finish_tick(&assets, 6).unwrap();
    }
    assert!(
        !pulses.is_empty(),
        "victim's paired animation never reached the player's controller"
    );
    assert!(
        pulses
            .iter()
            .all(|pulse| matches!(pulse, ([1, 252], 5) | ([1, 189], 10) | ([0, 126], 15)))
    );

    world = paired;
    world.actors[0].actor.target = None;
    world
        .control_actor(guard, &mut assets, &mut output)
        .unwrap();
    assert_eq!(
        output.feedback.action,
        Some(RumbleAction::StealthInterrupted)
    );
    assert_eq!(
        (output.feedback.motors, output.feedback.remaining),
        ([1, 252], 10)
    );

    world = original.clone();
    world.tick = 303;
    let player = &mut world.actors[0].actor;
    player.health = 43;
    player.breath = 0;
    let power = player
        .max_health
        .wrapping_sub(player.health)
        .wrapping_mul(2) as u8;
    player
        .change_motion(0x530, Velocity::Stop, &assets.resources)
        .unwrap();
    world.water = Water {
        depth: Some(Depth::Submerged),
        surface: -4000,
        bottom: 0,
    };
    world.control_actor(0, &mut assets, &mut output).unwrap();
    assert_eq!(world.actors[0].actor.health, 42);
    assert_eq!(output.feedback.action, Some(RumbleAction::Drowning));
    assert_eq!(
        (output.feedback.motors, output.feedback.remaining),
        ([1, power], 5)
    );

    world = original;
    for id in [0, guard] {
        let actor = &mut world.actors[id].actor;
        actor
            .change_motion(0x900, Velocity::Motion, &assets.resources)
            .unwrap();
        actor.attack.index = assets
            .resources
            .weapons
            .attacks
            .iter()
            .position(|a| Some(a.clip) == actor.motion.clip);
        actor.motion.frame =
            i16::from(assets.resources.weapons.attacks[actor.attack.index.unwrap()].start);
        output.feedback.clear();
        world.control_actor(id, &mut assets, &mut output).unwrap();
        if id == 0 {
            assert_eq!(output.feedback.action, Some(RumbleAction::Attack));
            assert_eq!(
                (output.feedback.motors, output.feedback.remaining),
                ([0, 63], 10)
            );
        } else {
            assert_eq!(
                *output.feedback,
                Feedback::default(),
                "enemy swing vibrated the player's controller"
            );
        }
    }

    let random = world.random;
    for actor in [world.player, guard] {
        environment::Events::emit(
            &mut output,
            environment::Event::Rumble {
                actor,
                action: RumbleAction::Scenery,
                kind: if actor == world.player { 1 } else { 4 },
                frames: 3,
            },
            &mut world.random,
        )
        .unwrap();
    }
    assert_eq!(output.feedback.action, Some(RumbleAction::Scenery));
    assert_eq!(
        (output.feedback.motors, output.feedback.remaining),
        ([0, 63], 15)
    );
    assert_eq!(world.random, random);
    assert!(!pending.iter().any(|event| event.contains("Rumble")));
}
