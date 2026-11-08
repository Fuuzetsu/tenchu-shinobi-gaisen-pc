use super::*;
use crate::{
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    tenchu2::rig::Rig,
};
use serde_json::{Value, json};
use std::io::BufRead;

#[test]
#[ignore = "requires TENCHU2_SPRITE_ORACLE and TENCHU2_EFFECT_TABLES from JP executable particle routines"]
fn particles_match_source_spawning_eviction_animation_and_expiry() {
    let tables: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("TENCHU2_EFFECT_TABLES").unwrap()).unwrap(),
    )
    .unwrap();
    let math = Math {
        tables: tenchu_assets::executable::MathTables {
            trig: serde_json::from_value(tables["trig"].clone()).unwrap(),
            atan: Vec::new(),
            sqrt: Vec::new(),
            normal: Vec::new(),
            hermite: [[0; 4]; 33],
        },
    };
    let rules = tenchu_assets::tenchu2::effects::Rules {
        burn_bones: serde_json::from_value(tables["burn_bones"].clone()).unwrap(),
        healing_bones: serde_json::from_value(tables["healing_bones"].clone()).unwrap(),
        point_weights: serde_json::from_value(tables["point_weights"].clone()).unwrap(),
        point_shifts: serde_json::from_value(tables["point_shifts"].clone()).unwrap(),
    };
    let file = std::fs::File::open(std::env::var_os("TENCHU2_SPRITE_ORACLE").unwrap()).unwrap();
    let mut count = 0;
    for (case, line) in std::io::BufReader::new(file).lines().enumerate() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let mut effects = Effects::default();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let hidden = row["hidden"].as_bool().unwrap();
        let mut scene = Scene::default();
        let rig = Rig {
            root: 0,
            joints: (0..row["bones"].as_u64().unwrap())
                .map(|_| scene.add(None, Matrix::default()))
                .collect(),
            weapons: [None; 5],
        };
        for command in row["commands"].as_array().unwrap() {
            let position = || {
                Position::from(
                    serde_json::from_value::<[i32; 3]>(command["position"].clone()).unwrap(),
                )
            };
            let kind = || command["kind"].as_u64().unwrap() as u8;
            match command["name"].as_str().unwrap() {
                "trail" => effects.trail(
                    position(),
                    Vector3::from(
                        serde_json::from_value::<[i16; 3]>(command["velocity"].clone()).unwrap(),
                    ),
                    kind(),
                    hidden,
                    &mut random,
                ),
                "smoke" => effects.smoke(position(), kind(), hidden, &math, &mut random),
                "burn" => effects.burn(
                    &rig,
                    command["interval"].as_i64().unwrap() as i16,
                    command["duration"].as_i64().unwrap() as i16,
                    &rules,
                ),
                "flash" => effects.flash(position(), kind() != 0),
                "explosion" => effects.explosion(position(), kind(), &math, &mut random),
                "heal" => effects.heal(&rig, kind(), &rules, &mut random),
                _ => panic!("Unknown particle command"),
            }
            .unwrap();
        }
        assert_eq!(
            random.seed,
            row["seed_after"].as_u64().unwrap() as u32,
            "RNG {case}"
        );
        compare(
            &serde_json::to_value(&effects.entries).unwrap(),
            &row["before"],
            &format!("spawn {case}"),
        );
        let mut drawings = Vec::new();
        let view = View {
            eye: Position::ZERO,
            look: Position::new(0, 0, 1000),
            focus: Position::ZERO,
            matrix: Matrix::default(),
            distance: 16000,
        };
        for _ in 0..row["ticks"].as_u64().unwrap() {
            let mut index = 0;
            while index < effects.entries.len() {
                let alive = match &mut effects.entries[index] {
                    Effect::Sprites(effect) => effect
                        .advance(
                            |p| (p, row["depth"].as_i64().unwrap() as i32),
                            &mut drawings,
                        )
                        .unwrap(),
                    Effect::Points(effect) => {
                        effect.update(&mut scene, view, &mut Vec::new()).unwrap()
                    }
                    _ => panic!("Unexpected surface effect"),
                };
                if alive {
                    index += 1;
                } else {
                    effects.entries.remove(index);
                }
            }
        }
        compare(
            &serde_json::to_value(&effects.entries).unwrap(),
            &row["after"],
            &format!("animation {case}"),
        );
        let actual: Vec<_> = drawings.iter().map(|d| json!({"image":d.image,"position":d.position,"scale":d.scale,"color":d.color,"rotation":d.rotation,"depth":d.depth})).collect();
        compare(
            &Value::Array(actual),
            &row["draws"],
            &format!("drawing {case}"),
        );
        count += 1;
    }
    assert_eq!(count, 600);
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
            assert_eq!(a.len(), b.len(), "{path} fields");
            for (key, value) in a {
                compare(value, &b[key], &format!("{path}/{key}"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

#[test]
#[ignore = "requires TENCHU2_SURFACE_DRAWING_ORACLE from original sloped water/blood primitives"]
fn surface_geometry_and_fading_match_source() {
    let file =
        std::fs::File::open(std::env::var_os("TENCHU2_SURFACE_DRAWING_ORACLE").unwrap()).unwrap();
    let mut count = 0;
    let map = tenchu_assets::tenchu2::collision::Collision {
        origin: [0; 2],
        dimensions: [1; 2],
        cell_size: 1000,
        cells: Vec::new(),
    };
    let view = View {
        eye: Position::ZERO,
        look: Position::ZERO,
        focus: Position::ZERO,
        matrix: Matrix::default(),
        distance: 16000,
    };
    for (case, line) in std::io::BufReader::new(file).lines().enumerate() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let mut effect: Effect = serde_json::from_value(row["before"].clone()).unwrap();
        let mut frame = Frame::default();
        match &mut effect {
            Effect::Ripple(ripple) => ripple.update(&mut frame.quads),
            Effect::Pool(pool) => pool.update(&mut frame.quads),
            Effect::Blood(blood) => blood.update(
                &mut Scene::default(),
                view,
                &map,
                &mut Cursor::default(),
                &mut frame,
            ),
            _ => panic!("Unexpected effect"),
        }
        .unwrap();
        let draws: Vec<_> = frame.quads.iter().map(|q| json!({"image":q.image,"position":q.position,"corners":q.corners.map(|p|p.to_array()),"color":q.color,"blend":format!("{:?}",q.blend)})).collect();
        compare(
            &Value::Array(draws),
            &row["draws"],
            &format!("geometry {case}"),
        );
        count += 1;
    }
    assert_eq!(count, 1000);
}
