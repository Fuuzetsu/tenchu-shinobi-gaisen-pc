use super::{
    source_combat::visibility,
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{combat::Rules, level::Level};
use tenchu_core::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{Actor, State},
        ai::{
            self,
            bait::{Observation, Outcome},
            behavior::{Events, Frame},
            combat::Context,
            navigation::Surroundings,
            perception::Global,
            sight::{Body, Ranges, Sight, Visibility},
        },
        terrain::Cursor,
    },
};

struct Silent;
impl Events for Silent {
    fn voice(&mut self, _: &Actor, _: u8) -> Result<()> {
        Ok(())
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_BAIT_AI_ORACLE; original JP lure sight/approach decisions"]
fn native_bait_ai_matches_original_sight_and_approach() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = resources();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in cases::<Value>("TENCHU2_BAIT_AI_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let map = maps
            .entry(int("map") as usize)
            .or_insert_with(|| Level::load(&disc, int("map") as usize).unwrap().collision);
        let mut actor = template.clone();
        actor.position = vector(&row["position"]);
        actor.rotation.y = int("yaw") as i16;
        actor.height = int("height") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.turn = 32;
        actor.target = None;
        let body = Body {
            position: vector(&row["player"]),
            yaw: 0,
            height: int("player_height") as i16,
            state: State::from_action((int("player_state") as i16) << 8).unwrap(),
            action: int("player_action") as i16,
            in_water: false,
        };
        let mut sight = Sight {
            destination: Position::new(100, 200, 300),
            ..Default::default()
        };
        let steering = Surroundings::default().steering(&actor, 0, 0, false);
        let mut seen = Visibility::Recognized;
        let mut random = Random::new(1);
        let mut c = Context {
            actor: &mut actor,
            target: None,
            sight: &mut sight,
            visibility: &mut seen,
            steering,
            observed_flags: 0,
            player_held: 0,
            facing: 123,
            difficulty: 2,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let mut global = Global::default();
        global.alert.remaining = int("alert") as i16;
        let mut ranges = Ranges::new(int("distance"));
        let outcome = ai::bait::update(
            vector(&row["bait"]),
            &mut c,
            &global,
            &mut Frame {
                terrain: map,
                cursor: &mut Cursor::default(),
                tick: int("tick") as u32,
                script: int("script") as u16,
                controllers: [0; 2],
                rotation_phase: &mut 0,
                events: &mut Silent,
                observation: Observation {
                    player: 1,
                    body,
                    slot: int("slot") as i16,
                    range: int("range") as i16,
                    ranges: &mut ranges,
                },
            },
        )
        .unwrap();
        assert_eq!(
            *c.visibility,
            visibility(int("result")),
            "visibility {case}"
        );
        assert_eq!(c.facing, int("facing"), "facing {case}");
        assert_eq!(
            json!(c.sight.destination.to_array()),
            row["destination"],
            "destination {case}"
        );
        assert_eq!(
            json!([
                ranges.standing.notice,
                ranges.standing.recognize,
                ranges.standing.forget,
                ranges.crouched.notice,
                ranges.crouched.recognize,
                ranges.crouched.forget
            ]),
            row["ranges"],
            "ranges {case}"
        );
        assert_eq!(
            matches!(outcome, Outcome::ResumePatrol),
            row["resume"].as_bool().unwrap(),
            "restore {case}"
        );
        assert_eq!(
            matches!(outcome, Outcome::LookAround),
            row["look"].as_bool().unwrap(),
            "fallback {case}"
        );
        if let Outcome::Continue(decision) = outcome {
            let expected = if let Some(movement) = row["movement"].as_array() {
                ai::steer(
                    movement[0].as_i64().unwrap() as i32,
                    movement[1].as_i64().unwrap() as i32,
                    &steering,
                    &resources.math,
                )
                .unwrap()
            } else {
                ai::Decision::default()
            };
            assert_eq!(decision.held, expected.held, "movement {case}");
            assert_eq!(decision.forced, expected.forced, "forced {case}");
        }
        count += 1;
    }
    assert!(count >= 3000);
    eprintln!("Original bait AI cases: {count}");
}
