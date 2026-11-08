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
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        control::{
            AttackStart, Request, StealthCue, StealthInput, StealthPlayback, StealthSession,
        },
    },
};

fn actor(row: &Value, kind: i16, template: &Actor) -> Actor {
    let mut actor = template.clone();
    actor.kind = kind;
    actor.state = State::from_action((row["state"].as_i64().unwrap() as i16) << 8).unwrap();
    actor.flags = row["flags"].as_u64().unwrap() as u32;
    actor.health = row["health"].as_i64().unwrap() as i16;
    actor.height = 1800;
    actor.position = vector(&row["position"]);
    actor.rotation = vector(&row["rotation"]);
    actor.velocity = vector(&row["velocity"]);
    actor.target = row["target"].as_bool().unwrap().then_some(8);
    actor.motion.action = row["motion"].as_i64().unwrap() as i16;
    actor.motion.clip = Some(row["clip"].as_i64().unwrap() as i16);
    actor.motion.frame = row["frame"].as_i64().unwrap() as i16;
    actor.motion.loops = row["loops"].as_i64().unwrap() as i16;
    actor.motion.mask = row["mask"].as_u64().unwrap() as u32;
    actor.motion.pose.rotation[0] =
        std::array::from_fn(|i| row["root_rotation"][i].as_i64().unwrap() as i16);
    actor.motion.pose.translation[0] = vector(&row["root_position"]);
    actor
}

fn state(actor: &Actor) -> Value {
    json!({"flags":actor.flags,"health":actor.health,"target":actor.target.is_some(),"position":actor.position.to_array(),"rotation":actor.rotation.to_array(),"velocity":actor.velocity.to_array(),"root_rotation":actor.motion.pose.rotation[0],"root_position":actor.motion.pose.translation[0].to_array(),"mask":actor.motion.mask,"motion":actor.motion.action,"clip":actor.motion.clip,"frame":actor.motion.frame,"loops":actor.motion.loops,"state":actor.state as u8})
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_STEALTH_ORACLE and TENCHU2_STEALTH_UPDATE_ORACLE"]
fn paired_stealth_setup_interruption_and_recovery_match_source() {
    let starts: Vec<Value> = cases("TENCHU2_STEALTH_ORACLE").collect();
    let updates: Vec<Value> = cases("TENCHU2_STEALTH_UPDATE_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted = std::collections::BTreeSet::new();
    for row in starts.iter().chain(&updates) {
        for state in [
            &row["actor"],
            &row["victim"],
            &row["expected"]["actor"],
            &row["expected"]["victim"],
        ] {
            if let Some(clip) = state["clip"].as_i64() {
                wanted.insert(clip as i16);
            }
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
    let template = template(&resources);
    for (id, row) in starts.iter().enumerate() {
        let mut player = actor(
            &row["actor"],
            row["hero"].as_i64().unwrap() as i16,
            &template,
        );
        let mut victim = actor(
            &row["victim"],
            row["victim_kind"].as_i64().unwrap() as i16,
            &template,
        );
        let mut request = Request {
            action: player.motion.action,
            velocity: Velocity::Keep,
        };
        let start = player
            .attack_input(
                request,
                0,
                Some(&victim),
                row["map"].as_i64().unwrap() as i16,
                &resources.math,
            )
            .unwrap();
        let AttackStart::Stealth(style) = start.start else {
            panic!("Not a stealth start: {row}");
        };
        let mut calls = Vec::new();
        if start.draw_weapon {
            calls.push(json!(["draw"]));
            player.flags |= 0x40;
        }
        let mut input = StealthInput {
            hero: row["hero"].as_u64().unwrap() as usize,
            victim: 8,
            style,
            penalties: row["penalties"].as_u64().unwrap() as u8,
            story_damage: 0,
        };
        let session = player
            .start_stealth(
                &mut victim,
                &mut request,
                &mut input,
                &resources,
                |_, _, cue| {
                    calls.push(match cue {
                        StealthCue::VictimWeapons => json!(["weapon", 3]),
                        StealthCue::VictimCollision => json!(["body", 0]),
                        StealthCue::KillCredit => json!(["kill"]),
                        _ => panic!("Unexpected start cue {cue:?}"),
                    });
                    Ok(())
                },
            )
            .unwrap();
        let actual = json!({"actor":state(&player),"victim":state(&victim),"action":request.action,"mode":1,"origin":session.origin.to_array(),"next":session.playback.next,"penalties":input.penalties,"story_damage":input.story_damage,"calls":calls,"style":style.index()});
        assert_eq!(actual, row["expected"], "start {id}: {row}");
    }
    let mut interrupted = 0;
    let mut completed = 0;
    for (id, row) in updates.iter().enumerate() {
        let mut player = actor(
            &row["actor"],
            row["hero"].as_i64().unwrap() as i16,
            &template,
        );
        player.terrain.height = row["ground"].as_i64().unwrap() as i32;
        let mut victim = template.clone();
        victim.position = vector(&row["victim_position"]);
        victim.target = row["victim_target"].as_bool().unwrap().then_some(0);
        let mut request = Request {
            action: player.motion.action,
            velocity: Velocity::Keep,
        };
        let session = StealthSession {
            playback: StealthPlayback {
                hero: row["hero"].as_u64().unwrap() as usize,
                style: 0,
                next: 0,
            },
            origin: vector(&row["origin"]),
        };
        let held = row["input"].as_u64().unwrap() as u16;
        let mut calls = Vec::new();
        player
            .update_stealth(
                &victim,
                &mut request,
                &session,
                &resources,
                |actor, request, cue| {
                    match cue {
                        StealthCue::Root => {
                            calls.push(json!(["root"]));
                            actor.commit_root(vector(&row["published"]));
                        }
                        StealthCue::Recover => {
                            calls.push(json!(["normal"]));
                            request.action = actor.normal_motion(held, |_| {
                                calls.push(json!(["root"]));
                                Ok(vector(&row["published"]))
                            })?;
                            request.velocity = Velocity::Motion;
                        }
                        StealthCue::ReleaseGrip => calls.push(json!(["grip", 0, 3, 1])),
                        StealthCue::Voice(id) => calls.push(json!(["voice", id])),
                        StealthCue::Tint => calls.push(json!(["tint", 0x90, 0, 0, 1])),
                        StealthCue::FinishContacts => calls.push(json!(["contacts", 1])),
                        _ => panic!("Unexpected update cue {cue:?}"),
                    }
                    Ok(())
                },
            )
            .unwrap();
        let actual = json!({"actor":state(&player),"action":request.action,"mode":match request.velocity {Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1},"calls":calls});
        assert_eq!(actual, row["expected"], "update {id}: {row}");
        interrupted += usize::from(player.target.is_none());
        completed += usize::from(player.state != State::Attack);
    }
    assert!(starts.len() >= 5000 && interrupted > 1000 && completed > 1000);
    assert_eq!(updates.len(), 12_600);
    eprintln!(
        "{} paired starts, 12600 updates; {interrupted} interrupted, {completed} finished",
        starts.len()
    );
}
