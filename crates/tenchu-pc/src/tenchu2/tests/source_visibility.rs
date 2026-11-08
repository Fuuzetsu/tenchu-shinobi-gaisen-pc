use serde_json::Value;
use tenchu_assets::tenchu2::level::{
    Level,
    objects::{Bounds, Static, StaticKind},
};
use tenchu_core::{
    math::{Math, Matrix, Position},
    scene::Scene,
    tenchu2::{
        collision::Collision,
        environment::{Environment, Object},
        visibility::View,
    },
};

#[test]
#[ignore = "requires TENCHU2_ACTOR_VISIBILITY_ORACLE from JP actor projection"]
fn native_actor_projection_matches_source() {
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_ACTOR_VISIBILITY_ORACLE") {
        let view = View {
            eye: Position::ZERO,
            look: Position::ZERO,
            focus: Position::ZERO,
            distance: row["distance"].as_i64().unwrap() as i32,
            matrix: Matrix {
                rotation: serde_json::from_value(row["rotation"].clone()).unwrap(),
                translation: super::source_physics::vector(&row["translation"]),
            },
        };
        assert_eq!(
            view.actor(&Matrix::default()).map_or(-1, i32::from),
            row["expected"].as_i64().unwrap() as i32,
            "case {count}: {row}"
        );
        count += 1;
    }
    assert!(count >= 12000);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SCENERY_VISIBILITY_ORACLE from JP static/dynamic scenery routines"]
fn native_scenery_activation_and_camera_culling_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut level = Level::load(&disc, 0).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    level.scene.bounds = vec![
        Bounds {
            offset: [0; 3],
            top: -1000,
            half_width: 250,
            half_depth: 250,
        };
        10
    ];
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_SCENERY_VISIBILITY_ORACLE") {
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let mut environment = Environment::default();
        let vector = super::source_physics::vector::<i32>;
        let view = View {
            eye: vector(&row["eye"]),
            look: vector(&row["look"]),
            focus: vector(&row["focus"]),
            distance: row["distance"].as_i64().unwrap() as i32,
            matrix: Matrix {
                rotation: serde_json::from_value(row["rotation"].clone()).unwrap(),
                translation: Position::ZERO,
            },
        };
        level.object_sizes = serde_json::from_value(row["sizes"].clone()).unwrap();
        level.grid.cells.clear();
        for (index, object) in row["objects"].as_array().unwrap().iter().enumerate() {
            let position = vector(&object["position"]);
            let coordinate = scene.add(
                None,
                Matrix {
                    translation: position,
                    ..Matrix::default()
                },
            );
            collision.insert(coordinate).unwrap();
            environment.objects.push(Object {
                cell: index,
                coordinate,
                position,
                model: usize::try_from(object["model"].as_i64().unwrap()).ok(),
                kind: object["kind"].as_i64().unwrap() as i16,
                link: object["link"].as_i64().unwrap() as i16,
                clock: object["clock"].as_i64().unwrap() as i16,
                yaw: object["yaw"].as_i64().unwrap() as i16,
                roll: 0,
                actor: None,
            });
            level
                .grid
                .cells
                .push(object["resident"].as_bool().unwrap().then_some(index));
        }
        level.scene.statics = row["statics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|object| Static {
                position: vector(&object["position"]).to_array(),
                rotation: [0; 3],
                kind: if let Some(kind) = object["kind"].as_u64() {
                    StaticKind::Effect(kind as u16)
                } else {
                    StaticKind::Model(object["model"].as_u64().unwrap() as usize)
                },
            })
            .collect();
        let visible = environment
            .activate(&level, &view, &mut collision, &math)
            .unwrap();
        let expected = &row["expected"];
        let statics: Vec<_> = expected["statics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event| {
                if event[0] == "draw" {
                    event[1].as_u64().unwrap() as usize
                } else {
                    level
                        .scene
                        .statics
                        .iter()
                        .position(|s| s.position == vector(&event[2]).to_array())
                        .unwrap()
                }
            })
            .collect();
        assert_eq!(visible.statics, statics, "case {count}: {row}");
        let dynamic: Vec<_> = expected["dynamic"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event[0] == "draw" || event[0] == "fall")
            .map(|event| event[1].as_u64().unwrap() as usize)
            .collect();
        assert_eq!(visible.dynamic, dynamic, "case {count}: {row}");
        for event in expected["dynamic"].as_array().unwrap() {
            if event[0] == "enable" || event[0] == "remove" {
                let object = &environment.objects[event[1].as_u64().unwrap() as usize];
                assert_eq!(
                    collision.index(object.coordinate).is_some(),
                    event[0] == "enable",
                    "case {count}: {event}"
                );
            }
        }
        assert_eq!(
            serde_json::to_value(
                environment
                    .objects
                    .iter()
                    .map(|o| o.clock)
                    .collect::<Vec<_>>()
            )
            .unwrap(),
            expected["clocks"],
            "case {count}"
        );
        count += 1;
    }
    assert!(count >= 4000);
}
