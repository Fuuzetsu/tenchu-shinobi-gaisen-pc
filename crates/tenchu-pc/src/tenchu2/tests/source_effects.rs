use serde_json::Value;
use tenchu_assets::tenchu2::{effects::Rules, level::Level};
use tenchu_core::{
    math::{Math, Matrix, Position},
    random::Random,
    scene::Scene,
    tenchu2::{effects::Effects, terrain::Cursor, visibility::View},
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SURFACE_EFFECTS_ORACLE from original water/blood routines"]
fn native_water_and_blood_contacts_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let rules = Rules::load(&disc).unwrap();
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_SURFACE_EFFECTS_ORACLE").enumerate()
    {
        let map = row["map"].as_u64().unwrap();
        let map = maps
            .entry(map)
            .or_insert_with(|| Level::load(&disc, map as usize).unwrap().collision);
        let mut effects = Effects::default();
        effects.blood_phase = row["phase"].as_bool().unwrap();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut cursor = Cursor::default();
        for command in row["commands"].as_array().unwrap() {
            let position = || {
                Position::from(
                    serde_json::from_value::<[i32; 3]>(command["position"].clone()).unwrap(),
                )
            };
            let int = |name: &str| command[name].as_i64().unwrap() as i32;
            match command["name"].as_str().unwrap() {
                "blood" => effects.blood(
                    serde_json::from_value(command.clone()).unwrap(),
                    &math,
                    &rules,
                    &mut random,
                ),
                "pool" => effects.death_pool(position(), map, &mut cursor),
                "ripple" => effects.ripple(position(), int("speed")),
                "splash" => effects.splash(position(), int("size"), &math, &mut random),
                "impact" => effects.impact(
                    position(),
                    Position::from(
                        serde_json::from_value::<[i32; 3]>(command["direction"].clone()).unwrap(),
                    ),
                    &math,
                    &rules,
                    &mut random,
                ),
                _ => panic!("Unknown surface effect command"),
            }
            .unwrap();
        }
        assert_eq!(
            random.seed,
            row["seed_after"].as_u64().unwrap() as u32,
            "seed {case}"
        );
        assert_eq!(
            effects.blood_phase,
            row["phase_after"].as_bool().unwrap(),
            "phase {case}"
        );
        compare(
            &serde_json::to_value(&effects.entries).unwrap(),
            &row["before"],
            &format!("spawn {case}"),
        );
        let mut scene = Scene::default();
        let view = View {
            eye: Position::ZERO,
            look: Position::ZERO,
            focus: Position::ZERO,
            matrix: Matrix {
                translation: Position::new(0, 0, -1000000),
                ..Matrix::default()
            },
            distance: 0,
        };
        let mut sounds = Vec::new();
        for _ in 0..row["ticks"].as_u64().unwrap() {
            sounds.extend(
                effects
                    .update(&mut scene, view, map, &mut cursor, &math)
                    .unwrap()
                    .blood_sounds,
            );
        }
        compare(
            &serde_json::to_value(&effects.entries).unwrap(),
            &row["after"],
            &format!("update {case}"),
        );
        compare(
            &serde_json::to_value(sounds).unwrap(),
            &row["sounds"],
            &format!("sounds {case}"),
        );
        count += 1;
    }
    assert_eq!(count, 1200);
}

fn compare(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path} length");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                compare(a, b, &format!("{path}/{i}"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let fields = || {
                a.iter().filter(|(key, _)| {
                    b.contains_key(key.as_str()) || !matches!(key.as_str(), "contact" | "emission")
                })
            };
            assert_eq!(fields().count(), b.len(), "{path} fields");
            for (key, value) in fields() {
                compare(value, &b[key], &format!("{path}/{key}"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}
