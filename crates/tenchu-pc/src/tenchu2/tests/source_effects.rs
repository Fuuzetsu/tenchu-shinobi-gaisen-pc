use serde_json::Value;
use tenchu_assets::tenchu2::{effects::Rules, level::Level};
use tenchu_core::{
    math::{Math, Matrix, Position},
    random::Random,
    scene::Scene,
    tenchu2::{effects::Effects, terrain::Cursor, visibility::View},
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STORY_EFFECTS_ORACLE; original event dispatch, particles and RNG"]
fn native_story_effect_commands_match_original_dispatch_and_updates() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let rules = Rules::load(&disc).unwrap();
    let level = Level::load(&disc, 0).unwrap();
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_STORY_EFFECTS_ORACLE") {
        let mut effects = tenchu_core::tenchu2::story::Effects::default();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut scene = Scene::default();
        if let Some(bones) = row["bones"].as_u64() {
            let bones: Vec<_> = (0..bones)
                .map(|_| scene.add(Some(0), Matrix::default()))
                .collect();
            effects.ignite(&bones).unwrap();
        } else {
            let command = tenchu_assets::tenchu2::story::Command::Effect {
                kind: (row["id"].as_u64().unwrap() as u8).try_into().unwrap(),
                position: serde_json::from_value(row["position"].clone()).unwrap(),
                heading: row["heading"].as_i64().unwrap() as i16,
                value: 0,
            };
            effects
                .dispatch(
                    &command,
                    &math,
                    &rules,
                    &level.collision,
                    &mut random,
                    super::source_physics::vector(&row["direction"]),
                )
                .unwrap();
        }
        assert_eq!(
            random.seed,
            row["seed_after"].as_u64().unwrap() as u32,
            "case {count} RNG"
        );
        compare(
            &serde_json::to_value(&effects.particles.entries).unwrap(),
            &row["before"],
            &format!("dispatch {count}"),
        );
        for _ in 0..row["ticks"].as_u64().unwrap() {
            effects
                .particles
                .update(
                    &mut scene,
                    View {
                        eye: Position::ZERO,
                        look: Position::ZERO,
                        focus: Position::ZERO,
                        matrix: Matrix::default(),
                        distance: 0,
                    },
                    &level.collision,
                    &mut effects.cursor,
                    &math,
                )
                .unwrap();
        }
        compare(
            &serde_json::to_value(&effects.particles.entries).unwrap(),
            &row["after"],
            &format!("update {count}"),
        );
        count += 1;
    }
    assert_eq!(count, 210);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_AMBIENT_RIPPLE_ORACLE from JP 80047208"]
fn native_ambient_water_ripples_match_source_queries_and_rng() {
    use serde_json::json;
    use tenchu_core::tenchu2::effects::Effect;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut terrain = None;
    let mut map = usize::MAX;
    let mut checked = 0;
    let mut emitted = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_AMBIENT_RIPPLE_ORACLE") {
        let id = row["map"].as_u64().unwrap() as usize;
        if map != id {
            terrain = Some(Level::load(&disc, id).unwrap().collision);
            map = id;
        }
        let mut effects = Effects::default();
        let mut cursor = Cursor::default();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let player = super::source_physics::vector(&row["player"]);
        effects
            .ambient_ripple(
                player,
                View {
                    eye: super::source_physics::vector(&row["eye"]),
                    look: super::source_physics::vector(&row["look"]),
                    focus: player,
                    matrix: Matrix::default(),
                    distance: 18000,
                },
                terrain.as_ref().unwrap(),
                &mut cursor,
                &mut random,
            )
            .unwrap();
        let ripples: Vec<_> = effects
            .entries
            .iter()
            .map(|effect| {
                let Effect::Ripple(ripple) = effect else {
                    panic!("Unexpected ambient effect")
                };
                assert_eq!((ripple.age, ripple.lifetime), (0, 32));
                [
                    ripple.position.x,
                    ripple.position.y,
                    ripple.position.z,
                    ripple.speed,
                ]
            })
            .collect();
        let actual = json!({
            "seed": random.seed, "ripple": ripples,
            "current": cursor.current.map(|id| [id.cell, id.node]),
            "last": cursor.last.map(|id| [id.cell, id.node]),
            "material": cursor.material, "index": cursor.index,
        });
        assert_eq!(actual, row["expected"], "case {checked}, map {id}");
        checked += 1;
        emitted += ripples.len();
    }
    assert_eq!((checked, emitted), (4600, 1071));
}

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
                    b.contains_key(key.as_str())
                        || !matches!(key.as_str(), "contact" | "emission" | "blood")
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

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STORY_BLOOD_ORACLE from original EVENT.BIN dispatch and particle updates"]
fn native_story_blood_commands_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let rules = Rules::load(&disc).unwrap();
    let mut maps = std::collections::BTreeMap::new();
    let mut checked = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_STORY_BLOOD_ORACLE").enumerate()
    {
        let map = row["map"].as_u64().unwrap() as usize;
        let level = maps
            .entry(map)
            .or_insert_with(|| Level::load(&disc, map).unwrap());
        let mut state = tenchu_core::tenchu2::story::Effects::default();
        state.particles.blood_phase = row["phase"].as_bool().unwrap();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        for command in row["commands"].as_array().unwrap() {
            let id = command["id"].as_u64().unwrap() as u8;
            kinds.insert(id);
            state
                .dispatch(
                    &tenchu_assets::tenchu2::story::Command::Effect {
                        kind: id.try_into().unwrap(),
                        position: serde_json::from_value(command["position"].clone()).unwrap(),
                        heading: command["heading"].as_i64().unwrap() as i16,
                        value: command["value"].as_i64().unwrap() as i32,
                    },
                    &math,
                    &rules,
                    &level.collision,
                    &mut random,
                    Position::new(0, 0, 4096),
                )
                .unwrap();
        }
        assert_eq!(
            random.seed,
            row["seed_after"].as_u64().unwrap() as u32,
            "RNG {case}"
        );
        assert_eq!(
            state.particles.blood_phase,
            row["phase_after"].as_bool().unwrap(),
            "phase {case}"
        );
        compare(
            &serde_json::to_value(&state.particles.entries).unwrap(),
            &row["before"],
            &format!("story spawn {case}"),
        );
        let mut scene = Scene::default();
        let mut sounds = Vec::new();
        for (tick, draw) in row["draws"].as_array().unwrap().iter().enumerate() {
            if draw.as_bool().unwrap() {
                state.frame = state
                    .particles
                    .update(
                        &mut scene,
                        View {
                            eye: Position::ZERO,
                            look: Position::new(0, 0, 4096),
                            focus: Position::ZERO,
                            matrix: Matrix::default(),
                            distance: level.draw_distance,
                        },
                        &level.collision,
                        &mut state.cursor,
                        &math,
                    )
                    .unwrap();
                sounds.extend(state.frame.blood_sounds.iter().copied());
            }
            if tick == 4 {
                state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
            }
        }
        compare(
            &serde_json::to_value(&state.particles.entries).unwrap(),
            &row["after"],
            &format!("story update {case}"),
        );
        assert_eq!(
            serde_json::to_value(sounds).unwrap(),
            row["sounds"],
            "contacts {case}"
        );
        checked += 1;
    }
    assert_eq!(checked, 240);
    assert_eq!(kinds, [2, 3, 4, 25, 26, 29, 34].into());
}
