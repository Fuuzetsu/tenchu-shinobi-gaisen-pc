use serde_json::{Value, json};
use tenchu_assets::tenchu2::campaign::Campaign;
use tenchu_core::{
    random::Random,
    tenchu2::{
        mission::Population,
        progress::{Completion, Progress, Record, Summary},
        world::Statistics,
    },
};

fn records(progress: &Progress) -> Value {
    json!(
        progress
            .records
            .iter()
            .flatten()
            .map(|r| json!([r.rank, r.stealth_kills, r.score, r.time_tenths]))
            .collect::<Vec<_>>()
    )
}

fn stocks(progress: &Progress) -> Value {
    json!(
        progress
            .stock
            .iter()
            .map(|s| s.items.map(|v| v.unwrap_or(255)))
            .collect::<Vec<_>>()
    )
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_RESULTS_ORACLE and TENCHU2_PROGRESS_INITIAL; original JP scoring, rewards, records and boot defaults"]
fn native_results_match_original_scores_records_rewards_and_unlocks() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let initial: Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("TENCHU2_PROGRESS_INITIAL").unwrap()).unwrap(),
    )
    .unwrap();
    let defaults = Progress::default();
    assert_eq!(stocks(&defaults), initial["stocks"]);
    assert_eq!(records(&defaults), initial["records"]);
    let mut checked = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_RESULTS_ORACLE").enumerate() {
        let mut progress = Progress::default();
        for (hero, stock) in progress.stock.iter_mut().enumerate() {
            stock.items = serde_json::from_value::<[u8; 21]>(row["stocks"][hero].clone())
                .unwrap()
                .map(|v| (v != 255).then_some(v));
        }
        for (record, input) in progress
            .records
            .iter_mut()
            .flatten()
            .zip(row["records"].as_array().unwrap())
        {
            *record = Record {
                rank: input[0].as_u64().unwrap() as u8,
                stealth_kills: input[1].as_u64().unwrap() as u8,
                score: input[2].as_i64().unwrap() as i16,
                time_tenths: input[3].as_i64().unwrap() as i32,
            };
        }
        progress.tatsumaru_unlocked = row["flags"].as_u64().unwrap() & 2 != 0;
        progress.all_grand_master = row["flags"].as_u64().unwrap() & 4 != 0;
        let before_stock = stocks(&progress);
        let [bosses, guards, civilians] =
            serde_json::from_value::<[u8; 3]>(row["population"].clone()).unwrap();
        let [killed, stealth_kills, combat_kills, innocents] =
            serde_json::from_value::<[u8; 4]>(row["kills"].clone()).unwrap();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let hero = row["hero"].as_u64().unwrap() as usize;
        let mission = row["mission"].as_u64().unwrap() as usize;
        let result = progress
            .complete(
                Completion {
                    hero,
                    mission,
                    sequence: row["sequence"].as_u64().unwrap() as usize,
                    summary: Summary {
                        population: Population {
                            bosses,
                            guards,
                            civilians,
                        },
                        kills: Statistics {
                            killed,
                            stealth_kills,
                            combat_kills,
                            innocents,
                        },
                        detections: row["detections"].as_u64().unwrap() as u8,
                        effort: row["effort"].as_i64().unwrap() as i16,
                        time_tenths: row["time"].as_i64().unwrap() as i32,
                    },
                    remaining: serde_json::from_value(row["remaining"].clone()).unwrap(),
                },
                &campaign.rewards,
                &mut random,
            )
            .unwrap();
        let expected = &row["expected"];
        assert_eq!(
            json!(result.record.score),
            expected["score"],
            "score {case}"
        );
        assert_eq!(json!(result.record.rank), expected["rank"], "rank {case}");
        assert_eq!(
            json!(result.new_best),
            expected["new_best"],
            "new best {case}"
        );
        assert_eq!(result.best, progress.records[hero][mission]);
        assert_eq!(records(&progress), expected["records"], "records {case}");
        assert_eq!(stocks(&progress), expected["stocks"], "supplies {case}");
        assert_eq!(json!(random.seed), expected["seed"], "RNG {case}");
        assert_eq!(
            json!(
                u8::from(progress.tatsumaru_unlocked) * 2 + u8::from(progress.all_grand_master) * 4
            ),
            expected["flags"],
            "unlocks {case}"
        );
        let s = &result.summary;
        assert_eq!(
            json!([
                i64::from(s.population.bosses),
                i64::from(s.population.guards),
                i64::from(s.population.civilians),
                i64::from(s.kills.killed),
                i64::from(s.kills.stealth_kills),
                i64::from(s.kills.combat_kills),
                i64::from(s.detections),
                i64::from(s.kills.innocents),
                i64::from(s.effort),
                i64::from(s.time_tenths)
            ]),
            expected["summary"],
            "summary {case}"
        );
        if let Some(item) = result.unlocked_item {
            assert!(
                (0..3)
                    .any(|hero| before_stock[hero][item] == 255
                        && expected["stocks"][hero][item] == 1)
            );
        }
        checked += 1;
    }
    assert!(checked > 100);
    eprintln!("{checked} original JP results/reward cases and boot progress");
}
