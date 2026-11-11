use super::{
    source_control::{cases, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    random::Random,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{Request, StandingCue, StandingInput},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STANDING_ORACLE from original standing/ready controllers"]
fn native_standing_ready_dodges_and_carrying_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_STANDING_ORACLE").collect();
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
    let mut seen = std::collections::BTreeSet::new();
    for (id, row) in rows.iter().enumerate() {
        let int = |k: &str| row[k].as_i64().unwrap() as i32;
        let boolean = |k: &str| row[k].as_bool().unwrap();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = if row["variant"] == "normal" {
            State::Normal
        } else {
            State::Ready
        };
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = row["mask"].as_u64().unwrap() as u32;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.skill = int("skill") as i16;
        actor.height = int("height") as i16;
        actor.turn = int("turn") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.velocity = vector(&row["velocity"]);
        actor.position = vector(&row["position"]);
        actor.previous = actor.position;
        actor.previous.y += int("dy");
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        actor.pad.previous = int("previous") as u16;
        actor.pad.history = std::array::from_fn(|i| row["history"][i].as_u64().unwrap() as u16);
        let input = StandingInput {
            unarmed: false,
            held: int("input") as u16,
            player: boolean("player"),
            controls_enabled: boolean("enabled"),
            water: int("water") >= 0,
            map: int("map") as i16,
            sequence: int("sequence") as i16,
            story_triggered: boolean("trigger"),
            carried_action: Some(int("carried") as i16),
        };
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut calls = Vec::new();
        let mut trigger = input.story_triggered;
        let mut secret = 0;
        actor
            .update_standing(
                &mut request,
                &input,
                &resources,
                &mut random,
                |actor, request, _, cue| {
                    match cue {
                        StandingCue::Status(status) => {
                            if status.died {
                                calls.push(json!(["status", 0]));
                                if input.player {
                                    calls.extend([json!(["status", 1]), json!(["status", 2])]);
                                } else {
                                    calls.push(json!(["voice", 15]));
                                }
                            }
                            if status.show_health {
                                calls.push(json!(["status", 3]));
                            }
                            if status.staggered {
                                calls.push(json!(["rumble", 4, 1]));
                            }
                        }
                        StandingCue::Voice(id) => calls.push(json!(["voice", id])),
                        StandingCue::Splash(kind) => calls.push(json!(["water", kind])),
                        StandingCue::Dust => calls.push(json!(["dust"])),
                        StandingCue::Footstep { check_surface } => {
                            if check_surface {
                                calls.push(json!(["surface"]));
                            }
                            if check_surface && int("surface") == 18 {
                                calls.extend([json!(["alert", 3]), json!(["sound", 3])]);
                            } else {
                                calls.push(json!(["sample", 18, 42]));
                            }
                        }
                        StandingCue::Ledge => {
                            calls.push(json!(["ledge"]));
                            return Ok(boolean("ledge"));
                        }
                        StandingCue::Jump => {
                            calls.push(json!(["jump"]));
                            request.action = 0x600;
                            request.velocity = Velocity::Keep;
                            actor.motion.action = 0x600;
                            actor.motion.frame = -5;
                            actor.motion.loops = 0;
                            actor.state = State::Jumping;
                        }
                        StandingCue::Attack => {
                            calls.push(json!(["attack"]));
                            request.action = 0x900;
                            request.velocity = Velocity::Keep;
                        }
                        StandingCue::Item => {
                            calls.push(json!(["item"]));
                            request.action = 0x802;
                            request.velocity = Velocity::Motion;
                        }
                        StandingCue::Recover => {
                            request.action = actor.normal_motion(input.held, |actor| {
                                calls.push(json!(["root"]));
                                Ok(actor.position)
                            })?;
                            request.velocity = Velocity::Motion;
                        }
                        StandingCue::HiddenMission => {
                            trigger = true;
                            secret = 0x6d6;
                            calls.push(json!(["sound", 1]));
                        }
                        StandingCue::HiddenCarry => {
                            trigger = true;
                            for _ in 0..10 {
                                calls.push(json!(["hidden_carry"]));
                            }
                            calls.push(json!(["sound", 1]));
                        }
                    }
                    Ok(false)
                },
            )
            .unwrap();
        let actual = json!({"flags":actor.flags,"health":actor.health,"action":request.action,"mode":match request.velocity { Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1 },"motion":actor.motion.action,"state":match actor.state {State::Normal=>0,State::Ready=>1,State::Jumping=>6,_=>panic!("Unexpected test state")},"frame":actor.motion.frame,"loops":actor.motion.loops,"yaw":actor.rotation.y,"velocity":actor.velocity.to_array(),"position":actor.position.to_array(),"ground":actor.terrain.height,"mask":actor.motion.mask,"history":actor.pad.history,"trigger":u8::from(trigger),"secret":secret,"seed":random.seed,"random":random.calls,"calls":calls});
        assert_eq!(
            actual,
            row["expected"],
            "case {id}, kind {}, action {}",
            actor.kind,
            int("action")
        );
        seen.insert(actor.kind);
    }
    assert_eq!(seen.len(), 130);
    assert_eq!(rows.len(), 40_000);
    eprintln!(
        "40000 native standing/ready steps match original instructions for all 130 definitions"
    );
}
