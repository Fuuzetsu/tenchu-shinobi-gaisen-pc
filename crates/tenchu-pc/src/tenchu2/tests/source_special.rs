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
    random::Random,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{Request, SpecialCue, SpecialInput},
        physics::{Depth, Terrain, Water},
        rig::WeaponMode,
        terrain::{Cursor, Options, Probe, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SPECIAL_ORACLE from original controllers"]
fn native_weapon_transitions_falls_landings_and_taunts_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_SPECIAL_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted = std::collections::BTreeSet::new();
    for row in &rows {
        wanted.extend([
            row["clip"].as_i64().unwrap() as i16,
            row["expected"]["clip"].as_i64().unwrap() as i16,
        ]);
        for action in [0xa23, 0xa24, 0xa26, 0xb00] {
            wanted.extend(registry.clip(row["kind"].as_i64().unwrap() as i16, action));
        }
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
    let mut switches = 0;
    let mut alignment = 0;
    let mut damage = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::Special;
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.motion.pose.rotation[0][1] = 111;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.height = int("height") as i16;
        actor.position = vector(&row["position"]);
        actor.rotation = vector(&row["rotation"]);
        actor.velocity = vector(&row["velocity"]);
        actor.last_vertical_velocity = int("last_y") as i16;
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        actor.terrain.blocked = int("blocked") as u16;
        actor.pad.previous = int("previous") as u16;
        let mut input = SpecialInput {
            held: int("input") as u16,
            player: boolean("player"),
            player_dead: boolean("player_dead"),
            difficulty: int("difficulty") as i16,
            water: match int("water") {
                0 => Some(Depth::Shallow),
                1 => Some(Depth::Wading),
                2 => Some(Depth::Swimming),
                3 => Some(Depth::Submerged),
                _ => None,
            },
            camera_actor: boolean("camera_actor"),
            camera: int("camera") as i16,
        };
        let mut request = Request {
            action: int("request") as i16,
            velocity: Velocity::Keep,
        };
        let mut cursor = Cursor::default();
        let mut scratch = Probe::default();
        let mut water = Water::default();
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
        let mut owner = false;
        let mut floor = |actor: &mut tenchu_core::tenchu2::actor::Actor,
                         calls: &mut Vec<Value>|
         -> tenchu_core::Result<()> {
            if actor.align_to_floor(&mut terrain, &resources.math, |_| {
                calls.push(json!(["point"]));
                Ok(vector(&row["root"]))
            })? {
                calls.push(json!(["publish"]));
                alignment += 1;
            }
            Ok(())
        };
        if row["op"] == "floor" {
            floor(&mut actor, &mut calls).unwrap();
        } else {
            let held = input.held;
            actor
                .update_special(
                    &mut request,
                    &mut input,
                    &rules,
                    &resources,
                    &mut random,
                    |actor, request, _random, cue| {
                        match cue {
                            SpecialCue::Weapon(mode) => {
                                let drawing = mode == WeaponMode::Draw;
                                calls.push(json!(["weapon", u8::from(!drawing)]));
                                if drawing {
                                    actor.flags |= 0x40;
                                } else {
                                    actor.flags &= !0x40;
                                }
                                switches += 1;
                            }
                            SpecialCue::Voice(value) => calls.push(json!(["voice", value])),
                            SpecialCue::Attack => {
                                calls.push(json!(["attack"]));
                                request.action = 0x900;
                                request.velocity = Velocity::Motion;
                            }
                            SpecialCue::Recover => {
                                request.action = actor.normal_motion(held, |actor| {
                                    calls.push(json!(["root"]));
                                    Ok(actor.position)
                                })?;
                                request.velocity = Velocity::Motion;
                            }
                            SpecialCue::AlignToFloor => floor(actor, &mut calls)?,
                            SpecialCue::ShowHealth => calls.push(json!(["health"])),
                            SpecialCue::Landing => calls.push(json!(["rumble", 2, 1])),
                            SpecialCue::Dust => calls.push(json!(["dust", 0])),
                            SpecialCue::Splash => calls.push(json!(["splash", 1])),
                            SpecialCue::Throw {
                                bone,
                                center,
                                direction,
                            } => {
                                calls.push(json!(["attachment", bone, center.to_array()]));
                                calls.push(json!(["throw", direction.to_array()]));
                                owner = true;
                            }
                        }
                        Ok(())
                    },
                )
                .unwrap();
        }
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
        let actual = json!({"action":request.action,"mode":match request.velocity {Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1},"health":actor.health,"flags":actor.flags,"position":actor.position.to_array(),"velocity":actor.velocity.to_array(),"rotation":actor.rotation.to_array(),"root_yaw":actor.motion.pose.rotation[0][1],"mask":actor.motion.mask,"motion":actor.motion.action,"clip":actor.motion.clip,"frame":actor.motion.frame,"loops":actor.motion.loops,"state":state,"camera":input.camera,"seed":random.seed,"random":random.calls,"owner":owner,"current":surface(terrain.cursor.current),"last":surface(terrain.cursor.last),"material":terrain.cursor.material,"index":terrain.cursor.index,"calls":calls});
        assert_eq!(
            actual,
            row["expected"],
            "case {id}, action {}",
            int("action")
        );
        damage += usize::from(actor.health != int("health") as i16);
    }
    assert_eq!(rows.len(), 23_000);
    assert!(switches > 100 && alignment > 100);
    eprintln!(
        "23000 special/floor source checks: {switches} weapon transitions, {alignment} alignments, {damage} damage changes"
    );
}
