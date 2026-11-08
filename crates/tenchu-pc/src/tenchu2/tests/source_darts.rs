use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix, Position},
    scene::Scene,
    tenchu2::{
        collision::Collision,
        items::{Context, Event, dart::Dart},
        terrain::Cursor,
        visibility::View,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_DART_ORACLE; original poison-dart constructors, terrain/range and contacts"]
fn native_poison_darts_match_original_flight_range_and_contacts() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_DART_ORACLE").enumerate() {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let position = |key: &str| -> Position {
            serde_json::from_value::<[i32; 3]>(row[key].clone())
                .unwrap()
                .into()
        };
        let launched = Dart::launch(0, position("origin"), position("direction"), &math);
        assert_eq!(json!(launched), row["launch"], "launch {case}");
        let mut dart: Dart = serde_json::from_value(row["before"].clone()).unwrap();
        let expected: Dart = serde_json::from_value(row["after"].clone()).unwrap();
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let other_index = row["other_index"].as_u64().unwrap() as usize;
        let flags = row["other_flags"].as_u64().unwrap() as u16;
        let mut model = 0;
        for i in 0..3 {
            let translation = if i == 1 {
                dart.flight.position
            } else if i == other_index && flags != 0 {
                position("other_position")
            } else {
                Position::new(1_000_000, 1_000_000, 1_000_000)
            };
            let root = scene.add(
                None,
                Matrix {
                    translation,
                    ..Default::default()
                },
            );
            let slot = collision.insert(root).unwrap();
            if i == 1 {
                model = root;
                dart.register(root, &mut collision).unwrap();
            } else {
                slot.flags = if i == other_index { flags } else { 0 };
                slot.size = [4000; 3].into();
                slot.owner = Some(row["other_owner"].as_u64().unwrap() as usize);
            }
        }
        collision.compute(|model| scene.world(model)).unwrap();
        // Projection is a controlled boundary in this source fixture.
        let depth = row["depth"].as_i64().unwrap() as i32;
        let point = expected.flight.position;
        let view = View {
            eye: Position::ZERO,
            look: Position::ZERO,
            focus: Position::ZERO,
            distance: row["distance"].as_i64().unwrap() as i32,
            matrix: Matrix {
                translation: Position::new(
                    -point.x,
                    -point.y,
                    depth * 16 - point.z - (expected.flight.velocity.x.wrapping_neg() >> 4),
                ),
                ..Default::default()
            },
        };
        let mut cues = Vec::new();
        let alive = dart
            .advance(
                model,
                view,
                &mut Context {
                    map,
                    cursor: &mut Cursor::default(),
                    math: &math,
                    scene: &mut scene,
                    collision: &mut collision,
                    player: 0,
                },
                |event| {
                    match event {
                        Event::Splash(p) => cues.push(json!(["splash", p.to_array()])),
                        Event::Ribbon { .. } => {}
                        other => panic!("Unexpected dart event {other:?}"),
                    }
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(json!(alive), row["alive"], "alive {case}");
        assert_eq!(json!(dart), row["after"], "state {case}");
        assert_eq!(json!(cues), row["cues"], "events {case}");
        count += 1;
    }
    assert!(count >= 1500);
    eprintln!("{count} original poison-dart constructors and updates");
}
