use serde::Deserialize;
use std::io::BufRead;
use tenchu_assets::{
    model::Part,
    tenchu2::{commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons},
};
use tenchu_core::{
    math::Math,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        control::{Affliction, Request, TargetQuery, find_target},
    },
};

pub(super) fn resources() -> Resources {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    Resources {
        clips: motions::collect(&disc, registry.clip(0, 0).into_iter().collect()).unwrap(),
        registry,
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    }
}

pub(super) fn template(resources: &Resources) -> Actor {
    let parts: Vec<_> = (0..20)
        .map(|child| Part {
            parent: if child == 0 { -1 } else { 0 },
            child,
            position: [0; 3],
            geometry: vec![],
        })
        .collect();
    Actor::new(0, &parts, resources).unwrap()
}

pub(super) fn cases<T: serde::de::DeserializeOwned>(variable: &str) -> impl Iterator<Item = T> {
    std::io::BufReader::new(std::fs::File::open(std::env::var_os(variable).unwrap()).unwrap())
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
}

pub(super) fn velocity(value: i16) -> Velocity {
    match value {
        -1 => Velocity::Stop,
        0 => Velocity::Keep,
        1 => Velocity::Motion,
        _ => panic!("Unexpected velocity mode"),
    }
}

#[derive(Deserialize)]
struct StatusExpected {
    flags: u32,
    health: i16,
    action: i16,
    velocity: i16,
    seed: u32,
    effect: u8,
    random: u64,
    calls: Vec<u8>,
}

#[derive(Deserialize)]
struct StatusCase {
    kind: i16,
    flags: u32,
    health: i16,
    skill: i16,
    frame: i16,
    loops: i16,
    state: i16,
    action: i16,
    velocity: i16,
    player: bool,
    enabled: bool,
    material: u16,
    seed: u32,
    expected: StatusExpected,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STATUS_ORACLE from original status controllers"]
fn native_afflictions_preserve_interrupts_damage_and_rng() {
    let resources = resources();
    let template = template(&resources);
    let mut checked = 0;
    for (index, case) in cases::<StatusCase>("TENCHU2_STATUS_ORACLE").enumerate() {
        let mut actor = template.clone();
        actor.kind = case.kind;
        actor.flags = case.flags;
        actor.health = case.health;
        actor.skill = case.skill;
        actor.motion.frame = case.frame;
        actor.motion.loops = case.loops;
        actor.state = State::from_action(case.state << 8).unwrap();
        actor.terrain.material = case.material;
        let mut random = Random::new(case.seed);
        let mut request = Request {
            action: case.action,
            velocity: velocity(case.velocity),
        };
        let result = actor.update_status(&mut request, &mut random, case.player, case.enabled);
        let expected = case.expected;
        assert_eq!(actor.flags, expected.flags, "case {index}");
        assert_eq!(actor.health, expected.health, "case {index}");
        assert_eq!(
            request,
            Request {
                action: expected.action,
                velocity: velocity(expected.velocity)
            },
            "case {index}"
        );
        assert_eq!(
            random,
            Random {
                seed: expected.seed,
                calls: expected.random
            },
            "case {index}"
        );
        assert_eq!(
            result.effect,
            match expected.effect {
                0 => None,
                1 => Some(Affliction::Burning),
                2 => Some(Affliction::Poison),
                3 => Some(Affliction::Confusion),
                _ => panic!("Unexpected status result"),
            },
            "case {index}"
        );
        assert_eq!(result.died, expected.calls.contains(&0), "case {index}");
        assert_eq!(
            result.show_health,
            expected.calls.contains(&4),
            "case {index}"
        );
        assert_eq!(
            result.staggered,
            expected.calls.contains(&5),
            "case {index}"
        );
        checked += 1;
    }
    assert_eq!(checked, 18_000);
}

#[derive(Deserialize)]
struct TargetActor {
    kind: i16,
    flags: u32,
    health: i16,
    position: [i32; 3],
}

#[derive(Deserialize)]
struct TargetCase {
    actors: Vec<TargetActor>,
    origin: usize,
    player: Option<usize>,
    distance: i32,
    expected: Option<usize>,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_TARGET_ORACLE from original target selection"]
fn native_target_selection_keeps_source_filters_order_and_distances() {
    let resources = resources();
    let template = template(&resources);
    let mut checked = 0;
    for (index, case) in cases::<TargetCase>("TENCHU2_TARGET_ORACLE").enumerate() {
        let actors: Vec<_> = case
            .actors
            .into_iter()
            .map(|input| {
                let mut actor = template.clone();
                actor.kind = input.kind;
                actor.flags = input.flags;
                actor.health = input.health;
                actor.position = input.position.into();
                actor
            })
            .collect();
        let distance = case.distance.wrapping_abs();
        let query = if distance as u32 <= 255 {
            TargetQuery::Kind(distance as u8)
        } else {
            TargetQuery::Nearby {
                distance,
                enemies_only: case.distance < 0,
            }
        };
        assert_eq!(
            find_target(
                actors.len(),
                |index| actors.get(index),
                case.origin,
                case.player,
                query,
                &resources.math
            ),
            case.expected,
            "case {index}"
        );
        checked += 1;
    }
    assert_eq!(checked, 6_000);
}
