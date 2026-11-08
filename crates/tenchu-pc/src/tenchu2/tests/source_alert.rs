use super::{
    source_combat::visibility,
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::combat::Rules;
use tenchu_core::{
    random::Random,
    tenchu2::{
        actor::State,
        ai::{
            Probe, Program, Sentry, SteeringInput,
            alert::{self, Confusion, ConfusionPhase, CorpseCue, CorpseInvestigation},
            combat::Context,
            sight::{Sight, Visibility},
        },
        inventory::Item,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ALERT_ORACLE from original alert decisions"]
fn native_alert_reactions_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut resources = resources();
    resources.weapons = resources.weapons.at_gameplay_rate();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let target = template.clone();
    let mut count = 0;
    let mut reports = 0;
    let mut items = 0;
    for (id, row) in cases::<Value>("TENCHU2_ALERT_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.max_health = int("max_health") as i16;
        actor.turn = int("turn") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.position = vector(&row["position"]);
        actor.motion.action = int("action") as i16;
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.attack.index = Some(int("attack") as usize);
        actor.pad.held = int("held") as u16;
        actor.inventory.selected = Some(Item::try_from(int("selected") as u8).unwrap());
        actor.inventory.counts = serde_json::from_value(row["stock"].clone()).unwrap();
        let mut sight = Sight {
            distance: int("distance"),
            angle: int("angle") as i16,
            destination: vector(&row["target_position"]),
            player_angle: int("bearing") as i16,
        };
        let mut visible = visibility(int("visibility"));
        let mut random = Random::new(int("seed") as u32);
        let mut context = Context {
            actor: &mut actor,
            target: Some((&target).into()),
            sight: &mut sight,
            visibility: &mut visible,
            steering: SteeringInput {
                yaw: int("yaw") as i16,
                turn: int("turn") as i16,
                target_angle: int("angle") as i16,
                alerted: int("cached") & 3 != 0,
                blocked: int("cached") & 0x48000 != 0,
                forced: int("forced") != 0,
                ahead: (int("height") != i32::MIN).then_some(Probe {
                    height: int("height"),
                    material: int("material") as u16,
                }),
                left: boolean("left"),
                right: boolean("right"),
            },
            observed_flags: int("cached") as u32,
            player_held: int("player_held") as u16,
            facing: int("facing"),
            difficulty: int("difficulty") as usize,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let mut delay = int("delay") as u8;
        let mut pause = int("return_delay") as u8;
        let mut phase = int("phase") as u8;
        let mut home = vector(&row["home"]);
        let mut elapsed = int("elapsed") as i16;
        let mut idle = Program::Disabled;
        let mut actions = Vec::new();
        let mut cues = Vec::new();
        let mode = row["mode"].as_str().unwrap();
        let decision = match mode {
            "watch" => {
                let mut sentry = if delay != 0 {
                    Sentry::Turning {
                        remaining: delay,
                        pause,
                    }
                } else if phase == 0 {
                    Sentry::Begin
                } else {
                    Sentry::Waiting { remaining: pause }
                };
                let shared = (!row["shared"].is_null()).then(|| vector(&row["shared"]));
                let result = alert::watch(
                    &mut sentry,
                    &mut context,
                    int("script") as u16,
                    shared,
                    |_, action| {
                        actions.push(action);
                        Ok(())
                    },
                )
                .unwrap();
                (delay, pause, phase) = match sentry {
                    Sentry::Begin => (0, 0, 0),
                    Sentry::Turning { remaining, pause } => (remaining, pause, 1),
                    Sentry::Waiting { remaining } => (0, remaining, 1),
                };
                result
            }
            "investigate" => {
                alert::investigate(&mut context, visibility(int("remembered"))).unwrap()
            }
            "flee" => alert::flee(&mut context).unwrap(),
            "confusion" => {
                let mut confusion = Confusion {
                    phase: match pause {
                        0 => ConfusionPhase::Right,
                        1 => ConfusionPhase::Retreat,
                        _ => ConfusionPhase::Left,
                    },
                    started: phase != 0,
                    remaining: delay,
                };
                let result = confusion.update(&mut context).unwrap();
                pause = match confusion.phase {
                    ConfusionPhase::Right => 0,
                    ConfusionPhase::Retreat => 1,
                    ConfusionPhase::Left => 2,
                };
                phase = u8::from(confusion.started);
                delay = confusion.remaining;
                result
            }
            "corpse" => {
                let mut search = CorpseInvestigation {
                    position: vector(&row["offset"]),
                    arrived: phase != 0,
                    elapsed,
                    turning: delay,
                };
                let result = search
                    .update(&mut context, &mut home, &mut idle, |_, _, cue| {
                        match cue {
                            CorpseCue::DrawWeapon => actions.push(0x700),
                            CorpseCue::SharedAlert => cues.push("alert"),
                            CorpseCue::Voice => cues.push("voice"),
                        }
                        Ok(())
                    })
                    .unwrap();
                phase = u8::from(search.arrived);
                elapsed = search.elapsed;
                delay = search.turning;
                result
            }
            _ => panic!("Unknown alert routine"),
        };
        let actual = json!({
            "held": decision.held, "selected": actor.inventory.selected.map(u8::from),
            "delay": delay, "return_delay": pause, "phase": phase, "home": home.to_array(), "elapsed": elapsed,
            "idle": match idle {Program::Disabled => 0, Program::LookAround => 3, Program::Wander => 4, _ => panic!("Unexpected idle program")},
            "forced": decision.forced.map_or(int("forced") as u32, |f| u32::from(f.held) << 16 | u32::from(f.remaining as u16)),
            "visibility": match visible {Visibility::Unchanged => 0, Visibility::Hidden => -2, Visibility::Unnoticed => -1, Visibility::Recognized => 1, Visibility::Suspicious => 2},
            "seed": random.seed, "random": random.calls, "actions": actions, "cues": cues,
        });
        for (key, value) in row["expected"].as_object().unwrap() {
            assert_eq!(&actual[key], value, "row {id}, mode {mode}: {key}");
        }
        count += 1;
        reports += usize::from(!cues.is_empty());
        items += usize::from(decision.held == tenchu_core::tenchu2::input::ITEM);
    }
    assert!(count >= 100000 && reports > 100 && items > 10);
    eprintln!("{count} alert decisions: {reports} corpse reports, {items} item uses");
}
