use super::*;
use crate::{
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    tenchu2::rig::Rig,
};
use serde_json::{Value, json};
use std::io::BufRead;

#[test]
fn attached_fire_emission_survives_culling_without_changing_source_particles() {
    let mut source = sprites::Sprites::new(12, Position::new(20, -40, 60), 2, 8);
    source.size = [[1024; 2]; 2];
    source.velocity = [0, -8, 0].into();
    source.particles[1].age = -3;
    let mut lit = source.clone();
    lit.emission = Some(Emission::BurningActor);
    let mut hidden = lit.clone();
    let mut scene = Scene::default();
    let parent = scene.add(None, Matrix::default());
    for effect in [&mut source, &mut lit, &mut hidden] {
        effect.parent = Some(parent);
    }
    let view = View {
        eye: Position::ZERO,
        look: Position::new(0, 0, 1000),
        focus: Position::ZERO,
        matrix: Matrix::default(),
        distance: 16000,
    };
    let hidden_view = View {
        matrix: Matrix {
            translation: Position::new(0, 0, -20000),
            ..Matrix::default()
        },
        ..view
    };
    let mut rotations = [Vec::new(), Vec::new(), Vec::new()];
    let mut saw_emission = false;
    for tick in 0..14 {
        let position = Position::new(tick * 25, 0, 4000);
        scene.set_translation(parent, position);
        let mut images = [Vec::new(), Vec::new(), Vec::new()];
        let mut emitters = [Vec::new(), Vec::new(), Vec::new()];
        for (index, (effect, camera)) in [
            (&mut source, view),
            (&mut lit, view),
            (&mut hidden, hidden_view),
        ]
        .into_iter()
        .enumerate()
        {
            effect
                .update(
                    &mut scene,
                    camera,
                    &mut rotations[index],
                    &mut images[index],
                    &mut emitters[index],
                )
                .unwrap();
        }
        assert_eq!(images[0], images[1], "lighting changed the source sprite");
        assert!(images[2].is_empty(), "off-screen fixture was still visible");
        assert!(emitters[0].is_empty());
        assert_eq!(emitters[1], emitters[2], "camera culled a light emitter");
        assert!(emitters[1].iter().all(|e| e.position.x == position.x + 20));
        saw_emission |= !emitters[1].is_empty();
        let mut untagged = lit.clone();
        untagged.emission = None;
        assert_eq!(
            serde_json::to_value(&source).unwrap(),
            serde_json::to_value(&untagged).unwrap()
        );
        if tick == 13 {
            assert!(emitters[1].is_empty(), "light outlived particles");
        }
    }
    assert!(saw_emission);
}

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
        powder_colors: serde_json::from_value(tables["powder_colors"].clone()).unwrap(),
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
                "dispersing" => effects.dispersing_smoke(
                    position(),
                    Position::from(
                        serde_json::from_value::<[i32; 3]>(command["velocity"].clone()).unwrap(),
                    ),
                    kind(),
                    &mut random,
                ),
                "smoke" => effects.smoke(position(), kind(), hidden, &math, &mut random),
                "sleep_gas" => {
                    effects.sleep_gas(position(), command["phase"].as_i64().unwrap() as i16, &math)
                }
                "powder" => effects.confusion_powder(
                    position(),
                    command["yaw"].as_i64().unwrap() as i16,
                    &math,
                    &rules,
                ),
                "burn" => effects.burn(
                    &rig,
                    command["interval"].as_i64().unwrap() as i16,
                    command["duration"].as_i64().unwrap() as i16,
                    &rules,
                ),
                "flash" => effects.flash(position(), kind() != 0),
                "explosion" => effects.explosion(position(), kind(), &math, &mut random),
                "fuse" => effects.fuse(position(), &rules, &mut random),
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
                            &mut effects.sprite_rotations,
                            &mut drawings,
                            &mut Vec::new(),
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
            // Presentation metadata has no counterpart in the source particle structure.
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
        let draws: Vec<_> = frame.quads.iter().map(|q| json!({"image":match q.image { surface::Image::Effect(id) => id, _ => panic!("Unexpected item texture") },"position":q.position,"corners":q.corners.map(|p|p.to_array()),"color":q.color,"blend":format!("{:?}",q.blend)})).collect();
        compare(
            &Value::Array(draws),
            &row["draws"],
            &format!("geometry {case}"),
        );
        count += 1;
    }
    assert_eq!(count, 1000);
}
