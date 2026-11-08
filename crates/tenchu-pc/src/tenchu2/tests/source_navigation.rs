use super::{
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    random::Random,
    tenchu2::{
        actor::State,
        ai::{
            ForcedInput,
            navigation::{Ground, Navigation, Surroundings},
        },
        terrain::{Cursor, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_NAVIGATION_ORACLE from original AI terrain checks"]
fn native_guard_navigation_matches_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = resources();
    let template = template(&resources);
    let mut map = None;
    let mut map_id = usize::MAX;
    let mut seen = std::collections::BTreeSet::new();
    let mut counts = [0; 2];
    for (id, row) in cases::<Value>("TENCHU2_NAVIGATION_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        if int("map") as usize != map_id {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
            seen.insert(map_id);
        }
        let mut actor = template.clone();
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.flags = int("flags") as u32;
        actor.rotation.y = int("yaw") as i16;
        actor.position = vector(&row["position"]);
        actor.pad.held = int("old_held") as u16;
        actor.terrain.floor = (int("floor") != i32::MIN).then_some(int("floor"));
        let mut target = template.clone();
        target.position = vector(&row["target_position"]);
        let heights: [i32; 4] = serde_json::from_value(row["heights"].clone()).unwrap();
        let materials: [u16; 4] = serde_json::from_value(row["materials"].clone()).unwrap();
        let probe = |i: usize| Ground {
            height: (heights[i] != i32::MIN).then_some(heights[i]),
            material: materials[i],
        };
        let mut surroundings = Surroundings {
            ahead: probe(0),
            behind: probe(1),
            left: probe(2),
            right: probe(3),
        };
        let mut cursor = Cursor::default();
        let mut random = Random::new(int("seed") as u32);
        let mut forced = (int("forced") != 0).then_some(ForcedInput {
            held: (int("forced") as u32 >> 16) as u16,
            remaining: int("forced") as i16,
        });
        let mode = row["mode"].as_str().unwrap();
        let actual = match mode {
            "probe" => {
                surroundings
                    .sample(
                        &actor,
                        int("tick") as u32,
                        map.as_ref().unwrap(),
                        &mut cursor,
                        &resources.math,
                    )
                    .unwrap();
                let ground = [
                    surroundings.ahead,
                    surroundings.behind,
                    surroundings.left,
                    surroundings.right,
                ];
                let surface = |id: Option<Surface>| id.map(|s| [s.cell, s.node]);
                counts[0] += 1;
                json!({
                    "heights": ground.map(|p| p.height.unwrap_or(i32::MIN)),
                    "materials": ground.map(|p| p.material),
                    "current": surface(cursor.current), "last": surface(cursor.last),
                    "material": cursor.material, "index": cursor.index,
                })
            }
            "finish" => {
                let navigation = Navigation {
                    actor: &actor,
                    target: boolean("target").then_some(&target),
                    surroundings: &surroundings,
                    facing: int("facing"),
                    alert: int("alert") as i16,
                    player_alive: boolean("alive"),
                };
                let held = navigation.finish(int("held") as u16, &mut forced, &mut random);
                counts[1] += 1;
                json!({
                    "held": held, "forced": forced.map_or(0, |f| u32::from(f.held) << 16 | u32::from(f.remaining as u16)),
                    "seed": random.seed, "random": random.calls,
                })
            }
            _ => panic!("Unknown navigation operation"),
        };
        assert_eq!(actual, row["expected"], "row {id} map {map_id} {mode}");
    }
    assert_eq!(seen.len(), Level::available(&disc).unwrap().len());
    assert!(counts[0] >= 13000 && counts[1] >= 23000);
    eprintln!(
        "{} terrain samples and {} movement constraints",
        counts[0], counts[1]
    );
}
