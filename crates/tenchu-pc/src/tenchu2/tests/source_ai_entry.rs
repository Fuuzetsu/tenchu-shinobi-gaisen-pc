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
            combat::Context,
            entry::{self, Global, Outcome, Reactions, Scene, Target},
            navigation::Surroundings,
            perception,
            sight::{Sight, Visibility},
            status::{Alert, Ki},
        },
        inventory::Item,
    },
};

struct Boundary<'a> {
    row: &'a Value,
    timers: [u8; 4],
    elapsed: i16,
    memory: Position,
    forced: u32,
    calls: Vec<Value>,
}

impl Boundary<'_> {
    fn routine(&mut self, name: &str) -> u16 {
        self.calls.push(json!([name]));
        self.row["routine_held"].as_u64().unwrap() as u16
    }
}

impl Reactions for Boundary<'_> {
    fn nearest_target(&mut self, _: &mut Context<'_>, range: i32) -> Result<Option<Target>> {
        self.calls.push(json!(["nearest", range]));
        let index = self.row["nearest_target"].as_u64().unwrap() as usize;
        Ok((index != 0).then_some(Target {
            actor: index,
            kind: if index == 3 {
                self.row["candidate_kind"].as_i64().unwrap() as i16
            } else {
                0
            },
        }))
    }
    fn action(&mut self, _: &mut Context<'_>, action: i16) -> Result<()> {
        self.calls.push(json!(["action", action]));
        Ok(())
    }
    fn idle(&mut self, _: &mut Context<'_>, _: &mut Global) -> Result<u16> {
        Ok(self.routine("idle"))
    }
    fn victory(&mut self, _: &mut Context<'_>) -> Result<u16> {
        Ok(self.routine("victory"))
    }
    fn corpses(&mut self, _: &mut Context<'_>, _: &mut Global) -> Result<()> {
        self.calls.push(json!(["corpses"]));
        Ok(())
    }
    fn confusion(&mut self, _: &mut Context<'_>) -> Result<u16> {
        Ok(self.routine("confusion"))
    }
    fn sight(&mut self, c: &mut Context<'_>) -> Result<()> {
        self.calls.push(json!(["sight"]));
        c.sight.angle = self.row["next_angle"].as_i64().unwrap() as i16;
        c.sight.distance = self.row["next_distance"].as_i64().unwrap() as i32;
        *c.visibility = visibility(self.row["next_visibility"].as_i64().unwrap() as i32);
        Ok(())
    }
    fn sample_player_terrain(&mut self, _: &mut Context<'_>) -> Result<()> {
        self.calls.push(json!(["sample", 1000]));
        Ok(())
    }
    fn reset(&mut self, forget: bool) {
        self.timers.fill(0);
        self.elapsed = 0;
        if forget {
            self.memory = Position::default();
        }
    }
    fn clear_forced(&mut self) {
        self.forced = 0;
    }
    fn recover_confusion(&mut self) {
        self.timers[2] = 50;
    }
}

fn ki(raw: i32) -> Ki {
    match raw {
        -32768 => Ki::Clear,
        -1 => Ki::Spotted,
        -2 => Ki::Searching,
        0 => Ki::Suspicious,
        _ => Ki::Nearby(raw as u8),
    }
}

