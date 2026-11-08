use serde_json::{Value, json};
use tenchu_assets::tenchu2::{camera::Tables, level::Level};
use tenchu_core::{
    math::Matrix,
    random::Random,
    tenchu2::{
        camera::{Camera, Context, Subject},
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CAMERA_ORACLE from the Japanese camera and terrain routines"]
fn native_camera_modes_obstruction_smoothing_and_aim_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = super::source_control::resources();
    let tables = Tables::load(&disc).unwrap();
    let mut actor = super::source_control::template(&resources);
    let mut level = None;
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_CAMERA_ORACLE") {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let map = int("map") as usize;
        if level.as_ref().is_none_or(|level: &Level| level.id != map) {
            level = Some(Level::load(&disc, map).unwrap());
        }
        actor.kind = int("kind") as i16;
        actor.position = super::source_physics::vector(&row["position"]);
        actor.rotation = super::source_physics::vector::<i16>(&row["rotation"]);
        actor.motion.action = int("action") as i16;
        actor.motion.frame = int("frame") as i16;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.pad.held = int("held") as u16;
        let mut state = row["before"].clone();
        for key in ["eye", "look"] {
            state[key] = json!(super::source_physics::vector::<i32>(&state[key]));
        }
        state["view"] = json!(Matrix::default());
        let mut camera: Camera = serde_json::from_value(state).unwrap();
        let mut random = Random::new(int("seed") as u32);
        let mut cursor = Cursor::default();
        camera
            .advance(
                &Subject {
                    actor: &actor,
                    root: super::source_physics::vector(&row["root"]),
                    target: row["target"].as_bool().unwrap().then(|| {
                        (
                            super::source_physics::vector(&row["target_root"]),
                            super::source_physics::vector::<i16>(&row["target_rotation"]),
                        )
                    }),
                },
                &mut Context {
                    map: &level.as_ref().unwrap().collision,
                    cursor: &mut cursor,
                    math: &resources.math,
                    tables: &tables,
                    hero: int("hero") as usize,
                    sequence: int("sequence") as u16,
                },
                &mut random,
            )
            .unwrap_or_else(|error| panic!("case {count}: {error}"));
        let mut result = json!(camera);
        result.as_object_mut().unwrap().remove("view");
        result["eye"] = json!(camera.eye.to_array());
        result["look"] = json!(camera.look.to_array());
        result["seed"] = json!(random.seed);
        result["current"] = json!(cursor.current.map(|id| [id.cell, id.node]));
        result["last"] = json!(cursor.last.map(|id| [id.cell, id.node]));
        result["material"] = json!(cursor.material);
        result["index"] = json!(cursor.index);
        for (key, expected) in row["expected"].as_object().unwrap() {
            assert_eq!(
                &result[key], expected,
                "case {count}, map {map}, field {key}, input {row}"
            );
        }
        count += 1;
    }
    assert!(count >= 5000);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CAMERA_VIEW_ORACLE from GsSetRefView2"]
fn native_camera_projection_matrix_matches_the_source_sdk() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = super::source_control::resources();
    let tables = Tables::load(&disc).unwrap();
    let level = Level::load(&disc, 0).unwrap();
    let actor = super::source_control::template(&resources);
    let mut camera = Camera::new(&tables);
    camera.mode = 22;
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_CAMERA_VIEW_ORACLE") {
        camera.eye = super::source_physics::vector(&row["eye"]);
        camera.look = super::source_physics::vector(&row["look"]);
        camera.roll = row["roll"].as_i64().unwrap() as i32;
        camera
            .advance(
                &Subject {
                    actor: &actor,
                    root: actor.position,
                    target: None,
                },
                &mut Context {
                    map: &level.collision,
                    cursor: &mut Cursor::default(),
                    math: &resources.math,
                    tables: &tables,
                    hero: 0,
                    sequence: 1,
                },
                &mut Random::new(1),
            )
            .unwrap();
        assert_eq!(
            json!(camera.view.rotation),
            row["rotation"],
            "case {count}: {row}"
        );
        assert_eq!(
            json!(camera.view.translation.to_array()),
            row["translation"],
            "case {count}: {row}"
        );
        count += 1;
    }
    assert!(count >= 2000);
}
