use serde_json::{Value, json};
use std::io::BufRead;
use tenchu_assets::{
    tenchu2::{
        characters,
        collision::{Cell, Collision as Map, Rules},
        commands::Commands,
        math_tables, motions,
        registry::Registry,
        weapons::Weapons,
    },
    terrain::Node,
};
use tenchu_core::{
    math::{Math, Vector3},
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State},
        collision::Collision,
        physics::{ContactActor, Terrain, Water},
        terrain::{Cursor, Options, Probe},
    },
};

fn vector<T: serde::de::DeserializeOwned>(value: &Value) -> Vector3<T> {
    Vector3::from(serde_json::from_value::<[T; 3]>(value.clone()).unwrap())
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_BODY_ORACLE from original body response code"]
fn native_body_response_preserves_knockback_and_contact_order() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let idle = registry.clip(0, 0).unwrap();
    let rig = characters::rig(&disc, &registry.actor(0).unwrap().model).unwrap();
    let mut resources = Resources {
        clips: motions::collect(&disc, [idle, 51, 52].into()).unwrap(),
        registry,
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let rules = Rules::load(&disc).unwrap();
    let mut map = Map {
        origin: [-10000; 2],
        dimensions: [1; 2],
        cell_size: 20000,
        cells: vec![Cell {
            minimum: 0,
            maximum: 0,
            starts: [0; 4],
            nodes: vec![Node {
                y: 0,
                dy: 0,
                bounds: [-10000, -10000, 10000, 10000],
                attributes: 0,
                division: u16::MAX,
            }],
        }],
    };
    let input = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_BODY_ORACLE").unwrap()).unwrap(),
    );
    let mut checked = 0;
    for (index, line) in input.lines().enumerate() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let int = |name: &str| row[name].as_i64().unwrap() as i32;
        let clip = resources.clips.get_mut(&idle).unwrap();
        clip.forward_speed = int("forward") as i16;
        clip.side_speed = int("side") as i16;
        let mut actor = Actor::new(0, &rig.parts, &resources).unwrap();
        actor.position = vector(&row["position"]);
        actor.previous = actor.position;
        actor.velocity = vector(&row["velocity"]);
        actor.width = int("width") as i16;
        actor.height = int("height") as i16;
        actor.turn = int("turn") as i16;
        actor.health = int("health") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.flags = int("flags") as u32;
        actor.motion.action = int("action") as i16;
        actor.motion.frame = int("frame") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        let mut collision = serde_json::to_value(Collision::default()).unwrap();
        let slots = row["slots"].as_array().unwrap();
        collision["len"] = json!(slots.len());
        for (i, slot) in slots.iter().enumerate() {
            collision["slots"][i] = json!({
                "model": i, "owner": slot["owner"],
                "position": vector::<i32>(&slot["position"]),
                "offset": Vector3::<i16>::default(), "size": vector::<i16>(&slot["size"]),
                "flags": slot["flags"], "contacts": if i == 0 { (1u64 << slots.len()) - 2 } else { 0 },
            });
        }
        let collision: Collision = serde_json::from_value(collision).unwrap();
        actor
            .move_on_terrain(
                &mut Terrain {
                    map: &mut map,
                    rules: &rules,
                    cursor: &mut Cursor::default(),
                    scratch: &mut Probe::default(),
                    water: &mut Water::default(),
                    options: Options::default(),
                },
                &resources,
                false,
                |actor| Ok(actor.position),
                |_, _| panic!("No surface effect on the flat contact-test terrain"),
            )
            .unwrap();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let voices = actor
            .resolve_contacts(&collision, 0, &resources, &mut random, |owner| {
                let other = row["others"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["id"].as_u64().unwrap() as usize == owner)
                    .unwrap();
                Ok(ContactActor {
                    position: vector(&other["position"]),
                    action: other["action"].as_i64().unwrap() as i16,
                    is_player: int("player") == owner as i32,
                })
            })
            .unwrap();
        let actual = json!({
            "position": actor.position.to_array(), "velocity": actor.velocity.to_array(),
            "yaw": actor.rotation.y, "flags": actor.flags, "health": actor.health,
            "hit": actor.hit.unwrap_or(255), "floor": actor.terrain.floor.unwrap_or(i32::MIN),
            "clearance": actor.terrain.height, "action": actor.motion.action,
            "frame": actor.motion.frame,
            "state": if actor.state == State::Hurt { 10 } else { int("state") },
            "seed": random.seed, "voices": voices,
        });
        assert_eq!(actual, row["expected"], "row {index}");
        assert_eq!(random.calls, voices.len() as u64);
        checked += 1;
    }
    assert_eq!(checked, 20000);
}
