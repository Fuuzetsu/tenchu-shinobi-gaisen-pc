use super::{
    source_control::{cases, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    collision::Rules, commands::Commands, level::Level, math_tables, motions, registry::Registry,
    weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{LedgeCue, LedgeInput, Request},
        physics::{Terrain, Water},
        terrain::{Cursor, Options, Probe, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_LEDGE_ORACLE from original ledge controllers"]
fn native_ledge_grabs_sideways_hanging_and_climbing_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_LEDGE_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted: std::collections::BTreeSet<_> = rows
        .iter()
        .flat_map(|row| {
            [
                row["clip"].as_i64().unwrap() as i16,
                row["expected"]["clip"].as_i64().unwrap() as i16,
            ]
        })
        .collect();
    wanted.extend(registry.clip(0, 0));
    let resources = Resources {
        clips: motions::collect(&disc, wanted)
            .unwrap()
            .into_iter()
            .map(|(id, c)| (id, c.at_gameplay_rate(registry.edition)))
            .collect(),
        registry,
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let template = template(&resources);
    let rules = Rules::load(&disc).unwrap();
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut successes = 0;
    let mut grabs = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |name: &str| row[name].as_i64().unwrap() as i32;
        let boolean = |name: &str| row[name].as_bool().unwrap();
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.motion.action = int("action") as i16;
        actor.state = State::from_action(actor.motion.action).unwrap();
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.health = int("health") as i16;
        actor.position = vector(&row["position"]);
        actor.velocity = vector(&row["velocity"]);
        actor.rotation.y = int("yaw") as i16;
        actor.width = int("width") as i16;
        actor.height = int("height") as i16;
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        let mut cursor = Cursor::default();
        let mut scratch = Probe::default();
        let mut water = Water::default();
        let mut terrain = Terrain {
            map: map.as_ref().unwrap(),
            rules: &rules,
            cursor: &mut cursor,
            scratch: &mut scratch,
            water: &mut water,
            options: Options::default(),
        };
        let mut request = Request {
            action: int("request") as i16,
            velocity: Velocity::Keep,
        };
        let mut input = LedgeInput {
            held: int("input") as u16,
            player: boolean("player"),
            tick: int("tick"),
            check: int("check") as i16,
            grapple_camera: boolean("camera"),
            map: int("map") as i16,
            story_triggered: boolean("trigger"),
        };
        let mut calls = Vec::new();
        let mut unlocked = 0;
        let mut cue =
            |actor: &mut tenchu_core::tenchu2::actor::Actor, request: &mut Request, held, cue| {
                match cue {
                    LedgeCue::Voice(value) => calls.push(json!(["voice", value])),
                    LedgeCue::Grabbed => {
                        calls.push(json!(["rumble", 2, 1]));
                        grabs += 1;
                    }
                    LedgeCue::Recover => {
                        request.action = actor.normal_motion(held, |actor| {
                            calls.push(json!(["root"]));
                            Ok(actor.position)
                        })?;
                        request.velocity = Velocity::Motion;
                    }
                    LedgeCue::ReleaseCamera => {}
                    LedgeCue::HiddenMission => {
                        unlocked |= 0x80;
                        calls.push(json!(["sound", 1]));
                    }
                }
                Ok(())
            };
        let result = if row["op"] == "entry" {
            actor
                .enter_ledge(&mut request, &mut input, &mut terrain, &resources, &mut cue)
                .unwrap()
        } else {
            actor
                .update_ledge(&mut request, &mut input, &mut terrain, &resources, &mut cue)
                .unwrap();
            false
        };
        let state = match actor.state {
            State::Normal => 0,
            State::Ready => 1,
            State::Crouched => 2,
            State::Wall => 3,
            State::Ledge => 4,
            State::Swimming => 5,
            State::Jumping => 6,
            State::Special => 7,
            State::Item => 8,
            State::Attack => 9,
            State::Hurt => 10,
            State::Dead => 11,
        };
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let actual = json!({"action":request.action,"mode":match request.velocity { Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1 },
            "input":input.held,"position":actor.position.to_array(),"velocity":actor.velocity.to_array(),
            "yaw":actor.rotation.y,"mask":actor.motion.mask,"motion":actor.motion.action,"clip":actor.motion.clip,
            "frame":actor.motion.frame,"loops":actor.motion.loops,"state":state,"check":input.check,
            "camera":input.grapple_camera,"trigger":input.story_triggered,"unlocked":unlocked,
            "current":surface(terrain.cursor.current),"last":surface(terrain.cursor.last),"material":terrain.cursor.material,
            "index":terrain.cursor.index,"result":result,"calls":calls });
        assert_eq!(actual, row["expected"], "case {id}: {}", row["op"]);
        successes += usize::from(result);
    }
    assert_eq!(rows.len(), 32_200);
    assert!(successes > 50 && grabs > 10);
    eprintln!("32200 source ledge checks: {successes} successful checks, {grabs} new grabs");
}
