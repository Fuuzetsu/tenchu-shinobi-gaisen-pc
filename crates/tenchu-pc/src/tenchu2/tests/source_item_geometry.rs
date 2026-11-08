use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::Math,
    tenchu2::{
        items::{
            aim::Aim,
            flight::{Contact, Flight, Motion},
        },
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ITEM_GEOMETRY_ORACLE from JP aiming/item physics"]
fn native_item_aim_and_flight_match_source_terrain_contacts() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut counts = [0; 2];
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_ITEM_GEOMETRY_ORACLE").enumerate()
    {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let mut cursor = Cursor::default();
        let after = if row["kind"] == "aim" {
            let mut aim: Aim = serde_json::from_value(row["before"].clone()).unwrap();
            let reticle = aim
                .update(
                    serde_json::from_value(row["input"].clone()).unwrap(),
                    map,
                    &mut cursor,
                    &math,
                )
                .unwrap();
            assert_eq!(
                reticle
                    .map(|r| json!({"scale":r.scale,"ready":r.ready}))
                    .unwrap_or(Value::Null),
                row["reticle"],
                "reticle case {case}"
            );
            counts[0] += 1;
            serde_json::to_value(aim).unwrap()
        } else {
            let mut flight: Flight = serde_json::from_value(row["before"].clone()).unwrap();
            let motion = if row["directed"].as_bool().unwrap() {
                Motion::Directed
            } else {
                Motion::Ballistic
            };
            let (contact, splash) = flight
                .advance(motion, map, &mut cursor, &math)
                .unwrap_or_else(|error| panic!("case {case}: {error}"));
            assert_eq!(
                match contact {
                    Contact::Air => 0,
                    Contact::Ground => 1,
                    Contact::Outside => -1,
                },
                row["contact"].as_i64().unwrap(),
                "contact case {case}"
            );
            assert_eq!(
                json!(splash.map(|p| p.to_array())),
                row["splash"],
                "splash case {case}"
            );
            counts[1] += 1;
            serde_json::to_value(flight).unwrap()
        };
        assert_eq!(after, row["after"], "case {case} map {id}");
        assert_eq!(
            json!({"current":cursor.current.map(|s| [s.cell,s.node]),"last":cursor.last.map(|s| [s.cell,s.node]),"material":cursor.material,"index":cursor.index}),
            row["cursor"],
            "cursor case {case}"
        );
    }
    eprintln!(
        "JP item geometry: {} aims and {} flights",
        counts[0], counts[1]
    );
    assert!(counts[0] >= 900 && counts[1] >= 1800);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_GRAPPLE_ORACLE from JP hook launch/pull routines"]
fn native_grapple_trajectories_and_release_match_source() {
    use tenchu_core::{
        math::Position,
        tenchu2::items::grapple::{Grapple, State},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let state = |grapple: &Option<Grapple>| {
        grapple.as_ref().map_or(Value::Null, |grapple| {
            let mut value = serde_json::to_value(grapple).unwrap();
            value.as_object_mut().unwrap().remove("rope");
            value["rope_count"] = json!(grapple.rope.len());
            value
        })
    };
    let mut count = 0;
    let mut updates = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_GRAPPLE_ORACLE").enumerate() {
        let vector = |name: &str| {
            Position::from(serde_json::from_value::<[i32; 3]>(row[name].clone()).unwrap())
        };
        let reach = row["reach"].as_i64().unwrap() as i32;
        let mut aim = Aim {
            target: vector("target"),
            direction: vector("direction"),
            distance: row["distance"].as_i64().unwrap() as i32,
            reach: (reach >= 0).then_some(reach),
            phase: row["phase"].as_i64().unwrap() as i32,
        };
        let mut grapple = Grapple::launch(0, vector("origin"), aim.direction, &mut aim, &math);
        assert_eq!(state(&grapple), row["launch"], "launch {case}");
        assert_eq!(
            i64::from(aim.phase),
            row["phase_after"].as_i64().unwrap(),
            "phase {case}"
        );
        for (tick, event) in row["events"].as_array().unwrap().iter().enumerate() {
            if tick == 5 {
                grapple = postcard::from_bytes(&postcard::to_allocvec(&grapple).unwrap()).unwrap();
            }
            let hook = grapple.as_mut().unwrap();
            if event["release"].as_bool().unwrap() {
                hook.release();
            }
            let impacts = if hook.state == State::Released {
                grapple = None;
                Vec::new()
            } else {
                let hand = Position::from(
                    serde_json::from_value::<[i32; 3]>(event["hand"].clone()).unwrap(),
                );
                hook.advance(aim.target, hand, &math)
                    .map(|direction| [hook.transform.translation.to_array(), direction.to_array()])
                    .into_iter()
                    .collect()
            };
            assert_eq!(json!(impacts), event["impacts"], "impact {case}/{tick}");
            assert_eq!(state(&grapple), event["state"], "update {case}/{tick}");
            updates += 1;
        }
        count += 1;
    }
    eprintln!("JP grapples: {count} trajectories, {updates} updates");
    assert!(count >= 400 && updates >= 5000);
}
