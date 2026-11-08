use tenchu_core::{
    random::Random,
    tenchu2::ai::{Program, Programs},
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_PATROL_ORACLE from the source route follower"]
fn native_patrol_routes_and_obstacle_responses_match_source() {
    use std::io::BufRead;
    use tenchu_assets::tenchu2::{Edition, layout::Waypoint, math_tables};
    use tenchu_core::{
        math::{Math, Position},
        tenchu2::ai::{Patrol, PatrolInput, Probe},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let edition = Edition::detect(&disc).unwrap();
    let math = Math {
        tables: math_tables(&disc).unwrap(),
    };
    let route = [
        Waypoint {
            x: 3000,
            z: 0,
            action: 1,
            wait: 20,
        },
        Waypoint {
            x: -7000,
            z: -2000,
            action: 2,
            wait: 0,
        },
        Waypoint {
            x: 500,
            z: 4000,
            action: 4,
            wait: 30,
        },
    ];
    let mut patrol = Patrol::new(&route, Position::ZERO);
    let mut rng = Random::new(0);
    let mut checked = 0;
    let data = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_PATROL_ORACLE").unwrap()).unwrap(),
    );
    for line in data.lines() {
        let line = line.unwrap();
        let fields: Vec<i64> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let [
            run,
            tick,
            seed,
            x,
            z,
            yaw,
            turn,
            action,
            health,
            held,
            flags,
            height,
            material,
            expected,
            motion,
            forced,
            next,
            wait,
            turning,
            next_seed,
            calls,
        ] = fields[..]
        else {
            panic!("Incomplete patrol trace")
        };
        if tick == 0 {
            patrol = Patrol::new(&route, Position::ZERO);
            rng = Random::new(seed as u32);
        }
        let actor = PatrolInput {
            position: Position::new(x as i32, 0, z as i32),
            yaw: yaw as i16,
            turn: turn as i16,
            action: action as i16,
            health: health as i16,
            held: held as u16,
            blocked: flags & 0x48000 != 0,
            touching_player: flags & 0x400000 != 0,
            in_water: flags & 0x2000 != 0,
            ahead: (height != i64::from(i32::MIN)).then_some(Probe {
                height: height as i32,
                material: material as u16,
            }),
        };
        let choice = patrol.update(edition, &actor, &math, &mut rng).unwrap();
        assert_eq!(
            (
                i64::from(choice.held),
                i64::from(choice.action.unwrap_or(-1))
            ),
            (expected, motion),
            "run {run} tick {tick}"
        );
        let override_input = choice.forced.map_or(0, |value| {
            (u32::from(value.held) << 16) | u32::from(value.remaining as u16)
        });
        assert_eq!(i64::from(override_input), forced, "run {run} tick {tick}");
        assert_eq!(
            (
                patrol.next as i64,
                i64::from(patrol.wait),
                i64::from(patrol.turning)
            ),
            (next, wait, turning),
            "run {run} tick {tick}"
        );
        assert_eq!(
            (i64::from(rng.seed), rng.calls as i64),
            (next_seed, calls),
            "run {run} tick {tick}"
        );
        checked += 1;
    }
    assert!(checked > 90000);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SENTRY_ORACLE from the source idle AI"]
fn sentry_turning_and_japanese_idle_actions_match_source() {
    use std::io::BufRead;
    use tenchu_assets::tenchu2::Edition;
    use tenchu_core::tenchu2::ai::{Sentry, SentryInput};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let edition = Edition::detect(&disc).unwrap();
    let input = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_SENTRY_ORACLE").unwrap()).unwrap(),
    );
    let mut sentry = Sentry::default();
    let mut rng = Random::new(0);
    let mut count = 0;
    let mut actions = 0;
    for line in input.lines() {
        let line = line.unwrap();
        let fields: Vec<i64> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let [
            run,
            tick,
            seed,
            turn,
            health,
            action,
            held,
            expected,
            motion,
            next_seed,
            calls,
        ] = fields[..]
        else {
            panic!("Incomplete sentry trace")
        };
        if tick == 0 {
            sentry = Sentry::default();
            rng = Random::new(seed as u32);
        }
        let (input, requested) = sentry
            .update(
                edition,
                SentryInput {
                    action: action as i16,
                    health: health as i16,
                    turn: turn as i16,
                    held: held as u16,
                },
                &mut rng,
            )
            .unwrap();
        assert_eq!(
            (i64::from(input), i64::from(requested.unwrap_or(-1))),
            (expected, motion),
            "run {run} tick {tick}"
        );
        assert_eq!(
            (i64::from(rng.seed), rng.calls as i64),
            (next_seed, calls),
            "run {run} tick {tick}"
        );
        actions += usize::from(requested.is_some());
        count += 1;
    }
    assert!(count > 300000);
    assert_eq!(actions > 0, edition == Edition::Japan);
}

#[test]
#[ignore = "requires TENCHU2_AI_ORACLE from the selected edition's AI assignment routine"]
fn ai_assignments_and_difficulty_randomness_match_source() {
    let data = std::fs::read_to_string(std::env::var_os("TENCHU2_AI_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    let mut random_calls = 0;
    let programs = |values: &[u32]| Programs {
        idle: Program::from_code(values[0] as u16).unwrap(),
        suspicious: Program::from_code(values[1] as u16).unwrap(),
        combat: Program::from_code(values[2] as u16).unwrap(),
        searching: Program::from_code(values[3] as u16).unwrap(),
    };
    for line in data.lines() {
        let values: Vec<u32> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(values.len(), 15);
        let [code, map, difficulty, seed, ..] = values[..] else {
            unreachable!()
        };
        let mut rng = Random::new(seed);
        let mut actual = programs(&values[4..8]);
        let active = actual
            .assign(code as u16, map as usize, difficulty as u8, &mut rng)
            .unwrap();
        assert_eq!(active, values[8] != 0, "row {checked}");
        assert_eq!(actual, programs(&values[11..]), "row {checked}");
        assert_eq!(
            (rng.seed, rng.calls),
            (values[9], u64::from(values[10])),
            "row {checked}"
        );
        random_calls += rng.calls;
        checked += 1;
    }
    assert!(checked > 30000);
    assert!(random_calls > 100);
}

#[test]
#[ignore = "requires TENCHU2_DISC; checks native cast and all placement AI programs"]
fn native_cast_and_ai_cover_the_source_layouts() {
    use tenchu_assets::tenchu2::{Edition, layout::Layout, registry::Registry};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    let total: usize = (0..24)
        .flat_map(|map| (0..3).map(move |hero| (hero, map)))
        .map(|(hero, map)| registry.cast(hero, map).unwrap().len())
        .sum();
    assert_eq!(
        total,
        if registry.edition == Edition::Europe {
            267
        } else {
            252
        }
    );
    let mut selected = 0;
    for layout in layouts {
        assert!(
            layout
                .actors
                .windows(2)
                .all(|actors| actors[0].slot < actors[1].slot)
        );
        for hero in 0..3 {
            let cast = registry.cast(hero, layout.map).unwrap();
            for actor in &layout.actors {
                if cast.contains(&actor.kind) {
                    assert!(registry.actor(actor.kind).is_some());
                    for difficulty in 1..=3 {
                        Programs::default()
                            .assign(actor.behavior, layout.map, difficulty, &mut Random::new(1))
                            .unwrap();
                    }
                    selected += 1;
                }
            }
        }
    }
    assert!(selected > 500);
}
