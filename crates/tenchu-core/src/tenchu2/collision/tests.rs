use super::*;
use serde_json::Value;
use std::collections::BTreeMap;

fn vector<T: serde::de::DeserializeOwned>(value: &Value) -> Vector3<T> {
    Vector3::from(serde_json::from_value::<[T; 3]>(value.clone()).unwrap())
}

fn contact(value: &Value) -> Option<Contact> {
    (!value.is_null()).then(|| Contact {
        index: value["index"].as_u64().unwrap() as usize,
        distance: vector(&value["distance"]),
    })
}

#[test]
#[ignore = "requires TENCHU2_CONTACT_ORACLE from original EU or JP collision routines"]
fn original_object_contacts_compaction_and_cover_match() {
    let data =
        std::fs::read_to_string(std::env::var_os("TENCHU2_CONTACT_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    for (case, line) in data.lines().enumerate() {
        let events: Vec<Value> = serde_json::from_str(line).unwrap();
        let mut collision = Collision::default();
        let mut cover = Cover::default();
        let mut matrices = BTreeMap::new();
        for (step, event) in events.iter().enumerate() {
            let model = || event["model"].as_u64().unwrap() as usize;
            match event["op"].as_str().unwrap() {
                "add" => {
                    collision.insert(model()).unwrap();
                    assert_eq!(
                        collision.index(model()),
                        Some(event["index"].as_u64().unwrap() as usize),
                        "{case}/{step}"
                    );
                }
                "remove" => collision.remove(model()),
                "update" => {
                    for value in event["slots"].as_array().unwrap() {
                        let model = value["model"].as_u64().unwrap() as usize;
                        let slot = collision.get_mut(model).unwrap();
                        slot.offset = vector(&value["offset"]);
                        slot.size = vector(&value["size"]);
                        slot.owner = value["owner"].as_u64().map(|v| v as usize);
                        slot.flags = value["flags"].as_u64().unwrap() as u16;
                        matrices.insert(
                            model,
                            Matrix {
                                rotation: serde_json::from_value(
                                    value["matrix"]["rotation"].clone(),
                                )
                                .unwrap(),
                                translation: vector(&value["matrix"]["translation"]),
                            },
                        );
                    }
                }
                "compute" => {
                    collision.compute(|model| Ok(matrices[&model])).unwrap();
                    let expected = event["slots"].as_array().unwrap();
                    assert_eq!(collision.len, expected.len(), "{case}/{step}");
                    for (slot, expected) in collision.slots().iter().zip(expected) {
                        assert_eq!(slot.model, expected["model"].as_u64().unwrap() as usize);
                        assert_eq!(
                            slot.position,
                            vector(&expected["position"]),
                            "{case}/{step}"
                        );
                        assert_eq!(
                            slot.contacts,
                            expected["contacts"].as_u64().unwrap(),
                            "{case}/{step}"
                        );
                        assert_eq!(
                            u64::from(slot.contacts.count_ones()),
                            expected["count"].as_u64().unwrap()
                        );
                    }
                    checked += 1;
                }
                "contacts" => {
                    let expected: Vec<_> = event["contacts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| contact(v).unwrap())
                        .collect();
                    assert_eq!(
                        collision.contacts(model()).collect::<Vec<_>>(),
                        expected,
                        "{case}/{step}"
                    );
                    assert_eq!(
                        collision
                            .contact(model(), event["direct_index"].as_u64().unwrap() as usize),
                        contact(&event["direct"]),
                        "{case}/{step}"
                    );
                }
                "cover_init" => {
                    let excluded = event["excluded"].as_i64().unwrap();
                    cover = Cover::new((excluded >= 0).then_some(excluded as usize));
                }
                "cover" => {
                    let expected = event["result"].as_i64().unwrap();
                    assert_eq!(
                        cover.check(
                            &collision,
                            vector(&event["position"]),
                            event["tick"].as_i64().unwrap() as i32
                        ),
                        (expected >= 0).then_some(expected as usize),
                        "{case}/{step}"
                    );
                }
                _ => panic!("Unknown collision oracle event"),
            }
        }
    }
    assert_eq!(checked, 6000);
}
