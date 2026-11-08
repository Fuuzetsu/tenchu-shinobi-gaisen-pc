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
    math::{Math, Position},
    random::Random,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{GrappleStatus, Request, SwimCue, SwimInput},
        inventory::Item,
        physics::{Depth, Terrain, Water},
        rig::{Grip, WeaponMode},
        terrain::{Cursor, Options, Probe, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SWIMMING_ORACLE from original controllers"]
fn native_swimming_diving_shore_climbing_and_grapple_inputs_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_SWIMMING_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted = std::collections::BTreeSet::new();
    for row in &rows {
        wanted.extend([
            row["clip"].as_i64().unwrap() as i16,
            row["expected"]["clip"].as_i64().unwrap() as i16,
        ]);
        wanted.extend(registry.clip(row["kind"].as_i64().unwrap() as i16, 0x606));
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
        },
    };
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut hooks = 0;
    let mut shore = 0;
    let mut breathing = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::Swimming;
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.motion.pose.rotation[0] = [10, 111, 20];
        actor.motion.pose.translation[0] = Position::new(111, 222, 333);
        actor.flags = int("flags") as u32;
        actor.health = 100;
        actor.height = int("height") as i16;
        actor.breath = int("breath") as u8;
        actor.turn = int("turn") as i16;
        actor.position = vector(&row["position"]);
        actor.rotation = vector(&row["rotation"]);
        actor.velocity = vector(&row["velocity"]);
        actor.pad.previous = int("previous") as u16;
        actor.pad.history = std::array::from_fn(|i| row["history"][i].as_u64().unwrap() as u16);
        actor.inventory.selected = Item::try_from(int("selected") as u8).ok();
        actor.inventory.counts[3] = int("count") as u8;
        let mut input = SwimInput {
            held: int("input") as u16,
            tick: int("tick"),
            camera_actor: row["camera_actor"].as_bool().unwrap(),
            camera: int("camera") as i16,
            previous_camera: int("old_camera") as i16,
            aim: vector(&row["aim"]),
            grapple: GrappleStatus::Flying,
            model_height: int("model_height") as i16,
        };
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut cursor = Cursor::default();
        let mut scratch = Probe::default();
        let mut water = Water {
            depth: match int("water") {
                0 => Some(Depth::Shallow),
                1 => Some(Depth::Wading),
                2 => Some(Depth::Swimming),
                3 => Some(Depth::Submerged),
                _ => None,
            },
            surface: int("surface"),
            bottom: int("bottom"),
        };
        let mut terrain = Terrain {
            map: map.as_mut().unwrap(),
            rules: &rules,
            cursor: &mut cursor,
            scratch: &mut scratch,
            water: &mut water,
            options: Options::default(),
        };
        let mut random = Random::new(int("seed") as u32);
        let mut calls = Vec::new();
        actor
            .update_swimming(
                &mut request,
                &mut input,
                &mut terrain,
                &resources,
                &mut random,
                |actor, request, input, _random, cue| {
                    match cue {
                        SwimCue::Recover => {
                            request.action = actor.normal_motion(input.held, |actor| {
                                calls.push(json!(["root"]));
                                Ok(actor.position)
                            })?;
                            request.velocity = Velocity::Motion;
                        }
                        SwimCue::CommitRoot => calls.push(json!(["root"])),
                        SwimCue::Submerge => {
                            calls.push(json!(["point"]));
                            actor.position = vector(&row["root"]);
                            calls.push(json!(["size"]));
                        }
                        SwimCue::PublishRoot => calls.push(json!(["publish"])),
                        SwimCue::Jump | SwimCue::Attack => {
                            let attack = cue == SwimCue::Attack;
                            calls.push(json!([if attack { "attack" } else { "jump" }]));
                            request.action = if attack { 0x900 } else { 0x600 };
                            request.velocity = Velocity::Motion;
                        }
                        SwimCue::Voice(value) => calls.push(json!(["voice", value])),
                        SwimCue::Splash(value) => calls.push(json!(["splash", value])),
                        SwimCue::Weapon(mode) => {
                            let drawing = mode == WeaponMode::Draw;
                            calls.push(json!(["weapon", u8::from(!drawing)]));
                            if drawing {
                                actor.flags |= 0x40;
                            } else {
                                actor.flags &= !0x40;
                            }
                        }
                        SwimCue::Grip { slot, spare, grip } => calls.push(json!([
                            "grip",
                            slot,
                            spare,
                            match grip {
                                Grip::Swap => 0,
                                Grip::Release => 1,
                                Grip::Hold => 2,
                            }
                        ])),
                        SwimCue::EmptyItem => calls.push(json!(["sound", 6])),
                        SwimCue::LaunchGrapple => {
                            calls.push(json!(["launch", input.aim.to_array()]));
                            hooks += 1;
                        }
                        SwimCue::UpdateGrapple => {
                            calls.push(json!(["grapple"]));
                            input.grapple = match int("grapple") {
                                -1 => GrappleStatus::Failed,
                                0 => GrappleStatus::Flying,
                                1 => GrappleStatus::Attached,
                                _ => GrappleStatus::Released,
                            };
                        }
                        SwimCue::BreathingTube(show) => {
                            calls.push(json!(["tube", show.then_some(23)]))
                        }
                        SwimCue::UseUnderwaterItem => {
                            calls.push(json!(["bubbles", 2]));
                            breathing += 1;
                        }
                    }
                    Ok(())
                },
            )
            .unwrap();
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
        let actual = json!({"action":request.action,"mode":match request.velocity {Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1},"input":input.held,"breath":actor.breath,"count":actor.inventory.counts[3],"height":actor.height,"flags":actor.flags,"position":actor.position.to_array(),"velocity":actor.velocity.to_array(),"rotation":actor.rotation.to_array(),"root_rotation":actor.motion.pose.rotation[0],"root_position":actor.motion.pose.translation[0].to_array(),"mask":actor.motion.mask,"motion":actor.motion.action,"clip":actor.motion.clip,"frame":actor.motion.frame,"loops":actor.motion.loops,"state":state,"camera":input.camera,"old_camera":input.previous_camera,"seed":random.seed,"random":random.calls,"current":surface(terrain.cursor.current),"last":surface(terrain.cursor.last),"material":terrain.cursor.material,"index":terrain.cursor.index,"calls":calls});
        assert_eq!(
            actual,
            row["expected"],
            "case {id}, action {}",
            int("action")
        );
        shore += usize::from(request.action == 0x520);
    }
    assert_eq!(rows.len(), 23_000);
    assert!(hooks > 50 && breathing > 20 && shore > 20);
    eprintln!(
        "23000 swimming source checks: {hooks} launches, {shore} shore climbs, {breathing} air refills"
    );
}
