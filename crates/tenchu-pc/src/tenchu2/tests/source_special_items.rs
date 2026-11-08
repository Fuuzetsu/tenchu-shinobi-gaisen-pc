use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix},
    scene::Scene,
    tenchu2::{collision::Collision, items::powder::Powder, terrain::Cursor},
};

#[test]
#[ignore = "requires TENCHU2_CLOUD_ORACLE; JP executable cloud collision growth, emission and expiry"]
fn native_status_cloud_matches_original_timing_and_collisions() {
    use tenchu_core::tenchu2::items::{Event, Items, State, cloud::Cloud};
    let mut checked = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_CLOUD_ORACLE").enumerate() {
        let position = super::source_physics::vector(&row["position"]);
        let direction = super::source_physics::vector(&row["direction"]);
        let owner = (!row["trap"].as_bool().unwrap()).then_some(3);
        let mut items = Items::default();
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let model = items
            .insert(
                State::Cloud(Cloud {
                    owner,
                    position,
                    direction,
                    age: 0,
                }),
                &mut scene,
                &mut collision,
            )
            .unwrap();
        assert_eq!(scene.world(model).unwrap().translation, position);
        let slot = collision.get_mut(model).unwrap();
        assert_eq!(json!(slot.flags), row["initial"]["flags"], "flags {case}");
        assert_eq!(
            json!(slot.size.to_array()),
            row["initial"]["size"],
            "size {case}"
        );
        assert_eq!(slot.owner, owner);
        for (tick, expected) in row["states"].as_array().unwrap().iter().enumerate() {
            if let Some(flags) = expected["flags"].as_u64() {
                collision.get_mut(model).unwrap().flags = flags as u16;
            }
            let State::Cloud(cloud) = &mut items.entries[0].state else {
                panic!("cloud lost")
            };
            let mut cues = Vec::new();
            let alive = cloud
                .advance(model, &mut collision, |cue| {
                    if let Event::DispersingSmoke {
                        position,
                        direction,
                        kind,
                    } = cue
                    {
                        cues.push(json!([position.to_array(), direction.to_array(), kind]));
                        Ok(())
                    } else {
                        panic!("Unexpected cloud event {cue:?}")
                    }
                })
                .unwrap();
            assert_eq!(json!(alive), expected["alive"], "expiry {case}/{tick}");
            assert_eq!(json!(cloud.age), expected["age"], "age {case}/{tick}");
            assert_eq!(json!(cues), expected["cues"], "emission {case}/{tick}");
            assert_eq!(
                json!(collision.get_mut(model).unwrap().size.to_array()),
                expected["size"],
                "bounds {case}/{tick}"
            );
            if !alive {
                items.remove(0, &mut collision);
                assert!(collision.index(model).is_none());
            }
            if tick == 32 {
                items = serde_json::from_slice(&serde_json::to_vec(&items).unwrap()).unwrap();
                collision =
                    serde_json::from_slice(&serde_json::to_vec(&collision).unwrap()).unwrap();
            }
            checked += 1;
        }
    }
    eprintln!("{checked} original JP cloud updates");
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_FIRE_BREATH_ORACLE and TENCHU2_FIRE_BLAST_ORACLE; original fire-breath placement, bursts and hit-volume lifetime"]
fn native_fire_breath_matches_original_bursts_and_damage_volumes() {
    use tenchu_core::tenchu2::items::{
        Event,
        fire::{Blast, Breath},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_FIRE_BREATH_ORACLE").enumerate()
    {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let origin = serde_json::from_value::<[i32; 3]>(row["origin"].clone()).unwrap();
        let launch = Breath::begin(
            0,
            origin.into(),
            row["yaw"].as_i64().unwrap() as i16,
            map,
            &mut Cursor::default(),
            &math,
        )
        .unwrap();
        assert_eq!(json!(launch), row["launch"], "launch {case}");
        let mut breath: Breath = serde_json::from_value(row["before"].clone()).unwrap();
        let mut cues = Vec::new();
        let alive = breath
            .advance(row["after_yaw"].as_i64().unwrap() as i16, &math, |cue| {
                cues.push(match cue {
                    Event::FireBlast { owner, position } => {
                        assert_eq!(owner, 0);
                        json!(["blast", position.to_array()])
                    }
                    Event::Sound { position, id } => json!(["sound", position.to_array(), id]),
                    _ => panic!("unexpected fire cue"),
                });
                Ok(())
            })
            .unwrap();
        assert_eq!(json!(alive), row["alive"], "expiry {case}");
        assert_eq!(json!(breath), row["after"], "update {case}");
        assert_eq!(json!(cues), row["cues"], "bursts {case}");
        count += 1;
    }
    assert_eq!(count, 600);
    count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_FIRE_BLAST_ORACLE").enumerate()
    {
        let initial = &row["initial"];
        let position = serde_json::from_value::<[i32; 3]>(initial["position"].clone())
            .unwrap()
            .into();
        let mut blast = Blast {
            owner: 0,
            position,
            age: 0,
        };
        let mut collision = Collision::default();
        blast.register(0, &mut collision).unwrap();
        let slot = collision.slot(collision.index(0).unwrap()).unwrap();
        assert_eq!(
            json!(slot.size.to_array()),
            initial["size"],
            "initial size {case}"
        );
        assert_eq!(json!(slot.flags), initial["flags"], "flags {case}");
        assert_eq!(slot.owner, Some(0));
        blast.age = row["age"].as_i64().unwrap() as i32;
        assert_eq!(
            json!(blast.advance(0, &mut collision).unwrap()),
            row["alive"],
            "expiry {case}"
        );
        assert_eq!(json!(blast.age), row["after_age"], "age {case}");
        assert_eq!(
            json!(collision.get_mut(0).unwrap().size.to_array()),
            row["size"],
            "size {case}"
        );
        count += 1;
    }
    assert_eq!(count, 120);
}

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

#[test]
#[ignore = "requires TENCHU2_INVISIBILITY_ORACLE; original activation, cancellation and expiry"]
fn native_invisibility_matches_original_activation_and_cancellation_boundaries() {
    use tenchu_core::tenchu2::items::invisibility::Invisibility;
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_INVISIBILITY_ORACLE").enumerate()
    {
        let mut mode = row["initial"].as_u64().unwrap() as u16;
        let invisibility = Invisibility::begin(0, &mut mode);
        assert_eq!(json!(invisibility), row["launch"], "launch {case}");
        assert_eq!(json!(mode), row["beginmode"], "launch mode {case}");
        let mut invisibility: Invisibility = serde_json::from_value(row["before"].clone()).unwrap();
        mode = row["mode"].as_u64().unwrap() as u16;
        let mut translucent = row["translucent"].as_bool().unwrap();
        assert_eq!(
            json!(invisibility.advance(&mut mode, &mut translucent)),
            row["alive"],
            "expiry {case}"
        );
        assert_eq!(json!(mode), row["after_mode"], "mode {case}");
        assert_eq!(json!(translucent), row["after_translucent"], "blend {case}");
        assert_eq!(json!(invisibility), row["after"], "update {case}");
        count += 1;
    }
    assert!(count >= 1200);
}
