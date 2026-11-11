use super::{
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{combat::Rules, layout::Waypoint, level::Level};
use tenchu_core::{
    Result,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State},
        ai::{
            behavior::Events,
            runtime::{Agent, Frame, System},
            sight::Visibility,
            status::Ki,
        },
        collision::Collision,
        terrain::{Cursor, Surface},
    },
};

#[derive(Default)]
struct Effects {
    current: usize,
    changed: bool,
    calls: Vec<Value>,
}

impl Events for Effects {
    fn action(&mut self, _: &mut Actor, action: i16, _: &Resources) -> Result<bool> {
        self.calls.push(json!(["action", self.current, action]));
        Ok(self.changed)
    }
    fn voice(&mut self, actor: &Actor, voice: u8) -> Result<()> {
        self.calls.push(json!(["voice", actor.kind, voice]));
        Ok(())
    }
}

fn radar(ki: Ki) -> i32 {
    match ki {
        Ki::Clear => -32768,
        Ki::Spotted => -1,
        Ki::Searching => -2,
        Ki::Suspicious => 0,
        Ki::Nearby(value) => i32::from(value),
    }
}

fn state(agent: &Agent) -> Value {
    let actor = &agent.actor;
    let probes = [
        agent.surroundings.ahead,
        agent.surroundings.behind,
        agent.surroundings.left,
        agent.surroundings.right,
    ];
    json!({
        "flags": actor.flags, "target": actor.target, "position": actor.position.to_array(),
        "yaw": actor.rotation.y, "forced": agent.behavior.forced.map_or(0, |f| (u32::from(f.held) << 16) | u32::from(f.remaining as u16)),
        "destination": agent.sight.destination.to_array(), "home": agent.behavior.home.to_array(),
        "selected": actor.inventory.selected.map_or(0, |item| item.index()),
        "pad": [actor.pad.held, actor.pad.previous, actor.pad.time], "history": actor.pad.history,
        "distance": agent.sight.distance, "angle": agent.sight.angle, "bearing": agent.sight.player_angle,
        "heights": probes.map(|probe| probe.height.unwrap_or(i32::MIN)), "materials": probes.map(|probe| probe.material),
    })
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_RUNTIME_ORACLE from complete original AI updates"]
fn native_ai_runtime_matches_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut resources = resources();
    resources.weapons = resources.weapons.at_gameplay_rate();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut system = System::new(16000);
    let mut agents = Vec::new();
    let mut random = Random::new(1);
    let mut terrain = Level::load(&disc, 1).unwrap().collision;
    let mut map = 1;
    let mut difficulty = 1;
    let collision = Collision::default();
    let mut cursor = Cursor::default();
    let mut ticks = 0;
    let mut runs = 0;
    for (id, row) in cases::<Value>("TENCHU2_RUNTIME_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        if !row["initial"].is_null() {
            let initial = &row["initial"];
            random = Random::new(initial["seed"].as_u64().unwrap() as u32);
            map = initial["map"].as_u64().unwrap() as usize;
            difficulty = initial["difficulty"].as_u64().unwrap() as u8;
            terrain = Level::load(&disc, map).unwrap().collision;
            system = System::new(16000);
            cursor = Cursor::default();
            agents.clear();
            for (index, init) in initial["actors"].as_array().unwrap().iter().enumerate() {
                let mut actor = template.clone();
                actor.kind = init["kind"].as_i64().unwrap() as i16;
                actor.flags = if index == 0 { 0 } else { 0x10 };
                actor.position = vector(&init["home"]);
                actor.turn = init["turn"].as_i64().unwrap() as i16;
                actor.health = 100;
                actor.max_health = 100;
                actor.height = 1800;
                actor.target = (index != 0).then_some(0);
                actor.terrain.floor = Some(0);
                actor.inventory.counts = [5; 20];
                actor.inventory.selected = Some(tenchu_core::tenchu2::inventory::Item::GRAPPLE);
                let route: Vec<_> = init["route"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|point| Waypoint {
                        x: point[0].as_i64().unwrap() as i32,
                        z: point[1].as_i64().unwrap() as i32,
                        action: point[2].as_i64().unwrap() as i32,
                        wait: point[3].as_i64().unwrap() as i32,
                    })
                    .collect();
                let mut agent = Agent::new(actor, index as i16, 12000, &route);
                agent
                    .behavior
                    .assign(
                        init["code"].as_u64().unwrap() as u16,
                        map,
                        difficulty,
                        &mut random,
                    )
                    .unwrap();
                agents.push(agent);
            }
            runs += 1;
        }
        for (agent, input) in agents.iter_mut().zip(row["inputs"].as_array().unwrap()) {
            let actor = &mut agent.actor;
            actor.position = vector(&input["position"]);
            actor.motion.action = input["action"].as_i64().unwrap() as i16;
            actor.state = State::from_action(actor.motion.action).unwrap();
            actor.motion.frame = input["frame"].as_i64().unwrap() as i16;
            actor.motion.loops = input["loops"].as_i64().unwrap() as i16;
            actor.health = input["health"].as_i64().unwrap() as i16;
            actor.rotation.y = input["yaw"].as_i64().unwrap() as i16;
            actor.attack.index = Some(input["attack"].as_u64().unwrap() as usize);
        }
        random.calls = 0;
        let mut effects = Effects {
            changed: row["changed"].as_bool().unwrap(),
            ..Default::default()
        };
        let mut held = Vec::new();
        for index in 0..agents.len() {
            effects.current = index;
            held.push(
                system
                    .update(
                        index,
                        &mut agents,
                        &mut Frame {
                            player: 0,
                            map,
                            sequence: 4,
                            script: int("script") as u16,
                            tick: int("step") as u32,
                            difficulty,
                            controllers: std::array::from_fn(|i| {
                                row["controllers"][i].as_u64().unwrap() as u16
                            }),
                            resources: &resources,
                            rules: &rules,
                            terrain: &terrain,
                            cursor: &mut cursor,
                            collision: &collision,
                            animated_corpse: None,
                            aim_yaw: None,
                            random: &mut random,
                            events: &mut effects,
                        },
                    )
                    .unwrap_or_else(|error| panic!("row {id}, actor {index}: {error}")),
            );
        }
        let visibility = match system.visibility {
            Visibility::Hidden => -2,
            Visibility::Unnoticed => -1,
            Visibility::Unchanged => 0,
            Visibility::Recognized => 1,
            Visibility::Suspicious => 2,
        };
        let surface = |s: Option<Surface>| s.map(|s| [s.cell, s.node]);
        let actual = json!({
            "held": held, "actors": agents.iter().map(state).collect::<Vec<_>>(),
            "visibility": visibility, "facing": system.facing,
            "alert": system.global.perception.alert.remaining, "alert_position": system.global.perception.alert.position.map(|p| p.to_array()),
            "nearest": system.global.perception.nearest_distance, "radar": radar(system.global.radar), "previous_radar": radar(system.global.previous_radar),
            "player_held": system.global.player_held, "phase": system.global.rotation_phase,
            "corpse": system.corpses.observer, "detections": system.detections, "spotter": system.last_spotter,
            "ranges": [system.ranges.standing.notice, system.ranges.standing.recognize, system.ranges.standing.forget, system.ranges.crouched.notice, system.ranges.crouched.recognize, system.ranges.crouched.forget],
            "seed": random.seed, "random": random.calls, "calls": effects.calls,
            "current": surface(cursor.current), "last": surface(cursor.last), "material": cursor.material, "index": cursor.index,
        });
        if actual != row["expected"] {
            for (key, value) in actual.as_object().unwrap() {
                if value != &row["expected"][key] {
                    eprintln!("{key}: native {value} source {}", row["expected"][key]);
                }
            }
            panic!("row {id}, run {}, step {}", runs - 1, int("step"));
        }
        ticks += agents.len();
    }
    assert!(ticks >= 48000 && runs >= 40);
    eprintln!("{ticks} complete actor AI updates in {runs} sequences");
}
