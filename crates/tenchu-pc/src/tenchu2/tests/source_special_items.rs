use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix},
    scene::Scene,
    tenchu2::{collision::Collision, items::powder::Powder, terrain::Cursor},
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_POWDER_ORACLE; original confusion-powder placement and expiry"]
fn native_confusion_powder_matches_original_terrain_placement_and_duration() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_POWDER_ORACLE").enumerate() {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let origin = serde_json::from_value::<[i32; 3]>(row["origin"].clone()).unwrap();
        let yaw = row["yaw"].as_i64().unwrap() as i16;
        let powder =
            Powder::blow(0, origin.into(), yaw, map, &mut Cursor::default(), &math).unwrap();
        assert_eq!(json!(powder), row["launch"], "launch {case}");
        let mut scene = Scene::default();
        let model = scene.add(
            None,
            Matrix {
                translation: powder.position,
                ..Default::default()
            },
        );
        let mut collision = Collision::default();
        powder.register(model, &mut collision).unwrap();
        let slot = collision.slot(collision.index(model).unwrap()).unwrap();
        assert_eq!(json!(slot.flags), row["flags"], "flags {case}");
        assert_eq!(json!(slot.size.to_array()), row["size"], "size {case}");
        let mut powder: Powder = serde_json::from_value(row["before"].clone()).unwrap();
        assert_eq!(
            powder.advance(),
            row["alive"].as_bool().unwrap(),
            "expiry {case}"
        );
        assert_eq!(json!(powder), row["after"], "update {case}");
        count += 1;
    }
    assert!(count >= 1000);
}
