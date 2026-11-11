use super::*;
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn snapshot_validation_keeps_retained_tail_contacts() {
    let mut collision = Collision::default();
    collision.insert(1).unwrap().owner = Some(0);
    collision.insert(2).unwrap().owner = Some(1);
    collision.slots[0].contacts.push(1);
    collision.len = 1;
    collision.validate_references(3, 2).unwrap();
    collision.slots[0].contacts[0] = collision.slots.len();
    assert!(collision.validate_references(3, 2).is_err());
    collision.slots[0].contacts[0] = 1;
    collision.len = collision.slots.len() + 1;
    assert!(collision.validate_references(3, 2).is_err());
    collision.len = 1;
    collision.slots[1].model = 3;
    assert!(collision.validate_references(3, 2).is_err());
    collision.slots[1].model = 2;
    collision.slots[1].owner = Some(2);
    assert!(collision.validate_references(3, 2).is_err());
}

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
fn retail_capacity_check_precedes_existing_slot_lookup() {
    let mut collision = Collision::default();
    for model in 0..64 {
        collision.insert(model).unwrap();
    }
    for model in [0, 64] {
        assert!(collision.insert(model).is_err());
    }
    collision.remove(63);
    assert!(collision.insert(64).is_err());
    collision.compute(|_| Ok(Matrix::default())).unwrap();
    assert!(collision.insert(0).is_ok());
    assert!(collision.insert(64).is_ok());
    collision.allow_expansion();
    assert!(collision.insert(0).is_ok());
    assert!(collision.insert(65).is_ok());
}

#[test]
fn expanded_contacts_preserve_order_and_frozen_tail_slots() {
    let mut collision = Collision::default();
    collision.allow_expansion();
    for model in 0..130 {
        let slot = collision.insert(model).unwrap();
        slot.owner = Some(model / 2);
        slot.size = Vector3::new(10, 10, 10);
    }
    collision.compute(|_| Ok(Matrix::default())).unwrap();
    for model in 0..130 {
        let expected: Vec<_> = (0..130).filter(|id| id / 2 != model / 2).collect();
        assert_eq!(
            collision
                .contacts(model)
                .map(|c| c.index)
                .collect::<Vec<_>>(),
            expected
        );
    }
    let frozen: Vec<_> = collision.contacts(0).collect();
    collision.get_mut(0).unwrap().flags |= DISABLED;
    collision.remove(64);
    collision.remove(129);
    collision.compute(|_| Ok(Matrix::default())).unwrap();
    assert_eq!(collision.slots().len(), 128);
    assert_eq!(collision.index(128), Some(64));
    assert_eq!(collision.contacts(0).collect::<Vec<_>>(), frozen);
    assert_eq!(collision.slot(129).unwrap().model, 129);
    let mut restored: Collision =
        serde_json::from_slice(&serde_json::to_vec(&collision).unwrap()).unwrap();
    for value in [&mut collision, &mut restored] {
        value.get_mut(0).unwrap().flags &= !DISABLED;
        value.compute(|_| Ok(Matrix::default())).unwrap();
        assert!(value.contacts(0).all(|c| c.index < 128));
        assert_eq!(value.contacts(0).count(), 126);
    }
    assert_eq!(
        serde_json::to_vec(&collision).unwrap(),
        serde_json::to_vec(&restored).unwrap()
    );
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
                            slot.contacts
                                .iter()
                                .fold(0u64, |mask, &id| mask | (1 << id)),
                            expected["contacts"].as_u64().unwrap(),
                            "{case}/{step}"
                        );
                        assert_eq!(
                            slot.contacts.len() as u64,
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
