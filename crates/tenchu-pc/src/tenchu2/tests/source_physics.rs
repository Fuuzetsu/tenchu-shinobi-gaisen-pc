use serde_json::{Value, json};
use std::io::BufRead;
use tenchu_assets::tenchu2::{
    characters, collision::Rules, commands::Commands, level::Level, math_tables, motions,
    registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::{Math, Vector3},
    tenchu2::{
        actor::{Actor, Resources, State},
        physics::{Depth, Terrain, Water},
        terrain::{Cursor, Options, Probe, Surface},
    },
};

pub(super) fn vector<T: serde::de::DeserializeOwned>(value: &Value) -> Vector3<T> {
    Vector3::from(serde_json::from_value::<[T; 3]>(value.clone()).unwrap())
}

pub(super) fn probe(value: &Value) -> Probe {
    let [floor, height, material, blocked, lower, higher]: [i32; 6] =
        serde_json::from_value(value.clone()).unwrap();
    Probe {
        floor: (floor != i32::MIN).then_some(floor),
        height,
        material: material as u16,
        blocked: blocked as u16,
        lower: lower as u16,
        higher: higher as u16,
    }
}

pub(super) fn probe_values(probe: &Probe) -> [i32; 6] {
    [
        probe.floor.unwrap_or(i32::MIN),
        probe.height,
        i32::from(probe.material),
        i32::from(probe.blocked),
        i32::from(probe.lower),
        i32::from(probe.higher),
    ]
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_PHYSICS_ORACLE from original movement/collision code"]
fn native_terrain_movement_matches_japanese_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let idle = registry.clip(0, 0).unwrap();
    let rig = characters::rig(&disc, &registry.actor(0).unwrap().model).unwrap();
    let mut resources = Resources {
        registry,
        clips: motions::collect(&disc, [idle].into()).unwrap(),
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let rules = Rules::load(&disc).unwrap();
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut checked = 0;
    let input = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_PHYSICS_ORACLE").unwrap()).unwrap(),
    );
    for (index, line) in input.lines().enumerate() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let int = |name: &str| row[name].as_i64().unwrap() as i32;
        let bool = |name: &str| row[name].as_bool().unwrap();
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let clip = resources.clips.get_mut(&idle).unwrap();
        clip.forward_speed = int("forward") as i16;
        clip.side_speed = int("side") as i16;
        let mut actor = Actor::new(0, &rig.parts, &resources).unwrap();
        actor.position = vector(&row["position"]);
        actor.previous = vector(&row["previous"]);
        actor.velocity = vector(&row["velocity"]);
        actor.rotation.y = int("yaw") as i16;
        actor.width = int("width") as i16;
        actor.height = int("height") as i16;
        actor.turn = int("turn") as i16;
        actor.flags = row["flags"].as_u64().unwrap() as u32;
        actor.health = int("health") as i16;
        actor.motion.action = int("action") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.terrain = probe(&row["probe"]);
        let mut scratch = probe(&row["scratch"]);
        let mut cursor = Cursor::default();
        let mut water = Water::default();
        let root_offset = vector(&row["root_offset"]);
        let mut roots = 0;
        let mut effects = 0;
        let death = actor
            .move_on_terrain(
                &mut Terrain {
                    map: map.as_ref().unwrap(),
                    rules: &rules,
                    cursor: &mut cursor,
                    scratch: &mut scratch,
                    water: &mut water,
                    options: Options {
                        avoid_water: bool("avoid"),
                        ignore_steps: bool("ignore"),
                    },
                },
                &resources,
                bool("is_player"),
                |actor| {
                    roots += 1;
                    Ok(actor.position.wrapping_add(root_offset))
                },
                |_| {
                    effects += 1;
                    int("effect") as i16
                },
            )
            .unwrap();
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let actual = json!({
            "position": actor.position.to_array(), "previous": actor.previous.to_array(),
            "velocity": actor.velocity.to_array(), "yaw": actor.rotation.y,
            "flags": actor.flags, "health": actor.health, "last_vy": actor.last_vertical_velocity,
            "probe": probe_values(&actor.terrain), "scratch": probe_values(&scratch),
            "motion_mask": actor.motion.mask,
            "water": [match water.depth {
                None => -1, Some(Depth::Shallow) => 0, Some(Depth::Wading) => 1,
                Some(Depth::Swimming) => 2, Some(Depth::Submerged) => 3,
            }, water.surface, water.bottom],
            "current": surface(cursor.current), "last": surface(cursor.last),
            "material": cursor.material, "index": cursor.index,
            "roots": roots, "effects": effects, "death": death.is_some(),
        });
        let mut expected = row["expected"].clone();
        expected.as_object_mut().unwrap().remove("sounds");
        assert_eq!(actual, expected, "row {index}, map {map_id}");
        checked += 1;
    }
    assert_eq!(checked, 17250);
}
