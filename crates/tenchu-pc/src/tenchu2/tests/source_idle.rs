use serde::Deserialize;
use tenchu_core::{
    math::{Math, Position},
    random::Random,
    tenchu2::ai::{ForcedInput, Probe, SteeringInput, Wander, WanderInput, steer},
};

#[derive(Deserialize)]
struct Row {
    mode: String,
    seed: u32,
    home: [i32; 3],
    destination: [i32; 3],
    position: [i32; 3],
    yaw: i16,
    turn: i16,
    action: i16,
    health: i16,
    loops: i16,
    flags: u32,
    height: i32,
    material: u16,
    left: bool,
    right: bool,
    angle: i16,
    phase: u8,
    remaining: u8,
    turning: u8,
    forced: u32,
    held: u16,
    requested: i16,
    next_forced: u32,
    next_phase: u8,
    next_remaining: u8,
    next_turning: u8,
    next_destination: [i32; 3],
    next_seed: u32,
    calls: u64,
}

fn wander(phase: u8, remaining: u8, turning: u8) -> Wander {
    match phase {
        0 => Wander::Begin,
        1 => Wander::Walking,
        _ => Wander::Waiting { remaining, turning },
    }
}

fn packed(input: Option<ForcedInput>) -> u32 {
    input.map_or(0, |input| {
        (u32::from(input.held) << 16) | u32::from(input.remaining as u16)
    })
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_IDLE_ORACLE from the original idle/steering routines"]
fn wandering_steering_and_forced_inputs_match_source() {
    use std::io::BufRead;
    use tenchu_assets::tenchu2::{Edition, math_tables};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let edition = Edition::detect(&disc).unwrap();
    let math = Math {
        tables: math_tables(&disc).unwrap(),
    };
    let input = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_IDLE_ORACLE").unwrap()).unwrap(),
    );
    let mut counts = [0; 3];
    let mut actions = 0;
    for (index, line) in input.lines().enumerate() {
        let row: Row = serde_json::from_str(&line.unwrap()).unwrap();
        let mut random = Random::new(row.seed);
        let navigation = SteeringInput {
            yaw: row.yaw,
            turn: row.turn,
            target_angle: row.angle,
            alerted: row.flags & 3 != 0,
            blocked: row.flags & 0x48000 != 0,
            forced: row.forced != 0,
            ahead: (row.height != i32::MIN).then_some(Probe {
                height: row.height,
                material: row.material,
            }),
            left: row.left,
            right: row.right,
        };
        let choice = match row.mode.as_str() {
            "wander" => {
                counts[0] += 1;
                let mut state = wander(row.phase, row.remaining, row.turning);
                let mut destination = Position::from(row.destination);
                let result = state
                    .update(
                        edition,
                        &WanderInput {
                            position: row.position.into(),
                            home: row.home.into(),
                            action: row.action,
                            health: row.health,
                            loops: row.loops,
                            in_water: row.flags & 0x2000 != 0,
                            steering: navigation,
                        },
                        &mut destination,
                        &math,
                        &mut random,
                    )
                    .unwrap();
                assert_eq!(
                    state,
                    wander(row.next_phase, row.next_remaining, row.next_turning),
                    "row {index}"
                );
                assert_eq!(destination.to_array(), row.next_destination, "row {index}");
                result
            }
            "steer" => {
                counts[1] += 1;
                steer(
                    row.destination[0].wrapping_sub(row.position[0]),
                    row.destination[2].wrapping_sub(row.position[2]),
                    &navigation,
                    &math,
                )
                .unwrap()
            }
            "forced" => {
                counts[2] += 1;
                let (held, next) = ForcedInput {
                    held: (row.forced >> 16) as u16,
                    remaining: row.forced as i16,
                }
                .advance(&mut random);
                assert_eq!(held, row.held, "row {index}");
                assert_eq!(packed(next), row.next_forced, "row {index}");
                assert_eq!(
                    (random.seed, random.calls),
                    (row.next_seed, row.calls),
                    "row {index}"
                );
                continue;
            }
            _ => panic!("Unknown idle oracle mode"),
        };
        assert_eq!(choice.held, row.held, "row {index}");
        assert_eq!(choice.action.unwrap_or(-1), row.requested, "row {index}");
        assert_eq!(
            choice.forced.map_or(row.forced, |f| packed(Some(f))),
            row.next_forced,
            "row {index}"
        );
        assert_eq!(
            (random.seed, random.calls),
            (row.next_seed, row.calls),
            "row {index}"
        );
        actions += usize::from(choice.action.is_some());
    }
    assert!(counts.iter().all(|&n| n > 10000));
    assert_eq!(actions > 0, edition == Edition::Japan);
}
