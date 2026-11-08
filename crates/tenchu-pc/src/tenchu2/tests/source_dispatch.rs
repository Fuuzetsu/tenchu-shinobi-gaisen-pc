use super::{
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_core::{
    Result,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        control::{ControlCue, ControlDriver, ControlInput, Request},
        input::ATTACK,
        physics::{Depth, Water},
    },
};

#[derive(Default)]
struct Driver {
    calls: Vec<Value>,
}

fn mode(velocity: Velocity) -> i16 {
    match velocity {
        Velocity::Stop => -1,
        Velocity::Keep => 0,
        Velocity::Motion => 1,
    }
}

impl ControlDriver for Driver {
    fn motion(&mut self, _: &mut Actor, request: &Request, _: &Resources) -> Result<()> {
        self.calls
            .push(json!(["motion", request.action, mode(request.velocity)]));
        Ok(())
    }

    fn cue(
        &mut self,
        _: &mut Actor,
        request: &mut Request,
        _: &mut ControlInput,
        cue: ControlCue,
    ) -> Result<()> {
        self.calls.push(match cue {
            ControlCue::Damage => {
                request.action = 0xa01;
                request.velocity = Velocity::Motion;
                json!(["damage"])
            }
            ControlCue::Ledge => json!(["ledge"]),
            ControlCue::Recover => {
                request.action = 0x100;
                request.velocity = Velocity::Motion;
                json!(["recover"])
            }
            ControlCue::Splash(kind) => json!(["splash", kind]),
            ControlCue::Voice(id) => json!(["voice", id]),
            ControlCue::DrowningRumble(strength) => json!(["rumble", strength]),
            ControlCue::SelectItem { previous } => json!(["item", u8::from(previous)]),
            ControlCue::ItemSound => json!(["sound", 5]),
        });
        Ok(())
    }

    fn state(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
    ) -> Result<()> {
        self.calls.push(json!(["state", actor.state as u8]));
        if input.held & ATTACK != 0 {
            request.action = 0x900;
            request.velocity = Velocity::Motion;
        }
        Ok(())
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC and TENCHU2_DISPATCH_ORACLE from the outer controller"]
fn native_control_dispatch_water_entry_and_breath_match_source() {
    let resources = resources();
    let template = template(&resources);
    let mut transitions = 0;
    let mut drowning = 0;
    let mut checked = 0;
    for (id, row) in cases::<Value>("TENCHU2_DISPATCH_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.motion.action = int("action") as i16;
        actor.state = State::from_action(actor.motion.action).unwrap();
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.max_health = 100;
        actor.breath = int("breath") as u8;
        actor.height = int("height") as i16;
        actor.position = vector(&row["position"]);
        actor.rotation = vector(&row["rotation"]);
        actor.velocity = vector(&row["velocity"]);
        actor.terrain.height = int("ground");
        actor.pad.held = int("input") as u16;
        actor.pad.previous = int("previous") as u16;
        let mut input = ControlInput {
            held: 0,
            player: boolean("player"),
            camera_actor: boolean("camera_actor"),
            camera: int("camera") as i16,
            script: int("script") as u16,
            tick: int("tick"),
            water: Water {
                depth: match int("water") {
                    -1 => None,
                    0 => Some(Depth::Shallow),
                    1 => Some(Depth::Wading),
                    2 => Some(Depth::Swimming),
                    3 => Some(Depth::Submerged),
                    _ => panic!("Invalid source water depth"),
                },
                bottom: 1000,
                surface: -1000,
            },
            model_height: int("model_height") as i16,
        };
        let mut request = Request {
            action: -1,
            velocity: Velocity::Stop,
        };
        let mut driver = Driver::default();
        actor
            .update_controls(&mut request, &mut input, &resources, &mut driver)
            .unwrap();
        assert_eq!(
            json!({
                "action": request.action, "mode": mode(request.velocity), "input": input.held,
                "health": actor.health, "breath": actor.breath, "height": actor.height,
                "position": actor.position.to_array(), "rotation": actor.rotation.to_array(),
                "velocity": actor.velocity.to_array(), "frame": actor.motion.frame,
                "mask": actor.motion.mask, "camera": input.camera, "calls": driver.calls,
            }),
            row["expected"],
            "case {id}: {row}"
        );
        transitions += usize::from(request.action != actor.motion.action);
        drowning += usize::from(driver.calls.iter().any(|call| call[0] == "rumble"));
        checked += 1;
    }
    assert_eq!(checked, 24000);
    assert!(transitions > 5000 && drowning > 50);
    eprintln!("{checked} controller updates: {transitions} transitions, {drowning} drowning ticks");
}
