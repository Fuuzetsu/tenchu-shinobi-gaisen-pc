use super::{
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    commands::Commands, level::Level, math_tables, motions, registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    random::Random,
    tenchu2::{
        actor::{Resources, State},
        ai::{CorpseContext, CorpseSearch, Sentry, Sleep, SleepInput},
        terrain::{Cursor, Surface},
    },
};

fn sentry(row: &Value) -> Sentry {
    let remaining = row[0].as_u64().unwrap() as u8;
    let pause = row[1].as_u64().unwrap() as u8;
    if remaining != 0 {
        Sentry::Turning { remaining, pause }
    } else if row[3] == 0 {
        Sentry::Begin
    } else {
        Sentry::Waiting { remaining: pause }
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SLEEP_ORACLE from original sleep controllers"]
fn native_sleep_waking_and_idle_transitions_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_SLEEP_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted = std::collections::BTreeSet::new();
    for row in &rows {
        wanted.extend([
            row["clip"].as_i64().unwrap() as i16,
            row["expected"]["clip"].as_i64().unwrap() as i16,
        ]);
    }
    wanted.extend(registry.clip(0, 0));
    let resources = Resources {
        clips: motions::collect(&disc, wanted)
            .unwrap()
            .into_iter()
            .map(|(id, c)| (id, c.at_gameplay_rate(registry.edition)))
            .collect(),
        registry,
        commands: Commands::load(&disc).unwrap(),
        weapons: Weapons::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        }
        .into(),
    };
    let template = template(&resources);
    let mut sleeping = 0;
    let mut waking = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.state = State::from_action(actor.motion.action).unwrap();
        actor.flags = int("flags") as u32;
        actor.position = vector(&row["position"]);
        actor.rotation = [10, int("yaw") as i16, 20].into();
        actor.velocity = [10, 20, 30].into();
        actor.turn = int("turn") as i16;
        actor.health = int("health") as i16;
        actor.pad.held = int("held") as u16;
        let mut sleep = Sleep {
            wait: int("wait") as i16,
            sentry: sentry(&row["sentry"]),
        };
        let mut destination = [1, 2, 3].into();
        let mut random = Random::new(int("seed") as u32);
        let result = sleep
            .update(
                &mut actor,
                SleepInput {
                    home: vector(&row["home"]),
                    alert: int("alert") as i16,
                },
                &mut destination,
                &mut random,
                |actor, action| {
                    actor
                        .change_motion(
                            action,
                            tenchu_core::tenchu2::actor::Velocity::Motion,
                            &resources,
                        )
                        .map(|_| ())
                },
            )
            .unwrap();
        let actual = json!({
            "action": actor.motion.action, "clip": actor.motion.clip, "frame": actor.motion.frame,
            "loops": actor.motion.loops, "state": actor.state as u8,
            "velocity": actor.velocity.to_array(), "wait": sleep.wait,
            "visibility": if result.unnoticed { -1 } else { 1 },
            "memory": if result.unnoticed { -1 } else { 2 },
            "held": result.held, "destination": destination.to_array(),
            "seed": random.seed, "random": random.calls,
        });
        for (key, value) in actual.as_object().unwrap() {
            assert_eq!(value, &row["expected"][key], "row {id}: {key}");
        }
        assert_eq!(sleep.sentry, sentry(&row["expected"]["sentry"]), "row {id}");
        sleeping += usize::from(result.unnoticed);
        waking += usize::from(result.held == tenchu_core::tenchu2::input::GUARD);
    }
    assert!(rows.len() > 25000 && sleeping > 500 && waking > 1000);
    eprintln!(
        "{} sleep updates: {sleeping} sleeping, {waking} waking",
        rows.len()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CORPSE_ORACLE from original corpse perception"]
fn native_corpse_search_and_terrain_visibility_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = resources();
    let template = template(&resources);
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut count = 0;
    let mut found = 0;
    let mut maps = std::collections::BTreeSet::new();
    for (id, row) in cases::<Value>("TENCHU2_CORPSE_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
            maps.insert(map_id);
        }
        let actors: Vec<_> = (0..5)
            .map(|slot| {
                let mut actor = template.clone();
                let body = slot == 3;
                actor.kind = int(if body { "body_kind" } else { "kind" }) as i16;
                actor.flags = int(if body { "body_flags" } else { "flags" }) as u32;
                actor.state = if body {
                    State::Dead
                } else {
                    State::from_action((int("state") as i16) << 8).unwrap()
                };
                actor.motion.action = if body { 0xb00 } else { int("action") as i16 };
                actor.motion.loops = if body { int("loops") as i16 } else { 0 };
                actor.position = vector(&row[if body { "body" } else { "position" }]);
                actor.rotation.y = int("yaw") as i16;
                actor.height = int("height") as i16;
                actor
            })
            .collect();
        let mut search = CorpseSearch {
            observer: int("last") as usize,
        };
        let mut cursor = Cursor::default();
        let observed = search
            .observe(
                3,
                actors.len(),
                |index| actors.get(index),
                &mut CorpseContext {
                    animated_position: None,
                    player: 0,
                    difficulty: int("difficulty") as u8,
                    distance: int("distance"),
                    terrain: map.as_ref().unwrap(),
                    cursor: &mut cursor,
                    math: &resources.math,
                },
            )
            .unwrap();
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let actual = json!({
            "observer": search.observer, "found": observed,
            "material": cursor.material, "index": cursor.index,
            "current": surface(cursor.current), "last": surface(cursor.last),
        });
        assert_eq!(actual, row["expected"], "row {id}, map {map_id}");
        count += 1;
        found += usize::from(observed.is_some());
    }
    assert_eq!(maps.len(), Level::available(&disc).unwrap().len());
    assert!(count > 30000 && found > 100);
    eprintln!("{count} corpse searches, {found} discoveries");
}
