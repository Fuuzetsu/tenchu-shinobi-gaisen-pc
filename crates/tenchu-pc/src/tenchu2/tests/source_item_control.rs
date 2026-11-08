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
        control::{GrappleStatus, ItemControlInput, ItemCue, Request},
        inventory::Item,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ITEM_CONTROL_ORACLE from original controllers"]
fn native_item_aiming_releases_medicine_and_grapple_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_ITEM_CONTROL_ORACLE").collect();
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
    let mut uses = 0;
    let mut medicines = 0;
    let mut launches = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::Item;
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
        actor.inventory.active = Item::try_from(int("active") as u8).unwrap();
        actor.inventory.counts[actor.inventory.active.index()] = int("count") as u8;
        let mut input = ItemControlInput {
            held: int("input") as u16,
            player: row["player"].as_bool().unwrap(),
            camera_actor: row["camera_actor"].as_bool().unwrap(),
            camera: int("camera") as i16,
            previous_camera: int("old_camera") as i16,
            camera_yaw: int("pitch") as i16,
            aim: vector(&row["aim"]),
            target_position: Some(vector(&row["target"])),
            grapple: GrappleStatus::Flying,
            grapple_position: Position::default(),
            difficulty: int("difficulty") as i16,
            map: int("map") as i16,
            story_triggered: row["story"].as_bool().unwrap(),
        };
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut random = Random::new(int("seed") as u32);
        let mut calls = Vec::new();
        actor
            .update_item(
                &mut request,
                &mut input,
                &resources,
                &mut random,
                |actor, request, input, _random, cue| {
                    match cue {
                        ItemCue::Recover => {
                            request.action = actor.normal_motion(input.held, |actor| {
                                calls.push(json!(["root"]));
                                Ok(actor.position)
                            })?;
                            request.velocity = Velocity::Motion;
                        }
                        ItemCue::Voice(value) => calls.push(json!(["voice", value])),
                        ItemCue::Sound { id, positional } => {
                            calls.push(json!(["sound", id, positional]))
                        }
                        ItemCue::PublishRoot => calls.push(json!(["publish"])),
                        ItemCue::HeldModel(value) => calls.push(json!(["held", value])),
                        ItemCue::LaunchGrapple(aim) => {
                            calls.push(json!(["launch", aim.to_array()]));
                            launches += 1;
                        }
                        ItemCue::UpdateGrapple => {
                            calls.push(json!(["grapple"]));
                            input.grapple = match int("grapple") {
                                -1 => GrappleStatus::Failed,
                                0 => GrappleStatus::Flying,
                                1 => GrappleStatus::Attached,
                                _ => GrappleStatus::Released,
                            };
                        }
                        ItemCue::ReleaseGrapple => calls.push(json!(["release"])),
                        ItemCue::GrapplePosition => {
                            calls.push(json!(["point"]));
                            input.grapple_position = vector(&row["grapple_position"]);
                        }
                        ItemCue::Use { item, direction } => {
                            calls.push(json!([
                                "use",
                                u8::from(item),
                                direction.map(Position::to_array)
                            ]));
                            uses += 1;
                        }
                        ItemCue::Medicine(value) => {
                            calls.push(json!(["medicine", value]));
                            medicines += 1;
                        }
                        ItemCue::ShowHealth => calls.push(json!(["hp"])),
                    }
                    Ok(())
                },
            )
            .unwrap();
        let actual = json!({
            "action": request.action,
            "mode": match request.velocity { Velocity::Keep => 0, Velocity::Motion => 1, Velocity::Stop => -1 },
            "input": input.held,
            "count": actor.inventory.counts[actor.inventory.active.index()],
            "health": actor.health,
            "max_health": actor.max_health,
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
            "camera": input.camera,
            "old_camera": input.previous_camera,
            "pitch": input.camera_yaw,
            "aim": input.aim.to_array(),
            "story": input.story_triggered,
            "seed": random.seed,
            "random": random.calls,
            "calls": calls,
        });
        assert_eq!(actual, row["expected"], "case {id}: {row}");
    }
    eprintln!(
        "{} source item updates; {uses} uses, {medicines} medicines, {launches} grapple launches",
        rows.len()
    );
}
