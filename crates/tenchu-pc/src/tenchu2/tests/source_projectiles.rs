use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix, Position, Vector3},
    scene::Scene,
    tenchu2::{
        collision::Collision,
        items::projectile::{Context, Event, Projectile},
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_PROJECTILE_ORACLE; compares original shuriken/kunai updates and contacts"]
fn native_thrown_blades_match_original_motion_and_contacts() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_PROJECTILE_ORACLE").enumerate()
    {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let mut projectile: Projectile = serde_json::from_value(row["before"].clone()).unwrap();
        let origin: [i32; 3] = serde_json::from_value(row["origin"].clone()).unwrap();
        let direction: [i32; 3] = serde_json::from_value(row["direction"].clone()).unwrap();
        let launched =
            Projectile::launch(projectile.kind, 0, origin.into(), direction.into(), &math);
        assert_eq!(
            serde_json::to_value(launched).unwrap(),
            row["launch"],
            "launch {case}"
        );
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let other_index = row["other_index"].as_u64().unwrap() as usize;
        let flags = row["other_flags"].as_u64().unwrap() as u16;
        let other: [i32; 3] = serde_json::from_value(row["other_position"].clone()).unwrap();
        let mut model = 0;
        for i in 0..3 {
            let position = if i == 1 {
                projectile.flight.position
            } else if i == other_index && flags != 0 {
                other.into()
            } else {
                Position::new(1_000_000, 1_000_000, 1_000_000)
            };
            let root = scene.add(
                None,
                Matrix {
                    translation: position,
                    ..Default::default()
                },
            );
            let slot = collision.insert(root).unwrap();
            if i == 1 {
                model = root;
                projectile.register(root, &mut collision).unwrap();
            } else {
                slot.flags = if i == other_index { flags } else { 0 };
                slot.size = Vector3::new(4000, 4000, 4000);
                slot.owner = Some(1);
            }
        }
        collision.compute(|model| scene.world(model)).unwrap();
        let mut cues = Vec::new();
        let alive = projectile
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
                        Event::Splash(position) => json!(["splash", position.to_array()]),
                        Event::Sound { position, id } => json!(["sound", position.to_array(), id]),
                        Event::Impact {
                            position,
                            direction,
                        } => json!(["impact", position.to_array(), direction.to_array()]),
                        Event::Alert(position) => json!(["alert", position.to_array()]),
                    });
                    Ok(())
                },
            )
            .unwrap_or_else(|e| panic!("case {case}: {e}"));
        assert_eq!(json!(alive), row["alive"], "alive {case}");
        assert_eq!(json!(cues), row["cues"], "events {case}");
        let after = serde_json::to_value(&projectile).unwrap();
        for key in ["flight", "rotation", "phase"] {
            assert_eq!(after[key], row["after"][key], "{key} {case}");
        }
        if alive {
            assert_eq!(after, row["after"], "state {case}");
            let slot = collision.get_mut(model).unwrap();
            assert_eq!(json!(slot.flags), row["flags"], "flags {case}");
            assert_eq!(json!(slot.owner), row["owner"], "owner {case}");
        }
        count += 1;
    }
    assert!(count > 1000);
    eprintln!("{count} original blade constructors and updates");
}
