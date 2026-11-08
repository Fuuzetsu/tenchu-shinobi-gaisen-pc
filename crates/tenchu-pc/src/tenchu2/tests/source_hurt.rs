use super::{
    source_control::{cases, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::{Math, Position},
    random::Random,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{HurtCue, HurtInput, Request},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_HURT_ORACLE from original controllers"]
fn native_hit_reactions_knockdowns_and_recovery_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_HURT_ORACLE").collect();
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
    let template = template(&resources);
    let mut blood = 0;
    let mut deaths = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::Hurt;
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.motion.pose.rotation[0] = [10, 111, 20];
        actor.motion.pose.translation[0] = Position::new(111, 222, 333);
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.max_health = int("max_health") as i16;
        actor.height = int("height") as i16;
        actor.position = vector(&row["position"]);
        actor.rotation = vector(&row["rotation"]);
        actor.velocity = vector(&row["velocity"]);
        actor.turn = int("turn") as i16;
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        let input = HurtInput {
            held: int("input") as u16,
            player: row["player"].as_bool().unwrap(),
            difficulty: int("difficulty") as i16,
            map: int("map") as i16,
            water_surface: int("surface"),
        };
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut random = Random::new(int("seed") as u32);
        let mut calls = Vec::new();
        actor
            .update_hurt(
                &mut request,
                &input,
                &resources,
                &mut random,
                |actor, request, random, cue| {
                    match cue {
                        HurtCue::Recover => {
                            request.action = actor.normal_motion(input.held, |actor| {
                                calls.push(json!(["root"]));
                                Ok(actor.position)
                            })?;
                            request.velocity = Velocity::Motion;
                        }
                        HurtCue::HideHeldItem => calls.push(json!(["hide"])),
                        HurtCue::Voice(value) => calls.push(json!(["voice", value])),
                        HurtCue::Bubbles => calls.push(json!(["bubbles", 2])),
                        HurtCue::Blood { joint, angle } => {
                            calls.push(json!(["point", joint]));
                            let angle = angle.unwrap_or_else(|| random.next15() as i16);
                            calls.push(json!(["blood", row["point"], angle]));
                            blood += 1;
                        }
                        HurtCue::WaterRipple { position, size } => {
                            calls.push(json!(["ripple", position.to_array(), size]))
                        }
                        HurtCue::Splash(value) => calls.push(json!(["splash", value])),
                        HurtCue::LandingRumble => calls.push(json!(["rumble", 2, 1])),
                        HurtCue::Alert => calls.push(json!(["alert"])),
                        HurtCue::Dust => calls.push(json!(["dust", 0])),
                        HurtCue::RemoveWeapons => calls.push(json!(["weapon", 3])),
                        HurtCue::AlignToFloor => calls.push(json!(["align"])),
                        HurtCue::Dead => {
                            calls.push(json!(["dead"]));
                            deaths += 1;
                        }
                    }
                    Ok(())
                },
            )
            .unwrap();
        let actual = json!({
            "action": request.action,
            "mode": match request.velocity { Velocity::Keep => 0, Velocity::Motion => 1, Velocity::Stop => -1 },
            "health": actor.health,
            "flags": actor.flags,
            "position": actor.position.to_array(),
            "velocity": actor.velocity.to_array(),
            "rotation": actor.rotation.to_array(),
            "root_rotation": actor.motion.pose.rotation[0],
            "root_position": actor.motion.pose.translation[0].to_array(),
            "mask": actor.motion.mask,
            "motion": actor.motion.action,
            "clip": actor.motion.clip,
            "frame": actor.motion.frame,
            "loops": actor.motion.loops,
            "state": actor.state as u8,
            "seed": random.seed,
            "random": random.calls,
            "calls": calls,
        });
        assert_eq!(actual, row["expected"], "case {id}: {row}");
    }
    assert!(rows.len() >= 50_000 && blood > 100 && deaths > 100);
    eprintln!(
        "{} hurt source checks; {blood} blood events, {deaths} deaths",
        rows.len()
    );
}
