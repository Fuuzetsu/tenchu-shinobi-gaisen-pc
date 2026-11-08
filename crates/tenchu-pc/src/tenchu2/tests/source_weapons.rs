use serde_json::{Value, json};
use tenchu_assets::{
    model::Part,
    tenchu2::{commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons},
};
use tenchu_core::{
    math::{Math, Matrix},
    tenchu2::{
        actor::{Actor, Resources},
        collision::Collision,
        rig::{Contacts, Grip, Rig, WeaponMode},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_WEAPON_ORACLE from original weapon control routines"]
fn native_weapon_contacts_and_attachments_match_every_actor_definition() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let resources = Resources {
        clips: motions::collect(&disc, [registry.clip(0, 0).unwrap()].into()).unwrap(),
        registry,
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let parts: Vec<_> = (0..64)
        .map(|i| Part {
            parent: if i == 0 { -1 } else { 0 },
            child: i,
            position: [0; 3],
            geometry: vec![],
        })
        .collect();
    let input =
        std::fs::read_to_string(std::env::var_os("TENCHU2_WEAPON_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    let mut seen = std::collections::BTreeSet::new();
    for line in input.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let mut actor = Actor::new(0, &parts, &resources).unwrap();
        actor.kind = row["kind"].as_i64().unwrap() as i16;
        actor.width = 450;
        actor.height = 1800;
        actor.health = 30;
        actor.attack.trails = 15;
        let mut rig = Rig {
            root: 64,
            joints: (0..64).collect(),
            weapons: std::array::from_fn(Some),
        };
        let mut collision = Collision::default();
        seen.insert(actor.kind);
        for (step, event) in row["events"].as_array().unwrap().iter().enumerate() {
            let previous_trails = actor.attack.trails;
            let mut voices = Vec::new();
            match event["op"].as_str().unwrap() {
                "weapon" => {
                    let mode = match event["mode"].as_u64().unwrap() {
                        0 => WeaponMode::Draw,
                        1 => WeaponMode::Sheathe,
                        2 => WeaponMode::Refresh,
                        3 => WeaponMode::Remove,
                        _ => panic!("Unknown weapon request"),
                    };
                    actor
                        .set_weapon(&mut rig, 0, &mut collision, &resources, mode)
                        .unwrap();
                }
                "contacts" => {
                    actor.attack.index = Some(event["attack"].as_u64().unwrap() as usize);
                    let mode = match event["mode"].as_u64().unwrap() {
                        0 => Contacts::Start,
                        1 => Contacts::Finish,
                        2 => Contacts::Stop,
                        _ => panic!("Unknown contact mode"),
                    };
                    actor
                        .set_attack_contacts(&rig, &mut collision, &resources, mode)
                        .unwrap();
                }
                "swap" => {
                    let mode = match event["mode"].as_u64().unwrap() {
                        0 => Grip::Swap,
                        1 => Grip::Release,
                        2 => Grip::Hold,
                        _ => panic!("Unknown grip mode"),
                    };
                    voices.extend(
                        actor
                            .swap_attachments(
                                &mut rig,
                                event["a"].as_u64().unwrap() as usize,
                                event["b"].as_u64().unwrap() as usize,
                                mode,
                            )
                            .unwrap(),
                    );
                }
                "body" => {
                    actor.health = event["health"].as_i64().unwrap() as i16;
                    actor
                        .set_body_collision(
                            rig.joints[0],
                            0,
                            &mut collision,
                            event["enabled"].as_u64().unwrap() != 0,
                        )
                        .unwrap();
                }
                "compute" => collision.compute(|_| Ok(Matrix::default())).unwrap(),
                _ => panic!("Unknown weapon oracle operation"),
            }
            let mut contacts = 0u32;
            let slots: Vec<_> = collision.slots().iter().map(|slot| {
                if slot.flags & 0xc400 == 0x4000 {
                    contacts |= 1u32.wrapping_shl(slot.model as u32);
                }
                json!({ "model": slot.model, "flags": slot.flags, "owner": slot.owner == Some(0), "offset": slot.offset.to_array(), "size": slot.size.to_array() })
            }).collect();
            let actual = json!({ "flags": actor.flags, "weapons": rig.weapons, "slots": slots, "contacts": contacts, "trails": actor.attack.trails, "voices": voices, "freed": previous_trails.count_ones() - actor.attack.trails.count_ones() });
            assert_eq!(
                actual, event["expected"],
                "kind {}, step {step}",
                actor.kind
            );
            checked += 1;
        }
    }
    assert_eq!(seen.len(), resources.registry.actors.len());
    assert!(checked >= 23530);
}
