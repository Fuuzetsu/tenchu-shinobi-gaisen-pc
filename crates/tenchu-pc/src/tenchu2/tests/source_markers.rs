use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix, Position},
    scene::Scene,
    tenchu2::{
        collision::Collision,
        items::{Context, marker::Marker},
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_MARKER_ORACLE; original coloured-rice terrain placement and surface changes"]
fn native_marker_placement_matches_original_surfaces() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    let mut placed = 0;
    let mut rejected = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_MARKER_ORACLE").enumerate() {
        let map = maps
            .entry(row["map"].as_u64().unwrap() as usize)
            .or_insert_with_key(|id| Level::load(&disc, *id).unwrap().collision);
        let origin: Position = serde_json::from_value::<[i32; 3]>(row["origin"].clone())
            .unwrap()
            .into();
        let mut scene = Scene::default();
        let model = scene.add(None, Matrix::default());
        let mut collision = Collision::default();
        let mut cursor = Cursor::default();
        let mut context = Context {
            map,
            cursor: &mut cursor,
            math: &math,
            scene: &mut scene,
            collision: &mut collision,
            player: 0,
        };
        let marker = Marker::place(
            0,
            row["color"].as_u64().unwrap() as u8,
            origin,
            row["yaw"].as_i64().unwrap() as i16,
            &mut context,
        )
        .unwrap();
        assert_eq!(json!(marker), row["initial"], "placement {case}");
        assert_eq!(
            json!(marker.floor.is_some()),
            row["valid"],
            "accepted {case}"
        );
        if marker.floor.is_some() {
            placed += 1;
            marker.register(model, context.collision).unwrap();
            let slot = context.collision.get_mut(model).unwrap();
            assert_eq!(json!(slot.flags), row["flags"], "flags {case}");
            assert_eq!(json!(slot.size.to_array()), row["size"], "bounds {case}");
            assert_eq!(slot.owner, None);
            let mut marker: Marker = serde_json::from_value(row["before"].clone()).unwrap();
            assert_eq!(
                json!(marker.advance(model, &mut context).unwrap()),
                row["alive"],
                "expiry {case}"
            );
            assert_eq!(json!(marker), row["after"], "update {case}");
            assert_eq!(
                context.scene.world(model).unwrap().translation,
                marker.position
            );
        } else {
            rejected += 1;
        }
        count += 1;
    }
    assert_eq!((count, placed, rejected), (1200, 773, 427));
}
