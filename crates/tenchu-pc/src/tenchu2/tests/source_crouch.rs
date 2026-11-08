use super::{
    source_control::{cases, template},
    source_physics::{probe, probe_values, vector},
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    collision::Rules, commands::Commands, level::Level, math_tables, motions, registry::Registry,
    weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    scene::Scene,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{CrouchCue, CrouchInput, Request},
        physics::{Depth, Terrain, Water},
        rig::Rig,
        terrain::{Cursor, Options, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CROUCH_ORACLE from original crouch/body/wall routines"]
fn native_crouching_looting_carrying_and_wall_entry_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_CROUCH_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted: std::collections::BTreeSet<_> = rows
        .iter()
        .map(|r| r["clip"].as_i64().unwrap() as i16)
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
        }
        .into(),
    };
    let template = template(&resources);
    let rules = Rules::load(&disc).unwrap();
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut searched = 0;
    let mut carried_count = 0;
    let mut wall_count = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |name: &str| row[name].as_i64().unwrap() as i32;
        let boolean = |name: &str| row[name].as_bool().unwrap();
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action((int("state") << 8) as i16).unwrap();
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.motion.pose.rotation[0][1] = 123;
        actor.position = vector(&row["position"]);
        actor.velocity = vector(&row["velocity"]);
        actor.height = int("height") as i16;
        actor.width = int("width") as i16;
        actor.turn = int("turn") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.flags = int("flags") as u32;
        actor.pad.previous = int("previous") as u16;
        actor.pad.history = std::array::from_fn(|i| row["history"][i].as_u64().unwrap() as u16);
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        actor.inventory.counts =
            std::array::from_fn(|i| row["inventory"][i].as_u64().unwrap() as u8);
        let mut body = template.clone();
        body.kind = int("target_kind") as i16;
        body.health = int("target_health") as i16;
        body.motion.loops = int("target_loops") as i16;
        body.state = State::from_action((int("target_state") << 8) as i16).unwrap();
        body.terrain = probe(&row["target_probe"]);
        body.position = vector(&row["target_position"]);
        body.inventory.counts = std::array::from_fn(|i| row["loot"][i].as_u64().unwrap() as u8);
        body.inventory.resurrections = if boolean("searched") { u8::MAX } else { 0 };
        let carry_body = body.clone();
        let mut scene = Scene::default();
        let rig = Rig::bind(&body, &mut scene, &resources.math);
        for (i, &joint) in rig.joints.iter().enumerate() {
            if !row["parents"][i].as_bool().unwrap() {
                scene.models[joint].parent = None;
            }
        }
        let mut scratch = probe(&row["scratch"]);
        let mut cursor = Cursor::default();
        let mut water = Water {
            depth: match int("water") {
                -1 => None,
                0 => Some(Depth::Shallow),
                1 => Some(Depth::Wading),
                _ => Some(Depth::Swimming),
            },
            ..Water::default()
        };
        let mut terrain = Terrain {
            map: map.as_mut().unwrap(),
            rules: &rules,
            cursor: &mut cursor,
            scratch: &mut scratch,
            water: &mut water,
            options: Options {
                avoid_water: boolean("avoid"),
                ignore_steps: boolean("ignore"),
            },
        };
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let unlocked = std::array::from_fn(|i| row["unlocked"][i].as_bool().unwrap());
        let mut input = CrouchInput {
            held: int("input") as u16,
            body: boolean("found").then_some(&mut body),
            unlocked: &unlocked,
            wall_allowed: boolean("wall"),
        };
        let mut calls = Vec::new();
        let mut carried = false;
        let carry = |actor: &mut tenchu_core::tenchu2::actor::Actor,
                     request: &mut Request,
                     calls: &mut Vec<Value>| {
            calls.push(json!(["carry"]));
            if boolean("found") && actor.can_carry(&carry_body, &rig, &scene, boolean("player")) {
                carried = true;
                calls.push(json!(["rumble", 2, 1]));
                request.action = 0x120;
                request.velocity = Velocity::Motion;
            }
        };
        let mut carry = carry;
        let mut wall_result = false;
        match row["op"].as_str().unwrap() {
            "carry" => carry(&mut actor, &mut request, &mut calls),
            "wall" => {
                wall_result = actor
                    .enter_wall(
                        &mut request,
                        input.held,
                        input.wall_allowed,
                        &mut terrain,
                        &resources,
                    )
                    .unwrap()
            }
            "crouch" => actor
                .update_crouched(
                    &mut request,
                    &mut input,
                    &mut terrain,
                    &resources,
                    |actor, request, held, cue| {
                        match cue {
                            CrouchCue::Root => {
                                calls.push(json!(["root"]));
                                actor.commit_root(vector(&row["root"]));
                            }
                            CrouchCue::Recover => {
                                request.action = actor.normal_motion(held, |_| {
                                    calls.push(json!(["root"]));
                                    Ok(vector(&row["root"]))
                                })?;
                                request.velocity = Velocity::Motion;
                            }
                            CrouchCue::Carry => carry(actor, request, &mut calls),
                            CrouchCue::Attack => {
                                calls.push(json!(["attack"]));
                                request.action = 0x900;
                                request.velocity = Velocity::Keep;
                            }
                            CrouchCue::Item => {
                                calls.push(json!(["item"]));
                                request.action = 0x802;
                                request.velocity = Velocity::Motion;
                            }
                            CrouchCue::Voice(value) => calls.push(json!(["voice", value])),
                            CrouchCue::Sound(value) => calls.push(json!(["sound", value])),
                            CrouchCue::Splash(value) => calls.push(json!(["splash", value])),
                            CrouchCue::Pickup(value) => calls.push(json!(["pickup", value])),
                        }
                        Ok(())
                    },
                )
                .unwrap(),
            _ => unreachable!(),
        }
        let held = input.held;
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let actual = json!({
            "action":request.action,"mode": match request.velocity { Velocity::Stop => -1, Velocity::Keep => 0, Velocity::Motion => 1 },
            "input":held,"position":actor.position.to_array(),"velocity":actor.velocity.to_array(),
            "yaw":actor.rotation.y,"root_yaw":actor.motion.pose.rotation[0][1],"mask":actor.motion.mask,"history":actor.pad.history,
            "inventory":actor.inventory.counts,"loot":body.inventory.counts,"searched":body.inventory.resurrections != 0,"carried":carried,
            "scratch":probe_values(terrain.scratch),"current":surface(terrain.cursor.current),"last":surface(terrain.cursor.last),
            "material":terrain.cursor.material,"index":terrain.cursor.index,"wall_result":wall_result,"calls":calls
        });
        assert_eq!(actual, row["expected"], "case {id}: {}", row["op"]);
        searched += usize::from(body.inventory.resurrections != 0 && !boolean("searched"));
        carried_count += usize::from(carried);
        wall_count += usize::from(wall_result || request.action == 0x300);
    }
    assert_eq!(rows.len(), 20_700);
    assert!(searched > 1000 && carried_count > 800 && wall_count > 5);
    eprintln!(
        "20700 source comparisons: {searched} searches, {carried_count} carries, {wall_count} wall entries"
    );
}
