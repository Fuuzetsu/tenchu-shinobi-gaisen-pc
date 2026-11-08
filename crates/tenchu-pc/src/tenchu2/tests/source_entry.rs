use super::source_control::{cases, template, velocity};
use serde::Deserialize;
use std::collections::BTreeSet;
use tenchu_assets::tenchu2::{
    commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    tenchu2::{
        actor::{Resources, State},
        control::{AttackStart, Request, StealthAttack},
    },
};

#[derive(Deserialize)]
struct Target {
    kind: i16,
    health: i16,
    flags: u32,
    action: i16,
    position: [i32; 3],
    angle: i16,
}

#[derive(Deserialize)]
struct Expected {
    action: i16,
    mode: i16,
    motion: i16,
    frame: i16,
    loops: i16,
    state: i16,
    clip: i16,
    velocity: [i16; 3],
    flags: u32,
    history: [u16; 7],
    draw: bool,
    voices: Vec<u8>,
    splash: bool,
    stealth: Option<usize>,
}

#[derive(Deserialize)]
struct Case {
    kind: i16,
    old: i16,
    action: i16,
    angle: i16,
    velocity: [i16; 3],
    flags: u32,
    input: u16,
    frame: i16,
    loops: i16,
    player: bool,
    blocked: u16,
    ground: i32,
    mode: i16,
    map: i16,
    history: [u16; 7],
    clip: i16,
    target: Option<Target>,
    expected: Expected,
}

fn verify(jump: bool) {
    let cases: Vec<Case> = cases(if jump {
        "TENCHU2_JUMP_ENTRY_ORACLE"
    } else {
        "TENCHU2_ATTACK_ENTRY_ORACLE"
    })
    .collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted: BTreeSet<i16> = cases
        .iter()
        .flat_map(|c| [c.clip, c.expected.clip])
        .collect();
    wanted.extend(registry.clip(0, 0));
    for case in &cases {
        for action in [
            0x600, 0x601, 0x602, 0x603, 0x604, 0x606, 0x607, 0x700, 0x900, 0x907, 0x90a, 0x910,
            0x911, 0x913, 0x914, 0x915,
        ] {
            wanted.extend(registry.clip(case.kind, action));
        }
    }
    let resources = Resources {
        clips: motions::collect(&disc, wanted).unwrap(),
        registry,
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let template = template(&resources);
    let styles = [
        StealthAttack::Drawing,
        StealthAttack::Falling,
        StealthAttack::Crouching,
        StealthAttack::Front,
        StealthAttack::Back,
        StealthAttack::Right,
        StealthAttack::Left,
    ];
    let mut seen = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        seen.insert(case.kind);
        let mut actor = template.clone();
        actor.kind = case.kind;
        actor
            .motion
            .select(case.old, case.kind, &resources.registry, &resources.clips)
            .unwrap()
            .unwrap();
        actor.state = State::from_action(case.old).unwrap();
        actor.motion.frame = case.frame;
        actor.motion.loops = case.loops;
        actor.flags = case.flags;
        actor.height = 1800;
        actor.position = [1000, -1000, 2000].into();
        actor.rotation.y = case.angle;
        actor.velocity = case.velocity.into();
        actor.terrain.height = case.ground;
        actor.terrain.blocked = case.blocked;
        actor.pad.held = case.input;
        actor.pad.previous = case.input;
        actor.pad.history = case.history;
        let mut request = Request {
            action: case.action,
            velocity: velocity(case.mode),
        };
        let expected = &case.expected;
        if jump {
            let effects = actor
                .jump_input(&mut request, case.input, &resources, case.player)
                .unwrap();
            assert_eq!(effects.voices, expected.voices, "case {index}");
            assert_eq!(effects.splash, expected.splash, "case {index}");
        } else {
            let target = case.target.as_ref().map(|target| {
                let mut actor = template.clone();
                actor.kind = target.kind;
                actor.health = target.health;
                actor.flags = target.flags;
                actor.motion.action = target.action;
                actor.position = target.position.into();
                actor.rotation.y = target.angle;
                actor
            });
            let input = actor
                .attack_input(
                    request,
                    case.input,
                    if case.player { target.as_ref() } else { None },
                    resources.registry.edition,
                    case.map,
                    &resources.math,
                )
                .unwrap();
            assert_eq!(input.draw_weapon, expected.draw, "case {index}");
            if input.draw_weapon {
                actor.flags |= 0x40;
            }
            if let Some(style) = expected.stealth {
                assert_eq!(
                    input.start,
                    AttackStart::Stealth(styles[style]),
                    "case {index}"
                );
            } else {
                assert!(
                    !matches!(input.start, AttackStart::Stealth(_)),
                    "case {index}"
                );
                actor
                    .start_strike(input.start, &mut request, &resources)
                    .unwrap();
            }
        }
        assert_eq!(
            (request.action, request.velocity),
            (expected.action, velocity(expected.mode)),
            "case {index}, actor {}, action {}",
            case.kind,
            case.old
        );
        assert_eq!(
            (
                actor.motion.action,
                actor.motion.frame,
                actor.motion.loops,
                actor.motion.clip
            ),
            (
                expected.motion,
                expected.frame,
                expected.loops,
                Some(expected.clip)
            ),
            "case {index}"
        );
        assert_eq!(
            actor.state,
            State::from_action(expected.state << 8).unwrap(),
            "case {index}"
        );
        assert_eq!(actor.flags, expected.flags, "case {index}");
        assert_eq!(actor.velocity, expected.velocity, "case {index}");
        assert_eq!(actor.pad.history, expected.history, "case {index}");
    }
    assert!(seen.len() >= 127);
    assert!(cases.len() >= 15_000);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_JUMP_ENTRY_ORACLE from the original jump controller"]
fn native_jump_entry_preserves_regional_speeds_and_motion_fallbacks() {
    verify(true);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ATTACK_ENTRY_ORACLE from the original attack controller"]
fn native_attack_entry_preserves_directional_inputs_and_stealth_selection() {
    verify(false);
}
