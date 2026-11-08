use serde::Deserialize;
use std::{collections::BTreeSet, io::BufRead};
use tenchu_assets::{
    model::Part,
    tenchu2::{commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons},
};
use tenchu_core::{
    math::Math,
    tenchu2::actor::{Actor, Change, Resources, State, Velocity},
};

#[derive(Deserialize)]
struct Expected {
    result: i16,
    action: i16,
    frame: i16,
    loops: i16,
    clip: i16,
    state: i16,
    velocity: [i16; 3],
    changed: bool,
    cleared: bool,
}

#[derive(Deserialize)]
struct Case {
    op: u8,
    kind: i16,
    old: i16,
    clip: i16,
    frame: i16,
    loops: i16,
    angle: i16,
    velocity: [i16; 3],
    drawn: bool,
    target: i16,
    mode: i16,
    expected: Expected,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_DIMENSIONS_ORACLE from the source actor constructor"]
fn native_actor_dimensions_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let data =
        std::fs::read_to_string(std::env::var_os("TENCHU2_DIMENSIONS_ORACLE").unwrap()).unwrap();
    let mut bodies = std::collections::BTreeMap::new();
    let mut checked = 0;
    for line in data.lines() {
        let values: Vec<i16> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let [kind, width, height] = values[..] else {
            panic!("Incomplete dimension trace")
        };
        let definition = registry.actor(kind).unwrap();
        let parts = bodies.entry(&definition.model).or_insert_with(|| {
            tenchu_assets::tenchu2::characters::rig(&disc, &definition.model)
                .unwrap()
                .parts
        });
        assert_eq!(
            tenchu_core::tenchu2::actor::dimensions(kind, parts).unwrap(),
            (width, height),
            "kind {kind}: {}",
            definition.model
        );
        checked += 1;
    }
    assert_eq!(checked, 130);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ACTOR_ORACLE from source motion-transition routines"]
fn native_actor_motion_transitions_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let cases: Vec<Case> = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_ACTOR_ORACLE").unwrap()).unwrap(),
    )
    .lines()
    .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
    .collect();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted: BTreeSet<_> = cases
        .iter()
        .flat_map(|c| [c.clip, c.expected.clip])
        .collect();
    for case in &cases {
        wanted.extend(registry.clip(case.kind, 0));
    }
    let resources = Resources {
        registry,
        clips: motions::collect(&disc, wanted).unwrap(),
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let parts: Vec<_> = (0..20)
        .map(|i| Part {
            parent: if i == 0 { -1 } else { 0 },
            child: i,
            position: [0; 3],
            geometry: vec![],
        })
        .collect();
    assert!(cases.len() > 9000);
    for (i, case) in cases.into_iter().enumerate() {
        let mut actor = Actor::new(case.kind, &parts, &resources).unwrap();
        actor
            .motion
            .select(case.old, case.kind, &resources.registry, &resources.clips)
            .unwrap()
            .unwrap();
        actor.state = State::from_action(case.old).unwrap();
        actor.motion.frame = case.frame;
        actor.motion.loops = case.loops;
        actor.rotation.y = case.angle;
        actor.velocity = case.velocity.into();
        actor.flags = if case.drawn { 0x40 } else { 0 };
        actor.attack.contacts = 0x1234;
        actor.attack.trails = 0x12;
        let mode = match case.mode {
            -1 => Velocity::Stop,
            0 => Velocity::Keep,
            1 => Velocity::Motion,
            _ => panic!("Unknown velocity mode"),
        };
        let result = match case.op {
            0 => actor.change_motion(case.target, mode, &resources).unwrap(),
            1 => actor
                .overlay_motion(case.target, &resources)
                .unwrap()
                .map(|action| Change {
                    action,
                    changed: false,
                }),
            _ => panic!("Unknown motion operation"),
        };
        let expected = case.expected;
        assert_eq!(
            result.map_or(-1, |v| v.action),
            expected.result,
            "case {i}: {} -> {}",
            case.kind,
            case.target
        );
        assert_eq!(
            result.is_some_and(|v| v.changed),
            expected.changed,
            "case {i}"
        );
        assert_eq!(
            (
                actor.motion.action,
                actor.motion.frame,
                actor.motion.loops,
                actor.motion.clip
            ),
            (
                expected.action,
                expected.frame,
                expected.loops,
                Some(expected.clip)
            ),
            "case {i}"
        );
        assert_eq!(
            actor.state,
            State::from_action(expected.state << 8).unwrap(),
            "case {i}"
        );
        assert_eq!(actor.velocity, expected.velocity, "case {i}");
        assert_eq!(
            (actor.attack.contacts, actor.attack.trails),
            if expected.cleared {
                (0, 0)
            } else {
                (0x1234, 0x12)
            },
            "case {i}"
        );
    }
}
