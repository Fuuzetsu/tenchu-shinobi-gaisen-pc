use super::{
    source_combat::visibility,
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::combat::Rules;
use tenchu_core::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::State,
        ai::{
            Decision,
            combat::Context,
            navigation::{Ground, Surroundings},
            perception::{self, Global, Reactions, Routine, Scene},
            sight::{Sight, Visibility},
            status::Alert,
        },
    },
};

struct Boundary<'a> {
    row: &'a Value,
    timers: [u8; 4],
    elapsed: i16,
    memory: Position,
    remembered: Visibility,
    forced: u32,
    calls: Vec<Value>,
}

impl Reactions for Boundary<'_> {
    fn decide(
        &mut self,
        routine: Routine,
        c: &mut Context<'_>,
        global: &mut Global,
    ) -> Result<Decision> {
        let name = match routine {
            Routine::Idle => "idle",
            Routine::Suspicious => "suspicious",
            Routine::Combat => "combat",
            Routine::Search => "search",
            Routine::Watch => "watch",
            Routine::Corpse => "corpse",
        };
        self.calls.push(json!(["decide", name]));
        c.observed_flags = self.row["next_flags"].as_u64().unwrap() as u32;
        *c.visibility = visibility(self.row["next_visibility"].as_i64().unwrap() as i32);
        self.forced = self.row["next_forced"].as_u64().unwrap() as u32;
        c.steering.forced = self.forced != 0;
        if let Some(alert) = self.row["next_alert"].as_i64() {
            global.alert.remaining = alert as i16;
        }
        Ok(Decision {
            held: self.row["decision"].as_u64().unwrap() as u16,
            ..Default::default()
        })
    }
    fn action(&mut self, _: &mut Context<'_>, action: i16) -> Result<bool> {
        self.calls.push(json!(["action", action]));
        Ok(self.row["changed"].as_bool().unwrap())
    }
    fn voice(&mut self, _: &mut Context<'_>, voice: u8) -> Result<()> {
        self.calls.push(json!(["voice", voice]));
        Ok(())
    }
    fn reset(&mut self, forget: bool) {
        self.timers.fill(0);
        self.elapsed = 0;
        if forget {
            self.memory = Position::default();
        }
    }
    fn remember(&mut self, position: Position) {
        self.memory = position;
    }
    fn observe(&mut self, visibility: Visibility) {
        self.remembered = visibility;
    }
}

fn raw_visibility(visible: Visibility) -> i16 {
    match visible {
        Visibility::Hidden => -2,
        Visibility::Unnoticed => -1,
        Visibility::Unchanged => 0,
        Visibility::Recognized => 1,
        Visibility::Suspicious => 2,
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_PERCEPTION_ORACLE from original AI transitions"]
fn native_guard_perception_transitions_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = resources();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut count = 0;
    let mut spots = 0;
    for (id, row) in cases::<Value>("TENCHU2_PERCEPTION_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.turn = int("turn") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.motion.action = int("action") as i16;
        actor.target = Some(1);
        actor.pad.held = int("held") as u16;
        actor.pad.previous = int("previous") as u16;
        actor.pad.time = int("padtime") as i16;
        actor.pad.history = serde_json::from_value(row["history"].clone()).unwrap();
        let mut target = template.clone();
        target.state = State::from_action((int("target_state") as i16) << 8).unwrap();
        target.motion.action = int("target_action") as i16;
        target.position = vector(&row["target_position"]);
        let heights: [i32; 4] = serde_json::from_value(row["heights"].clone()).unwrap();
        let materials: [u16; 4] = serde_json::from_value(row["materials"].clone()).unwrap();
        let probe = |i: usize| Ground {
            height: (heights[i] != i32::MIN).then_some(heights[i]),
            material: materials[i],
        };
        let surroundings = Surroundings {
            ahead: probe(0),
            behind: probe(1),
            left: probe(2),
            right: probe(3),
        };
        let steering = surroundings.steering(
            &actor,
            int("cached") as u32,
            int("angle") as i16,
            int("forced") != 0,
        );
        let mut random = Random::new(int("seed") as u32);
        let mut visible = visibility(int("visibility"));
        let mut sight = Sight {
            angle: int("angle") as i16,
            ..Default::default()
        };
        let mut context = Context {
            actor: &mut actor,
            target: &target,
            sight: &mut sight,
            visibility: &mut visible,
            steering,
            observed_flags: int("cached") as u32,
            player_held: 0,
            facing: 0,
            difficulty: int("difficulty") as usize,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let scene = Scene {
            map: int("map") as usize,
            sequence: int("sequence") as u16,
            script: int("script") as u16,
            player_distance: int("distance"),
            notice_distance: int("notice"),
            player_action: int("player_action") as i16,
            target_is_player: boolean("target_is_player"),
            recovering_confusion: boolean("recover"),
            surroundings: &surroundings,
        };
        let mut global = Global {
            alert: Alert {
                remaining: int("alert") as i16,
                position: None,
            },
            nearest_distance: int("nearest"),
        };
        let mut boundary = Boundary {
            row: &row,
            timers: serde_json::from_value(row["timers"].clone()).unwrap(),
            elapsed: int("elapsed") as i16,
            memory: vector(&row["memory"]),
            remembered: visibility(int("remembered")),
            forced: int("forced") as u32,
            calls: Vec::new(),
        };
        let outcome = perception::update(&mut context, &scene, &mut global, &mut boundary).unwrap();
        let actual = json!({
            "held": outcome.decision.held, "flags": context.observed_flags, "visibility": raw_visibility(*context.visibility),
            "alert": global.alert.remaining, "nearest": global.nearest_distance, "spotted": outcome.spotted,
            "timers": boundary.timers, "elapsed": boundary.elapsed, "memory": boundary.memory.to_array(),
            "remembered": raw_visibility(boundary.remembered),
            "forced": outcome.decision.forced.map_or(boundary.forced, |f| u32::from(f.held) << 16 | u32::from(f.remaining as u16)),
            "pad": [actor.pad.held, actor.pad.previous, actor.pad.time as u16], "history": actor.pad.history,
            "seed": random.seed, "random": random.calls, "calls": boundary.calls,
        });
        for (key, value) in row["expected"].as_object().unwrap() {
            assert_eq!(&actual[key], value, "row {id} {key}");
        }
        count += 1;
        spots += usize::from(outcome.spotted);
    }
    assert!(count >= 40000 && spots > 100);
    eprintln!("{count} perception transitions, {spots} spotted events");
}
