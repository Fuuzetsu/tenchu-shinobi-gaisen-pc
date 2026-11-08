use super::{
    source_control::{cases, template, velocity},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    Edition, commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{ItemInput, ItemSound, Request},
        inventory::{Item, Unavailable},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ITEM_ENTRY_ORACLE from original item/overlay routines"]
fn native_inventory_consumption_and_item_entry_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_ITEM_ENTRY_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted = std::collections::BTreeSet::new();
    for row in &rows {
        wanted.extend([
            row["clip"].as_i64().unwrap() as i16,
            row["expected"]["clip"].as_i64().unwrap() as i16,
        ]);
        for action in 0x800..=0x80e {
            wanted.extend(registry.clip(row["kind"].as_i64().unwrap() as i16, action));
        }
    }
    wanted.extend(registry.clip(0, 0));
    let resources = Resources {
        clips: motions::collect(&disc, wanted).unwrap(),
        registry,
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        }
        .into(),
    };
    let template = template(&resources);
    let mut kinds = std::collections::BTreeSet::new();
    for (id, row) in rows.iter().enumerate() {
        let int = |k: &str| row[k].as_i64().unwrap() as i32;
        let boolean = |k: &str| row[k].as_bool().unwrap();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action(int("old") as i16).unwrap();
        actor
            .motion
            .select(
                int("old") as i16,
                actor.kind,
                &resources.registry,
                &resources.clips,
            )
            .unwrap();
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.rotation.y = int("angle") as i16;
        actor.velocity = vector(&row["velocity"]);
        actor.pad.previous = int("previous") as u16;
        actor.inventory.counts = std::array::from_fn(|i| row["counts"][i].as_u64().unwrap() as u8);
        actor.inventory.selected = Item::try_from(int("selected") as u8).ok();
        actor.inventory.active = Item::try_from(int("active") as u8).unwrap();
        let mut request = Request {
            action: int("action") as i16,
            velocity: velocity(int("mode") as i16),
        };
        let input = ItemInput {
            held: int("input") as u16,
            player: boolean("player"),
            scripted: boolean("scripted"),
        };
        let mut sounds = Vec::new();
        let taken = if row["op"] == "consume" {
            let taken = actor.inventory.take();
            if taken == Err(Unavailable::Empty) && input.player {
                sounds.push("empty");
            }
            Some(taken.is_ok())
        } else {
            actor
                .item_input(&mut request, &input, &resources, |sound| {
                    sounds.push(match sound {
                        ItemSound::EmptyInventory => "empty",
                        ItemSound::Unavailable => "unavailable",
                    });
                    Ok(())
                })
                .unwrap();
            None
        };
        let state = match actor.state {
            State::Normal => 0,
            State::Ready => 1,
            State::Crouched => 2,
            State::Jumping => 6,
            State::Item => 8,
            _ => panic!("Unexpected item test state"),
        };
        let actual = json!({"action":request.action,"mode":match request.velocity { Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1 },
            "motion":actor.motion.action,"frame":actor.motion.frame,"loops":actor.motion.loops,"mask":actor.motion.mask,"state":state,
            "clip":actor.motion.clip,"velocity":actor.velocity.to_array(),"active":u8::from(actor.inventory.active),
            "counts":actor.inventory.counts,"taken":taken,"sounds":sounds});
        assert_eq!(
            actual,
            row["expected"],
            "case {id}, kind {}, item {}",
            actor.kind,
            int("selected")
        );
        kinds.insert(actor.kind);
    }
    assert_eq!(
        kinds.len(),
        if resources.registry.edition == Edition::Japan {
            127
        } else {
            130
        }
    );
    assert_eq!(rows.len(), kinds.len() * 200);
    eprintln!(
        "{} native item entry/consumption checks across {} definitions",
        rows.len(),
        kinds.len()
    );
}
