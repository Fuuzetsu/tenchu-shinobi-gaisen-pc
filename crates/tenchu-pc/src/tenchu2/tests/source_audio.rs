use serde_json::{Value, json};
use tenchu_core::{
    math::{Math, Position},
    random::Random,
    tenchu2::audio::{Audio, Listener},
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_AUDIO_ORACLE from JP sound dispatch and ambience"]
fn native_audio_matches_spatial_voices_cooldowns_and_ambient_rng() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut counts = [0; 3];
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_AUDIO_ORACLE").enumerate() {
        let mut audio: Audio = serde_json::from_value(row["before"].clone()).unwrap();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let tick = row["tick"].as_u64().unwrap() as u32;
        let sound = match row["kind"].as_str().unwrap() {
            "ambient" => {
                counts[0] += 1;
                audio.ambient(
                    row["map"].as_u64().unwrap() as usize,
                    row["interval"].as_i64().unwrap() as i16,
                    tick,
                    &mut random,
                    &math,
                )
            }
            "play" => {
                counts[1] += 1;
                audio.play(
                    row["id"].as_u64().unwrap() as u8,
                    row["direction"].as_i64().unwrap() as i32,
                    row["volume"].as_u64().unwrap() as u8,
                    tick,
                    &math,
                )
            }
            "spatial" => {
                counts[2] += 1;
                let listener = Listener {
                    actor: 0,
                    position: Position::from(
                        serde_json::from_value::<[i32; 3]>(row["listener"].clone()).unwrap(),
                    ),
                    yaw: row["yaw"].as_i64().unwrap() as i16,
                    distance: row["distance"].as_i64().unwrap() as i32,
                };
                let position = serde_json::from_value::<Option<[i32; 3]>>(row["position"].clone())
                    .unwrap()
                    .map(Position::from);
                listener
                    .parameters(position, &math)
                    .unwrap()
                    .and_then(|(direction, volume)| {
                        audio.play(
                            row["id"].as_u64().unwrap() as u8,
                            direction,
                            volume,
                            tick,
                            &math,
                        )
                    })
            }
            _ => panic!("Unknown audio case"),
        };
        assert_eq!(
            sound
                .map(|s| json!({"id":s.id,"slot":s.slot,"note":s.note,"volume":s.volume}))
                .unwrap_or(Value::Null),
            row["sound"],
            "sound case {case}"
        );
        assert_eq!(
            serde_json::to_value(audio).unwrap(),
            row["after"],
            "state case {case}"
        );
        assert_eq!(
            u64::from(random.seed),
            row["seed_after"].as_u64().unwrap(),
            "RNG case {case}"
        );
    }
    assert!(counts.into_iter().all(|n| n > 2000));
    eprintln!("JP audio: {counts:?} ambient, direct and spatial cases");
}
