use super::{
    source_combat::visibility,
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
            Program,
            behavior::{Behavior, Events, Frame},
            combat::Context,
            navigation::Surroundings,
            perception::{Global, Scene},
            sight::{Sight, Visibility},
        },
        terrain::{Cursor, Surface},
    },
};

#[derive(Default)]
struct Effects {
    changed: bool,
    calls: Vec<Value>,
}
impl Events for Effects {
    fn action(&mut self, _: &mut Actor, action: i16, _: &Resources) -> Result<bool> {
        self.calls.push(json!(["action", action]));
        Ok(self.changed)
    }
    fn voice(&mut self, _: &Actor, voice: u8) -> Result<()> {
        self.calls.push(json!(["voice", voice]));
        Ok(())
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_BEHAVIOR_ORACLE from persistent original AI sequences"]
fn native_behavior_sequences_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut resources = resources();
    resources.weapons = resources.weapons.at_gameplay_rate();
    let rules = Rules::load(&disc).unwrap();
    let terrain = Level::load(&disc, 1).unwrap().collision;
    let template = template(&resources);
    let mut actor = template.clone();
    let mut target = template.clone();
    let mut behavior = Behavior::new(Default::default(), &[]);
    let mut random = Random::new(1);
    let mut global = Global::default();
    let mut sight = Sight::default();
    let mut surroundings = Surroundings::default();
    let mut cursor = Cursor::default();
    let mut phase = 0;
    let mut difficulty = 1;
    let mut count = 0;
    let mut runs = 0;
    for (id, row) in cases::<Value>("TENCHU2_BEHAVIOR_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        if !row["initial"].is_null() {
            let initial = &row["initial"];
            let init = |key: &str| initial[key].as_i64().unwrap() as i32;
            difficulty = init("difficulty") as usize;
            random = Random::new(init("seed") as u32);
            let home = vector(&initial["home"]);
            let route: Vec<_> = initial["route"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Waypoint {
                    x: p[0].as_i64().unwrap() as i32,
                    z: p[1].as_i64().unwrap() as i32,
                    action: p[2].as_i64().unwrap() as i32,
                    wait: p[3].as_i64().unwrap() as i32,
                })
                .collect();
            behavior = Behavior::new(home, &route);
            assert!(
                behavior
                    .assign(init("code") as u16, 1, difficulty as u8, &mut random)
                    .unwrap()
            );
            actor = template.clone();
            actor.kind = init("kind") as i16;
            actor.flags = 0x10 | if init("run") % 2 == 0 { 0 } else { 0x40 };
            actor.health = 100;
            actor.max_health = 100;
            actor.turn = init("turn") as i16;
            actor.inventory.counts = [5; 20];
            actor.inventory.selected = Some(tenchu_core::tenchu2::inventory::Item::GRAPPLE);
            actor.target = Some(1);
            global = Global::default();
            phase = 0;
            sight = Sight {
                destination: home,
                ..Default::default()
            };
            surroundings = Surroundings::default();
            cursor = Cursor::default();
            runs += 1;
        }
        random.calls = 0;
        actor.position = vector(&row["position"]);
        actor.rotation.y = int("yaw") as i16;
        actor.state = State::from_action(int("action") as i16).unwrap();
        actor.motion.action = int("action") as i16;
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.attack.index = Some(int("attack") as usize);
        target.flags = int("target_flags") as u32;
        target.state = State::from_action((int("target_state") as i16) << 8).unwrap();
        target.motion.action = int("target_action") as i16;
        target.position = vector(&row["target_position"]);
        if row["corpse_event"].as_bool().unwrap() {
            actor.flags = actor.flags & !3 | 0x81;
            behavior.reset(false);
            behavior.remember(target.position);
        }
        global.alert.advance();
        global.nearest_distance = i32::MAX;
        sight.angle = int("angle") as i16;
        sight.distance = int("distance");
        sight.player_angle = int("bearing") as i16;
        let mut seen = visibility(int("visibility"));
        surroundings
            .sample(
                &actor,
                int("step") as u32,
                1500,
                &terrain,
                &mut cursor,
                &resources.math,
            )
            .unwrap();
        let steering =
            surroundings.steering(&actor, actor.flags, sight.angle, behavior.forced.is_some());
        let cached_flags = actor.flags;
        let mut c = Context {
            actor: &mut actor,
            target: Some((&target).into()),
            sight: &mut sight,
            visibility: &mut seen,
            steering,
            observed_flags: cached_flags,
            player_held: int("player_held") as u16,
            facing: int("angle").abs(),
            difficulty,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let mut effects = Effects {
            changed: row["changed"].as_bool().unwrap(),
            ..Default::default()
        };
        let scene = Scene {
            map: 1,
            sequence: 4,
            script: int("script") as u16,
            player_distance: int("distance"),
            notice_distance: 10000,
            player_action: target.motion.action,
            target_is_player: true,
            recovering_confusion: behavior.recovering_confusion,
            surroundings: &surroundings,
        };
        let outcome = behavior
            .perceive(
                &mut c,
                &scene,
                &mut global,
                &mut Frame {
                    terrain: &terrain,
                    cursor: &mut cursor,
                    tick: int("step") as u32,
                    script: int("script") as u16,
                    controllers: [0; 2],
                    rotation_phase: &mut phase,
                    events: &mut effects,
                },
            )
            .unwrap();
        let vis = match *c.visibility {
            Visibility::Hidden => -2,
            Visibility::Unnoticed => -1,
            Visibility::Unchanged => 0,
            Visibility::Recognized => 1,
            Visibility::Suspicious => 2,
        };
        let surface = |s: Option<Surface>| s.map(|s| [s.cell, s.node]);
        let idle = match behavior.programs.idle {
            Program::LookAround => 3,
            Program::Wander => 4,
            Program::Stationary => 5,
            Program::Sleep => 6,
            Program::Patrol => 8,
            _ => panic!("Unexpected idle routine"),
        };
        let actual = json!({
            "held": outcome.decision.held, "flags": c.observed_flags, "visibility": vis, "alert": global.alert.remaining,
            "nearest": global.nearest_distance, "spotted": outcome.spotted,
            "forced": behavior.forced.map_or(0, |f| u32::from(f.held)<<16 | u32::from(f.remaining as u16)),
            "angle": c.sight.angle, "facing": c.facing, "yaw": c.actor.rotation.y,
            "destination": c.sight.destination.to_array(), "home": behavior.home.to_array(), "idle": idle,
            "phase": phase, "selected": c.actor.inventory.selected.map(u8::from),
            "pad": [c.actor.pad.held,c.actor.pad.previous,c.actor.pad.time as u16], "history": c.actor.pad.history,
            "seed": c.random.seed, "random": c.random.calls, "calls": effects.calls,
            "current": surface(cursor.current), "last": surface(cursor.last), "material": cursor.material, "index": cursor.index,
        });
        assert_eq!(actual, row["expected"], "case {id}: {row}");
        c.actor.flags = c.observed_flags;
        c.actor.pad.update(outcome.decision.held);
        count += 1;
    }
    eprintln!("{count} persistent AI steps in {runs} sequences");
    assert!(count >= 48000 && runs >= 80);
}
