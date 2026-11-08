use serde_json::Value;
use tenchu_assets::tenchu2::campaign::{Campaign, Kind, Objective, Region, Sequence};
use tenchu_core::{
    math::Position,
    random::Random,
    tenchu2::{
        actor::State,
        mission::{Input, Mission, Outcome, Phase, Population, Vitals},
    },
};

fn outcome(value: i64) -> Option<Outcome> {
    match value {
        0 => None,
        1 => Some(Outcome::Arrived),
        2 => Some(Outcome::BossWeakened),
        3 => Some(Outcome::TargetsDefeated),
        4 => Some(Outcome::QuotaReached),
        5 => Some(Outcome::AreaCleared),
        -1 => Some(Outcome::Dead),
        -2 => Some(Outcome::Detected),
        _ => panic!("invalid source outcome"),
    }
}

#[test]
#[ignore = "requires TENCHU2_MISSION_ORACLE; original objective checks and complete end countdowns"]
fn native_mission_objectives_match_original_gameplay_controller() {
    let mut checked = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_MISSION_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap();
        let params: [i16; 4] = serde_json::from_value(row["params"].clone()).unwrap();
        let objective = match int("kind") {
            1 | 2 => Objective::Reach {
                region: Region {
                    center: params[..3].try_into().unwrap(),
                    radius: params[3],
                },
                unseen: int("kind") == 2,
            },
            3 => Objective::Weaken {
                targets: params[..3].try_into().unwrap(),
            },
            4 => Objective::Defeat {
                targets: params[..3].try_into().unwrap(),
            },
            5 => Objective::EliminateUnseen { count: params[0] },
            6 => Objective::EliminateAll,
            _ => unreachable!(),
        };
        let mut mission = Mission::new(
            Sequence {
                kind: Kind::Gameplay(objective),
                resource: 0,
                next: Some(1),
                failure: row["failure"].as_bool().unwrap().then_some(2),
            },
            Some(if row["results"].as_bool().unwrap() {
                Kind::Results
            } else {
                Kind::Event
            }),
            0,
        )
        .unwrap();
        let [bosses, guards, civilians]: [u8; 3] =
            serde_json::from_value(row["population"].clone()).unwrap();
        mission.population = Population {
            bosses,
            guards,
            civilians,
        };
        mission.phase = Phase::Playing {
            pending: outcome(int("pending")),
        };
        let targets: [Option<[i16; 2]>; 3] =
            serde_json::from_value(row["targets"].clone()).unwrap();
        let mut random = Random::new(int("seed") as u32);
        let mut fade = None;
        let mut finished_at = None;
        for (tick, expected) in std::iter::once((int("tick") as u32, &row["expected"])).chain(
            row["steps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|step| (step["tick"].as_u64().unwrap() as u32, &step["expected"])),
        ) {
            let update = mission.update(Input {
                tick,
                position: Position::from(
                    serde_json::from_value::<[i32; 3]>(row["position"].clone()).unwrap(),
                ),
                health: int("health") as i16,
                state: State::from_action((int("state") as i16) << 8).unwrap(),
                motion_loops: int("loops") as i16,
                killed: int("killed") as u8,
                detections: int("detected") as u8,
                targets: targets.map(|t| t.map(|[health, maximum]| Vitals { health, maximum })),
            });
            let note = (update.detection_sound && random.next15() % 3 == 0)
                .then(|| 36 - random.next15() % 4);
            assert_eq!(
                serde_json::json!(note),
                expected["note"],
                "sound {case}:{tick}"
            );
            assert_eq!(
                random.seed,
                expected["seed"].as_u64().unwrap() as u32,
                "RNG {case}:{tick}"
            );
            assert_eq!(
                update.face_spotter,
                expected["face"].as_bool().unwrap(),
                "spotter {case}:{tick}"
            );
            if update.began {
                finished_at = Some(tick);
            }
            let (result, remaining, finished) = match mission.phase {
                Phase::Playing { pending } => (pending, Some(0), false),
                Phase::Ending {
                    outcome,
                    remaining,
                    finished_at: actual,
                } => {
                    assert_eq!(Some(actual), finished_at, "completion tick {case}:{tick}");
                    (Some(outcome), Some(remaining), false)
                }
                Phase::Finished {
                    outcome,
                    finished_at: actual,
                } => {
                    assert_eq!(Some(actual), finished_at, "completion tick {case}:{tick}");
                    (Some(outcome), None, true)
                }
            };
            assert_eq!(
                result,
                outcome(expected["outcome"].as_i64().unwrap()),
                "outcome {case}:{tick}"
            );
            if let Some(remaining) = remaining {
                assert_eq!(
                    remaining,
                    expected["remaining"].as_u64().unwrap() as u16,
                    "countdown {case}:{tick}"
                );
            }
            assert_eq!(
                finished,
                expected["finished"].as_bool().unwrap(),
                "finish {case}:{tick}"
            );
            if let Some(value) = expected["fade"].as_u64() {
                fade = Some(value as u8);
            }
            assert_eq!(mission.fade, fade, "fade {case}:{tick}");
            checked += 1;
        }
    }
    assert!(checked > 1000);
    eprintln!("{checked} original JP objective/countdown updates");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_POPULATION_ORACLE; original objective targets and population classification"]
fn native_mission_targets_and_population_match_original() {
    let resources = super::source_control::resources();
    let template = super::source_control::template(&resources);
    let mut checked = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_POPULATION_ORACLE").enumerate()
    {
        let targets = serde_json::from_value(row["targets"].clone()).unwrap();
        let objective = match row["kind"].as_i64().unwrap() {
            3 => Objective::Weaken { targets },
            4 => Objective::Defeat { targets },
            _ => Objective::EliminateAll,
        };
        let mut mission = Mission::new(
            Sequence {
                kind: Kind::Gameplay(objective),
                resource: 0,
                next: None,
                failure: None,
            },
            None,
            0,
        )
        .unwrap();
        let [bosses, guards, civilians] =
            serde_json::from_value::<[u8; 3]>(row["before"].clone()).unwrap();
        mission.population = Population {
            bosses,
            guards,
            civilians,
        };
        let actors: Vec<_> = serde_json::from_value::<Vec<[i16; 2]>>(row["actors"].clone())
            .unwrap()
            .into_iter()
            .map(|[kind, health]| {
                let mut actor = template.clone();
                actor.kind = kind;
                actor.health = health;
                tenchu_core::tenchu2::ai::runtime::Agent::new(actor, 0, 0, &[])
            })
            .collect();
        mission.initialize(&actors, row["hero"].as_u64().unwrap() as usize);
        assert_eq!(
            serde_json::json!([
                mission.population.bosses,
                mission.population.guards,
                mission.population.civilians
            ]),
            row["population"],
            "population {case}"
        );
        assert_eq!(
            serde_json::json!(mission.targets),
            row["selected"],
            "targets {case}"
        );
        checked += 1;
    }
    assert!(checked > 100);
}

#[test]
#[ignore = "requires TENCHU2_DISC; validates gameplay layouts and all mission progression links"]
fn native_campaign_reaches_results_from_each_mission_start() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    for layout in &layouts {
        let entry = campaign.sequences[layout.sequence];
        assert!(matches!(entry.kind, Kind::Gameplay(_)));
        assert_eq!(usize::from(entry.resource), layout.map);
    }
    for start in campaign.starts.iter().flatten().flatten() {
        let mut index = *start;
        let mut seen = std::collections::BTreeSet::new();
        let mut gameplay = false;
        loop {
            assert!(seen.insert(index), "sequence loop from {start} at {index}");
            let entry = campaign.sequences[index];
            if matches!(entry.kind, Kind::Gameplay(_)) {
                assert!(layouts.iter().any(|l| l.sequence == index));
                gameplay = true;
            }
            if entry.kind == Kind::Results {
                assert!(gameplay);
                break;
            }
            index = entry
                .next
                .unwrap_or_else(|| panic!("mission {start} ends without results at {index}"));
        }
    }
}
