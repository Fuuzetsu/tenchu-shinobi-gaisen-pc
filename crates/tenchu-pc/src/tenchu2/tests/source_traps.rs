use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix},
    scene::Scene,
    tenchu2::{
        collision::Collision,
        items::{Context, Event, trap::Spear},
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_TRAP_ORACLE; source spear extension, retraction, terrain and contacts"]
fn native_spear_traps_match_original_motion_and_contacts() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut checked = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_TRAP_ORACLE").enumerate() {
        let map = maps
            .entry(row["map"].as_u64().unwrap() as usize)
            .or_insert_with_key(|&id| Level::load(&disc, id).unwrap().collision);
        let position = super::source_physics::vector(&row["origin"]);
        let direction = super::source_physics::vector(&row["direction"]);
        let spear = Spear::launch(position, direction, &math);
        assert_eq!(json!(spear), row["launch"], "launch {case}");
        let mut scene = Scene::default();
        let other = scene.add(
            None,
            Matrix {
                translation: position,
                ..Default::default()
            },
        );
        let model = scene.add(None, spear.transform);
        let mut collision = Collision::default();
        let slot = collision.insert(other).unwrap();
        slot.flags = row["other_flags"].as_u64().unwrap() as u16;
        slot.size = [500; 3].into();
        spear.register(model, &mut collision).unwrap();
        let slot = collision.get_mut(model).unwrap();
        assert_eq!(json!(slot.flags), row["initial_flags"], "flags {case}");
        assert_eq!(
            json!(slot.size.to_array()),
            row["initial_size"],
            "size {case}"
        );
        assert_eq!(slot.owner, None);
        collision.compute(|id| scene.world(id)).unwrap();
        let mut spear: Spear = serde_json::from_value(row["before"].clone()).unwrap();
        let mut cues = Vec::new();
        let alive = spear
            .advance(
                model,
                &mut Context {
                    map,
                    cursor: &mut Cursor::default(),
                    math: &math,
                    scene: &mut scene,
                    collision: &mut collision,
                    player: 0,
                },
                |event| {
                    cues.push(match event {
                        Event::Sound { position, id } => json!(["sound", position.to_array(), id]),
                        Event::Splash(position) => json!(["splash", position.to_array()]),
                        other => panic!("Unexpected trap event {other:?}"),
                    });
                    Ok(())
                },
            )
            .unwrap();
        if !alive {
            collision.remove(model);
        }
        assert_eq!(json!(alive), row["alive"], "expiry {case}");
        assert_eq!(json!(spear), row["after"], "state {case}");
        assert_eq!(
            json!(collision.index(model).is_some()),
            row["collider"],
            "collider {case}"
        );
        assert_eq!(json!(cues), row["cues"], "effects {case}");
        checked += 1;
    }
    eprintln!("{checked} original JP spear-trap cases");
}
