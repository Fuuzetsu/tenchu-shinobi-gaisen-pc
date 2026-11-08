use super::source_physics::vector;
use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::Matrix,
    scene::Scene,
    tenchu2::{
        actor::State,
        ai::runtime::Agent,
        collision::Collision,
        items::{
            Context, Event,
            rice::{self, Rice},
        },
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_RICE_ORACLE; original JP rice throw and terrain callbacks"]
fn native_rice_matches_original_launch_and_landings() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = super::source_control::resources();
    let mut actor = super::source_control::template(&resources);
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_RICE_ORACLE").enumerate() {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        actor.position = vector(&row["actor"]);
        actor.rotation.y = row["yaw"].as_i64().unwrap() as i16;
        actor.motion.action = row["action"].as_i64().unwrap() as i16;
        let mut scene = Scene::default();
        let model = scene.add(None, Matrix::default());
        let mut collision = Collision::default();
        let mut cursor = Cursor::default();
        let mut context = Context {
            map,
            cursor: &mut cursor,
            math: &resources.math,
            scene: &mut scene,
            collision: &mut collision,
            player: 0,
        };
        let rice = Rice::launch(
            0,
            vector(&row["origin"]),
            &actor,
            vector(&row["direction"]),
            &mut context,
        )
        .unwrap();
        let mut actual = json!(rice);
        actual.as_object_mut().unwrap().remove("floor_initialized");
        assert_eq!(actual, row["launch"], "launch {case}");
        let mut initial = row["before"].clone();
        initial["floor_initialized"] = json!(rice.floor_initialized);
        let mut rice: Rice = serde_json::from_value(initial).unwrap();
        rice.register(model, context.collision).unwrap();
        let mut cues = Vec::new();
        let alive = rice
            .advance(model, &mut context, |event| {
                match event {
                    Event::Splash(p) => cues.push(json!(["splash", p.to_array()])),
                    other => panic!("Unexpected rice event {other:?}"),
                }
                Ok(())
            })
            .unwrap();
        if !alive {
            context.collision.remove(model);
        }
        let mut actual = json!(rice);
        actual.as_object_mut().unwrap().remove("floor_initialized");
        if !alive {
            actual["transform"]["translation"] = actual["flight"]["position"].clone();
        }
        assert_eq!(actual, row["after"], "state {case}");
        assert_eq!(json!(alive), row["alive"], "alive {case}");
        assert_eq!(json!(cues), row["cues"], "events {case}");
        let slot = context.collision.slot(0).unwrap();
        assert_eq!(json!(slot.owner), row["owner"], "owner {case}");
        assert_eq!(json!(slot.flags), row["flags"], "flags {case}");
        count += 1;
    }
    assert!(count >= 1400);
    eprintln!("Original rice launch/update cases: {count}");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_RICE_TARGET_ORACLE; original JP bait search with real map terrain"]
fn native_rice_targets_match_original_distance_and_cover_checks() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = super::source_control::resources();
    let template = super::source_control::template(&resources);
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_RICE_TARGET_ORACLE").enumerate()
    {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let actors: Vec<_> = row["actors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| {
                let mut actor = template.clone();
                actor.position = vector(&a["position"]);
                actor.flags = a["flags"].as_u64().unwrap() as u32;
                actor.state =
                    State::from_action((a["state"].as_i64().unwrap() as i16) << 8).unwrap();
                Agent::new(actor, 0, 10000, &[])
            })
            .collect();
        let target = rice::nearest(
            0,
            vector(&row["position"]),
            &actors,
            map,
            &mut Cursor::default(),
            &resources.math,
        )
        .unwrap();
        assert_eq!(json!(target), row["target"], "target {case}");
        count += 1;
    }
    assert!(count >= 1000);
    eprintln!("Original rice target cases: {count}");
}
