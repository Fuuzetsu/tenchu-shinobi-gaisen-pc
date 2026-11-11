use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    scene::Scene,
    tenchu2::{
        collision::Collision,
        horizontal_distance,
        items::{
            Context, Event,
            arrow::{Arrow, FireAim},
            gun::Gunshot,
        },
        terrain::{self, Cursor, Query},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_RANGED_ORACLE; compares original arrows, fire arrows and gunshots"]
fn native_ranged_weapons_match_original_flight_embedding_and_damage_windows() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut count = [0; 4];
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_RANGED_ORACLE").enumerate() {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let kind = row["kind"].as_str().unwrap();
        let position = |key: &str| -> Position {
            let p: [i32; 3] = serde_json::from_value(row[key].clone()).unwrap();
            p.into()
        };
        let origin = position("origin");
        let direction = position("direction");
        let distance = row["distance"].as_i64().unwrap() as i32;
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut cursor = Cursor::default();
        let mut launched = if kind == "Gun" {
            let target = (!row["target"].is_null())
                .then(|| horizontal_distance(origin, position("target"), &math));
            json!(Gunshot::launch(0, origin, direction, target, map, &mut cursor, &math).unwrap())
        } else if kind.starts_with("Fire") {
            let aim = if terrain::level(map, &mut cursor, origin, Query::Surface)
                .unwrap()
                .is_none()
            {
                FireAim::Blocked
            } else if kind == "FirePlayer" {
                FireAim::Player
            } else {
                FireAim::Guard
            };
            json!(Arrow::fire(
                0,
                origin,
                direction,
                distance,
                aim,
                &math,
                &mut random
            ))
        } else {
            json!(Arrow::launch(
                origin,
                direction,
                distance,
                &math,
                &mut random
            ))
        };
        launched.as_object_mut().unwrap().remove("launched_by");
        assert_eq!(launched, row["launch"], "launch {case} {kind}");
        assert_eq!(
            json!(random.seed),
            row["launch_seed"],
            "launch RNG {case} {kind}"
        );
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let other_index = row["other_index"].as_u64().unwrap() as usize;
        let flags = row["other_flags"].as_u64().unwrap() as u16;
        let mut model = 0;
        for i in 0..3 {
            let translation = if i == 1 {
                origin
            } else if i == other_index && flags != 0 {
                position("other_position")
            } else {
                Position::new(1_000_000, 1_000_000, 1_000_000)
            };
            let rotation = if i == other_index {
                serde_json::from_value(row["other_rotation"].clone()).unwrap()
            } else {
                Matrix::default().rotation
            };
            let root = scene.add(
                None,
                Matrix {
                    translation,
                    rotation,
                },
            );
            let slot = collision.insert(root).unwrap();
            if i == 1 {
                model = root;
            } else {
                slot.flags = if i == other_index { flags } else { 0 };
                slot.size = Vector3::new(4000, 4000, 4000);
                slot.owner = Some(1);
            }
        }
        let mut arrow = (kind != "Gun")
            .then(|| serde_json::from_value::<Arrow>(row["before"].clone()).unwrap());
        let mut gun = (kind == "Gun")
            .then(|| serde_json::from_value::<Gunshot>(row["before"].clone()).unwrap());
        if let Some(arrow) = &arrow {
            arrow.register(model, &mut collision).unwrap();
        }
        if let Some(gun) = &gun {
            gun.register(model, &mut collision).unwrap();
        }
        collision.compute(|model| scene.world(model)).unwrap();
        if arrow.as_ref().is_some_and(|a| {
            matches!(
                a.phase,
                tenchu_core::tenchu2::items::arrow::Phase::Embedded { .. }
            )
        }) {
            collision.get_mut(model).unwrap().flags &= 0xbfbf;
            collision.remove(model);
        }
        let mut cues = Vec::new();
        let mut emit = |event| {
            match event {
                Event::Splash(p) => cues.push(json!(["splash", p.to_array()])),
                Event::Impact {
                    position,
                    direction,
                } => cues.push(json!(["impact", position.to_array(), direction.to_array()])),
                Event::Sound { position, id } => {
                    cues.push(json!(["sound", position.to_array(), id]))
                }
                Event::Alert(p) => cues.push(json!(["alert", p.to_array(), 1])),
                Event::Explosion {
                    position,
                    contact,
                    kind,
                } => {
                    cues.push(json!(["explode", position.to_array(), kind]));
                    cues.push(json!([
                        "alert",
                        position.to_array(),
                        if contact { 2 } else { 3 }
                    ]));
                }
                Event::Smoke { position, kind } => {
                    cues.push(json!(["smoke", position.to_array(), kind]))
                }
                Event::Fuse(_) => panic!("Unexpected fuse"),
                Event::Ribbon { .. } => panic!("Unexpected ribbon"),
                Event::SleepGas { .. } => panic!("Unexpected sleeping gas"),
                Event::FireBlast { .. } => panic!("Unexpected fire breath"),
                Event::DispersingSmoke { .. } => panic!("Unexpected status cloud"),
            }
            Ok(())
        };
        let mut context = Context {
            map,
            cursor: &mut cursor,
            math: &math,
            scene: &mut scene,
            collision: &mut collision,
            player: 1,
        };
        let alive = if let Some(arrow) = &mut arrow {
            arrow.advance(
                model,
                row["tick"].as_u64().unwrap() as u32,
                &mut context,
                &mut emit,
            )
        } else {
            gun.as_mut()
                .unwrap()
                .advance(model, &mut context, &mut emit)
        }
        .unwrap_or_else(|e| panic!("case {case} {kind}: {e}"));
        if !alive {
            collision.remove(model);
        }
        assert_eq!(json!(alive), row["alive"], "alive {case} {kind}");
        assert_eq!(json!(cues), row["cues"], "events {case} {kind}");
        let mut after = if let Some(arrow) = &arrow {
            json!(arrow)
        } else {
            json!(gun.as_ref().unwrap())
        };
        after.as_object_mut().unwrap().remove("launched_by");
        assert_eq!(after, row["after"], "state {case} {kind}");
        let slot = collision.slot(1).unwrap();
        assert_eq!(json!(slot.flags), row["flags"], "flags {case} {kind}");
        assert_eq!(
            json!(slot.size.to_array()),
            row["size"],
            "size {case} {kind}"
        );
        count[match kind {
            "Arrow" => 0,
            "Fire" => 1,
            "FirePlayer" => 2,
            _ => 3,
        }] += 1;
    }
    assert!(count.iter().all(|&n| n > 400));
    eprintln!("Original ranged weapon cases: {count:?}");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_AIM_TARGET_ORACLE; compares fire-arrow target selection"]
fn native_fire_arrow_targets_match_original_cone_and_eligibility() {
    use tenchu_core::tenchu2::{actor::State, items::aim};
    let resources = super::source_control::resources();
    let template = super::source_control::template(&resources);
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_AIM_TARGET_ORACLE").enumerate()
    {
        let vector = |value: &Value| -> Position {
            let v: [i32; 3] = serde_json::from_value(value.clone()).unwrap();
            v.into()
        };
        let actors: Vec<_> = row["actors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| {
                let mut actor = template.clone();
                actor.flags = source["flags"].as_u64().unwrap() as u32;
                actor.state =
                    State::from_action((source["state"].as_i64().unwrap() as i16) << 8).unwrap();
                actor.position = vector(&source["position"]);
                actor
            })
            .collect();
        let target = aim::target(
            vector(&row["origin"]),
            vector(&row["direction"]),
            row["owner"].as_u64().unwrap() as usize,
            actors.iter().enumerate(),
            &resources.math,
        );
        assert_eq!(json!(target), row["target"], "target {case}");
        count += 1;
    }
    assert_eq!(count, 1000);
}