fn raw_ki(ki: Ki) -> i32 {
    match ki {
        Ki::Clear => -32768,
        Ki::Spotted => -1,
        Ki::Searching => -2,
        Ki::Suspicious => 0,
        Ki::Nearby(value) => i32::from(value),
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ENTRY_ORACLE from the enclosing AI entry"]
fn native_ai_entry_matches_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = resources();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut count = 0;
    let mut dispatched = 0;
    for (id, row) in cases::<Value>("TENCHU2_ENTRY_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.max_health = int("max_health") as i16;
        actor.position = vector(&row["position"]);
        actor.motion.action = int("action") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.target = (int("target") != 0).then_some(int("target") as usize);
        actor.inventory.counts[2] = int("stock") as u8;
        actor.inventory.selected = Some(Item::try_from(int("selected") as u8).unwrap());
        actor.pad.held = int("held") as u16;
        actor.pad.previous = int("previous") as u16;
        actor.pad.time = int("padtime") as i16;
        actor.pad.history = serde_json::from_value(row["history"].clone()).unwrap();
        let mut random = Random::new(int("seed") as u32);
        let mut visible = visibility(int("visibility"));
        let mut sight = Sight {
            angle: int("angle") as i16,
            distance: int("distance"),
            ..Default::default()
        };
        let steering =
            Surroundings::default().steering(&actor, actor.flags, sight.angle, int("forced") != 0);
        let mut c = Context {
            actor: &mut actor,
            target: Some((&template).into()),
            sight: &mut sight,
            visibility: &mut visible,
            steering,
            observed_flags: 0,
            player_held: int("player_held") as u16,
            facing: 0,
            difficulty: 2,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let mut global = Global {
            perception: perception::Global {
                alert: Alert {
                    remaining: int("alert") as i16,
                    position: boolean("valid").then(|| vector(&row["shared"])),
                },
                nearest_distance: int("nearest"),
            },
            radar: ki(int("radar")),
            previous_radar: ki(int("previous_radar")),
            player_held: int("player_held") as u16,
            rotation_phase: int("phase") as i16,
        };
        let scene = Scene {
            actor: 1,
            player: if boolean("is_player") { 1 } else { 2 },
            player_health: int("player_health") as i16,
            player_position: vector(&row["player_position"]),
            tick: int("tick") as u32,
            slot: int("slot") as i16,
            map: int("map") as usize,
            has_suspicious_program: boolean("has_suspicious"),
        };
        let mut boundary = Boundary {
            row: &row,
            timers: serde_json::from_value(row["timers"].clone()).unwrap(),
            elapsed: int("elapsed") as i16,
            memory: vector(&row["memory"]),
            forced: int("forced") as u32,
            calls: Vec::new(),
        };
        let outcome = entry::update(&mut c, &scene, &mut global, &mut boundary).unwrap();
        let (perception, result) = match outcome {
            Outcome::Complete(held) => (false, i32::from(held)),
            Outcome::Perception { player_distance } => (true, player_distance),
        };
        let visible = match *c.visibility {
            Visibility::Hidden => -2,
            Visibility::Unnoticed => -1,
            Visibility::Unchanged => 0,
            Visibility::Recognized => 1,
            Visibility::Suspicious => 2,
        };
        let actual = json!({
            "perception": perception, "result": result, "flags": c.actor.flags, "cached": c.observed_flags,
            "target": c.actor.target.unwrap_or(0), "selected": u8::from(c.actor.inventory.selected.unwrap()),
            "alert": global.perception.alert.remaining,
            "shared": global.perception.alert.position.unwrap_or_else(|| vector(&row["shared"])).to_array(),
            "valid": global.perception.alert.position.is_some(), "radar": raw_ki(global.radar),
            "previous_radar": raw_ki(global.previous_radar), "nearest": global.perception.nearest_distance,
            "player_held": global.player_held, "phase": global.rotation_phase,
            "angle": c.sight.angle, "distance": c.sight.distance, "visibility": visible,
            "forced": boundary.forced, "timers": boundary.timers, "elapsed": boundary.elapsed,
            "memory": boundary.memory.to_array(), "pad": [c.actor.pad.held, c.actor.pad.previous, c.actor.pad.time as u16],
            "history": c.actor.pad.history, "seed": c.random.seed, "random": c.random.calls, "calls": boundary.calls,
        });
        assert_eq!(actual, row["expected"], "case {id}: {row}");
        count += 1;
        dispatched += usize::from(perception);
    }
    eprintln!("{count} AI entry updates, {dispatched} perception dispatches");
    assert!(count >= 50000);
}
