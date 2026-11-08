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
        control::{DeathCue, DeathInput, DeathPlayer, Request, StealthPlayback},
        physics::{Depth, Terrain, Water},
        rig::Grip,
        terrain::{Cursor, Options, Probe, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_DEATH_ORACLE from original controllers"]
fn native_death_carrying_and_stealth_events_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_DEATH_ORACLE").collect();
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
        },
    };
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut carries = 0;
    let mut events = 0;
    let mut finished = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let link = |key: &str| row[key].as_u64().map(|v| v as usize);
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::Dead;
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.motion.pose.rotation[0] = [10, 111, 20];
        actor.motion.pose.translation[0] = Position::new(111, 222, 333);
        actor.flags = int("flags") as u32;
        actor.health = 0;
        actor.height = int("height") as i16;
        actor.width = int("width") as i16;
        actor.turn = int("turn") as i16;
        actor.position = vector(&row["position"]);
        actor.rotation = vector(&row["rotation"]);
        actor.velocity = vector(&row["velocity"]);
        actor.target = link("target");
        actor.terrain.blocked = int("blocked") as u16;
        let mut input = DeathInput {
            actor: 5,
            is_player: row["player"].as_bool().unwrap(),
            player: DeathPlayer {
                position: vector(&row["player_position"]),
                yaw: int("player_yaw") as i16,
                velocity: vector(&row["player_velocity"]),
                turn: int("player_turn") as i16,
                action: int("player_action") as i16,
                target: link("player_target"),
            },
            tick: int("tick"),
            map: int("map") as i16,
            stealth: Some(StealthPlayback {
                hero: int("hero") as usize,
                style: int("style") as usize,
                next: int("next") as usize,
            }),
            focus: link("focus"),
            carry_point: vector(&row["point"]),
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
            bottom: 0,
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
        let mut body_flags = int("body_flags") as u16;
        let mut body_size = [250, 800, 250];
        actor
            .update_dead(
                &mut request,
                &mut input,
                &mut terrain,
                &resources,
                &mut random,
                |actor, _request, input, random, _, cue| {
                    match cue {
                        DeathCue::CarryPoint => {
                            calls.push(json!(["point", "player", 5]));
                            carries += 1;
                        }
                        DeathCue::BodyDisabled(disabled) => {
                            if disabled {
                                body_flags |= 0x8000;
                            } else {
                                body_flags &= !0x8000;
                            }
                        }
                        DeathCue::Voice { player, id } => calls.push(json!(["voice", player, id])),
                        DeathCue::Sound(value) => calls.push(json!(["sound", value])),
                        DeathCue::Rumble { kind, strength } => {
                            calls.push(json!(["rumble", kind, strength]))
                        }
                        DeathCue::Blood { joint, angle } => {
                            calls.push(json!(["point", "actor", joint]));
                            let angle = angle.unwrap_or_else(|| random.next15() as i16);
                            calls.push(json!(["blood", input.carry_point.to_array(), angle]));
                        }
                        DeathCue::Bubbles => calls.push(json!(["bubbles", 2])),
                        DeathCue::WaterRipple { position, size } => {
                            calls.push(json!(["ripple", position.to_array(), size]))
                        }
                        DeathCue::Splash(value) => calls.push(json!(["splash", value])),
                        DeathCue::PlayerGrip(grip) => calls.push(json!([
                            "grip",
                            0,
                            3,
                            match grip {
                                Grip::Swap => 0,
                                Grip::Release => 1,
                                Grip::Hold => 2,
                            }
                        ])),
                        DeathCue::HideHeldItem => calls.push(json!(["hide"])),
                        DeathCue::RemoveWeapons => calls.push(json!(["weapon", 3])),
                        DeathCue::GroundPool => calls.push(json!(["pool"])),
                        DeathCue::RemoveJoint(joint) => calls.push(json!(["remove", joint, 0])),
                        DeathCue::BodyCollision => {
                            calls.push(json!(["body", 1]));
                            body_flags = 0x2000;
                            body_size = [actor.width / 2, actor.height / 8, actor.width / 2];
                        }
                        DeathCue::ResizeBody => {
                            body_size = [actor.width, actor.height, actor.width]
                        }
                        DeathCue::FinishContacts => calls.push(json!(["contacts", 1])),
                        DeathCue::AlignToFloor => calls.push(json!(["align"])),
                    }
                    Ok(())
                },
            )
            .unwrap();
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let actual = json!({"action":request.action,"mode":match request.velocity {Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1},"height":actor.height,"width":actor.width,"target":actor.target,"flags":actor.flags,"position":actor.position.to_array(),"velocity":actor.velocity.to_array(),"rotation":actor.rotation.to_array(),"root_rotation":actor.motion.pose.rotation[0],"mask":actor.motion.mask,"motion":actor.motion.action,"clip":actor.motion.clip,"frame":actor.motion.frame,"loops":actor.motion.loops,"state":actor.state as u8,"next":input.stealth.unwrap().next,"focus":input.focus,"body_flags":body_flags,"body_size":body_size,"point":input.carry_point.to_array(),"seed":random.seed,"random":random.calls,"current":surface(terrain.cursor.current),"last":surface(terrain.cursor.last),"material":terrain.cursor.material,"index":terrain.cursor.index,"calls":calls});
        assert_eq!(actual, row["expected"], "case {id}: {row}");
        events += usize::from(input.stealth.unwrap().next != int("next") as usize);
        finished +=
            usize::from(actor.motion.loops == i16::MIN && int("loops") != i32::from(i16::MIN));
    }
    assert_eq!(rows.len(), 27_600);
    assert!(events > 100 && carries > 100 && finished > 100);
    eprintln!(
        "27600 death source updates; {carries} carries, {events} stealth events, {finished} settled bodies"
    );
}
