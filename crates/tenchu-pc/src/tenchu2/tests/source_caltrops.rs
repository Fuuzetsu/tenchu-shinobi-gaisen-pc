use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Matrix, Position, Vector3},
    random::Random,
    scene::Scene,
    tenchu2::{
        collision::Collision,
        items::{Context, Event, caltrop::Caltrop},
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CALTROP_ORACLE; original JP scatter, terrain and contact callbacks"]
fn native_caltrops_match_original_scatter_landings_and_contacts() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = super::source_control::resources();
    let mut actor = super::source_control::template(&resources);
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_CALTROP_ORACLE").enumerate() {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let position = |key: &str| -> Position {
            serde_json::from_value::<[i32; 3]>(row[key].clone())
                .unwrap()
                .into()
        };
        actor.position = position("actor");
        actor.rotation.y = row["yaw"].as_i64().unwrap() as i16;
        actor.motion.action = row["action"].as_i64().unwrap() as i16;
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let launched = Caltrop::scatter(
            0,
            position("origin"),
            &actor,
            position("direction"),
            &resources.math,
            &mut random,
        );
        assert_eq!(json!(launched), row["launch"], "launch {case}");
        assert_eq!(json!(random.seed), row["after_seed"], "scatter RNG {case}");

        let mut caltrop: Caltrop = serde_json::from_value(row["before"].clone()).unwrap();
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let other_index = row["other_index"].as_u64().unwrap() as usize;
        let flags = row["other_flags"].as_u64().unwrap() as u16;
        let mut model = 0;
        for i in 0..3 {
            let translation = if i == 1 {
                caltrop.flight.position
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
                caltrop.register(model, &mut collision).unwrap();
            } else {
                slot.flags = if i == other_index { flags } else { 0 };
                slot.size = Vector3::new(4000, 4000, 4000);
                slot.owner = Some(1);
            }
        }
        collision.compute(|model| scene.world(model)).unwrap();
        let mut cues = Vec::new();
        let alive = caltrop
            .advance(
                model,
                &mut Context {
                    map,
                    cursor: &mut Cursor::default(),
                    math: &resources.math,
                    scene: &mut scene,
                    collision: &mut collision,
                    player: 0,
                },
                |event| {
                    cues.push(match event {
                        Event::Splash(p) => json!(["splash", p.to_array()]),
                        Event::Sound { position, id } => json!(["sound", position.to_array(), id]),
                        other => panic!("Unexpected caltrop event {other:?}"),
                    });
                    Ok(())
                },
            )
            .unwrap();
        if !alive {
            collision.remove(model);
        }
        assert_eq!(json!(alive), row["alive"], "alive {case}");
        assert_eq!(json!(caltrop), row["after"], "state {case}");
        assert_eq!(json!(cues), row["cues"], "events {case}");
        let slot = collision.slot(1).unwrap();
        assert_eq!(json!(slot.flags), row["flags"], "flags {case}");
        assert_eq!(json!(slot.owner), row["owner"], "owner {case}");
        count += 1;
    }
    assert!(count >= 1000);
    eprintln!("Original caltrop cases: {count}");
}
