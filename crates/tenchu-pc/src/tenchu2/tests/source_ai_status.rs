use super::{
    source_combat::visibility,
    source_control::{cases, resources, template},
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::combat::Rules;
use tenchu_core::{
    random::Random,
    tenchu2::{
        actor::State,
        ai::{
            sight::Visibility,
            status::{Alert, AlertCause, Ki, Victory},
        },
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STATUS_ORACLE from original alert/radar routines"]
fn native_alert_radar_and_victory_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = resources();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut count = 0;
    let mut seen = std::collections::BTreeSet::new();
    for (id, row) in cases::<Value>("TENCHU2_STATUS_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        let mut random = Random::new(int("seed") as u32);
        let mut alert = Alert {
            remaining: int("alert") as i16,
            position: row["valid"]
                .as_bool()
                .unwrap()
                .then_some(Default::default()),
        };
        let mode = row["mode"].as_str().unwrap();
        let actual = match mode {
            "victory" => {
                let mut victory = match int("phase") {
                    -1 => Victory::Begin,
                    0 => Victory::Waiting(int("remaining") as u8),
                    1 => Victory::Taunt,
                    2 => Victory::Sheathe,
                    3 => Victory::Done,
                    _ => panic!("Unknown victory phase"),
                };
                let mut actions = Vec::new();
                victory
                    .update(&mut actor, &mut random, |_, action| {
                        actions.push(action);
                        Ok(())
                    })
                    .unwrap();
                let (phase, remaining) = match victory {
                    Victory::Begin => (-1, 0),
                    Victory::Waiting(ticks) => (0, ticks),
                    Victory::Taunt => (1, 0),
                    Victory::Sheathe => (2, 0),
                    Victory::Done => (3, 0),
                };
                json!({"phase": phase, "remaining": remaining, "seed": random.seed, "random": random.calls, "actions": actions})
            }
            "radar" => {
                let value = match Ki::from_distance(int("distance")) {
                    Ki::Clear => -32768,
                    Ki::Spotted => -1,
                    Ki::Searching => -2,
                    Ki::Suspicious => 0,
                    Ki::Nearby(value) => i32::from(value),
                };
                json!({"value": value})
            }
            "raise" => {
                alert
                    .raise(
                        match int("cause") {
                            1 => AlertCause::Suspicion,
                            2 => AlertCause::Corpse,
                            _ => AlertCause::Combat,
                        },
                        int("difficulty") as u8,
                        &rules,
                    )
                    .unwrap();
                json!({"alert": alert.remaining})
            }
            "reset" => {
                alert.reset(int("difficulty") as u8, &rules).unwrap();
                json!({"alert": alert.remaining})
            }
            "advance" => {
                alert.advance();
                json!({"alert": alert.remaining, "valid": alert.position.is_some()})
            }
            "visibility" => {
                let observed = alert.visibility(
                    &actor,
                    int("cached") as u32,
                    int("script") as u16,
                    visibility(int("visibility")),
                );
                json!({"visibility": match observed {Visibility::Hidden => -2, Visibility::Unnoticed => -1, Visibility::Unchanged => 0, Visibility::Recognized => 1, Visibility::Suspicious => 2}})
            }
            _ => panic!("Unknown alert operation"),
        };
        assert_eq!(actual, row["expected"], "row {id} {mode}");
        seen.insert(mode.to_owned());
        count += 1;
    }
    assert!(count >= 60000 && seen.len() == 6);
}
