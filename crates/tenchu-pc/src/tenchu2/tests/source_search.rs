use super::{
    source_combat::visibility,
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{combat::Rules, level::Level};
use tenchu_core::{
    random::Random,
    tenchu2::{
        ai::{
            Probe, SteeringInput,
            combat::Context,
            search::{Pursuit, PursuitPhase, Waiting},
            sight::Sight,
        },
        terrain::{Cursor, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SEARCH_ORACLE from original search decisions"]
fn native_lost_target_search_matches_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = resources();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let target = template.clone();
    let mut count = 0;
    let mut finished = 0;
    let mut map = None;
    let mut map_id = usize::MAX;
    let mut seen = std::collections::BTreeSet::new();
    for (id, row) in cases::<Value>("TENCHU2_SEARCH_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        let next_map = int("map") as usize;
        if next_map != map_id {
            map = Some(Level::load(&disc, next_map).unwrap().collision);
            map_id = next_map;
            seen.insert(map_id);
        }
        let mut cursor = Cursor::default();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.flags = int("flags") as u32;
        actor.turn = int("turn") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.position = vector(&row["position"]);
        actor.motion.action = 0;
        actor.motion.loops = int("loops") as i16;
        actor.pad.held = int("held") as u16;
        let mut sight = Sight {
            distance: int("distance"),
            angle: int("angle") as i16,
            destination: vector(&row["destination"]),
            ..Default::default()
        };
        let mut visible = visibility(int("visibility"));
        let mut random = Random::new(int("seed") as u32);
        let mut context = Context {
            actor: &mut actor,
            target: &target,
            sight: &mut sight,
            visibility: &mut visible,
            steering: SteeringInput {
                yaw: int("yaw") as i16,
                turn: int("turn") as i16,
                target_angle: int("angle") as i16,
                alerted: int("cached") & 3 != 0,
                blocked: int("cached") & 0x48000 != 0,
                forced: int("forced") != 0,
                ahead: (int("height") != i32::MIN).then_some(Probe {
                    height: int("height"),
                    material: int("material") as u16,
                }),
                left: boolean("left"),
                right: boolean("right"),
            },
            observed_flags: int("cached") as u32,
            player_held: 0,
            facing: int("facing"),
            difficulty: 2,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let mut alert = int("alert") as i16;
        let mut actions = Vec::new();
        let mut apply = |_: &mut tenchu_core::tenchu2::actor::Actor, action| {
            actions.push(action);
            Ok(())
        };
        let waiting = Waiting {
            turning: int("timer") as u8,
            aligning: int("turning") as u8,
        };
        let mode = row["mode"].as_str().unwrap();
        let (result, phase, memory) = if mode == "wait" {
            let mut waiting = waiting;
            let result = waiting
                .update(&mut context, &mut alert, boolean("align"), &mut apply)
                .unwrap();
            (result, json!(waiting), Value::Null)
        } else {
            let mut pursuit = Pursuit {
                phase: match int("phase") {
                    0 | 1 => PursuitPhase::Following {
                        detour: int("phase") != 0,
                        elapsed: int("elapsed") as i16,
                    },
                    3 => PursuitPhase::Looking {
                        elapsed: int("timer") as u8,
                        turning: int("turning") as u8,
                    },
                    4 => PursuitPhase::Waiting(waiting),
                    _ => panic!("Invalid search phase"),
                },
                remembered: vector(&row["memory"]),
            };
            let result = pursuit
                .update(
                    &mut context,
                    &mut alert,
                    map.as_ref().unwrap(),
                    &mut cursor,
                    &mut apply,
                )
                .unwrap();
            let phase = match &pursuit.phase {
                PursuitPhase::Following { detour, elapsed } => {
                    json!({"phase": "following", "detour": detour, "elapsed": elapsed})
                }
                PursuitPhase::Looking { elapsed, turning } => {
                    json!({"phase": "looking", "elapsed": elapsed, "turning": turning})
                }
                PursuitPhase::Waiting(waiting) => {
                    json!({"phase": "waiting", "turning": waiting.turning, "aligning": waiting.aligning})
                }
            };
            (result, phase, json!(pursuit.remembered.to_array()))
        };
        let surface = |id: Option<Surface>| id.map(|s| [s.cell, s.node]);
        let actual = json!({
            "held": result.decision.held, "finished": result.finished,
            "flags": context.observed_flags, "facing": context.facing,
            "angle": context.sight.angle, "yaw": context.actor.rotation.y,
            "destination": context.sight.destination.to_array(),
            "alert": alert, "phase": phase, "memory": memory,
            "forced": result.decision.forced.map_or(int("forced") as u32, |f| u32::from(f.held) << 16 | u32::from(f.remaining as u16)),
            "seed": random.seed, "random": random.calls, "actions": actions,
            "current": surface(cursor.current), "last": surface(cursor.last),
            "material": cursor.material, "index": cursor.index,
        });
        for (key, value) in row["expected"].as_object().unwrap() {
            assert_eq!(
                &actual[key], value,
                "row {id}, map {map_id}, mode {mode}: {key}"
            );
        }
        count += 1;
        finished += usize::from(result.finished);
    }
    assert_eq!(seen.len(), Level::available(&disc).unwrap().len());
    assert!(count >= 40000 && finished > 1000);
    eprintln!("{count} search decisions, {finished} completed searches");
}
