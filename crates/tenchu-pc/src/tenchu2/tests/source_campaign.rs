use serde_json::Value;
use tenchu_assets::tenchu2::campaign::Campaign;
use tenchu_core::tenchu2::{
    campaign::{Carry, Destination, Run},
    inventory::Inventory,
    mission::{Outcome, Phase, Population},
    progress::Summary,
    world::Statistics,
};

fn summary(values: &[i64]) -> Summary {
    Summary {
        population: Population {
            bosses: values[2] as u8,
            guards: values[3] as u8,
            civilians: values[4] as u8,
        },
        kills: Statistics {
            killed: values[5] as u8,
            stealth_kills: values[6] as u8,
            combat_kills: values[7] as u8,
            innocents: values[9] as u8,
        },
        detections: values[8] as u8,
        effort: values[10] as i16,
        time_tenths: values[11] as i32,
    }
}

fn carry(bytes: &[u8]) -> Carry {
    Carry {
        drawn: bytes[0] & 1 != 0,
        poisoned: bytes[0] & 4 != 0,
        alternate: bytes[0] & 2 != 0,
        health: bytes[1],
        breath: bytes[32],
        inventory: bytes[2..23].try_into().unwrap(),
        bleeding: None,
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CAMPAIGN_ENTRY_ORACLE; original area-entry/reset instructions"]
fn native_campaign_reset_matches_original_area_entry() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut checked = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_CAMPAIGN_ENTRY_ORACLE").enumerate()
    {
        let mut run = Run::new(
            &campaign,
            row["hero"].as_u64().unwrap() as usize,
            row["sequence"].as_u64().unwrap() as usize,
            2,
            1,
        )
        .unwrap();
        run.summary = summary(&serde_json::from_value::<Vec<i64>>(row["summary"].clone()).unwrap());
        run.carry = carry(&serde_json::from_value::<Vec<u8>>(row["carry"].clone()).unwrap());
        let mut snapshot = serde_json::to_value(&run).unwrap();
        snapshot["previous_area"] = row["previous"].clone();
        run = serde_json::from_value(snapshot).unwrap();
        let needs_equipment = run.prepare_area(&campaign).unwrap();
        let expected = &row["expected"];
        let expected_carry =
            carry(&serde_json::from_value::<Vec<u8>>(expected["carry"].clone()).unwrap());
        assert_eq!(
            serde_json::to_value(&run.summary).unwrap(),
            serde_json::to_value(summary(
                &serde_json::from_value::<Vec<i64>>(expected["summary"].clone()).unwrap()
            ))
            .unwrap(),
            "summary case {case}, sequence {}",
            run.sequence
        );
        assert_eq!(
            serde_json::to_value(&run.carry).unwrap(),
            serde_json::to_value(&expected_carry).unwrap(),
            "carry case {case}, sequence {}",
            run.sequence
        );
        assert_eq!(needs_equipment, expected_carry.inventory[0] == 0);
        assert_eq!(
            serde_json::to_value(run).unwrap()["previous_area"],
            expected["previous"]
        );
        checked += 1;
    }
    assert!(checked > 300);
    eprintln!("{checked} original JP campaign area entries");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CAMPAIGN_EXIT_ORACLE; original health/inventory/time carry-over instructions"]
fn native_campaign_carry_matches_original_area_exit() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = crate::tenchu2::native::Session::load(&disc, 1, 1).unwrap();
    let campaign = &session.assets.campaign;
    let world = &mut session.world;
    let mut checked = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_CAMPAIGN_EXIT_ORACLE").enumerate()
    {
        let mut run = Run::new(campaign, 0, 1, 2, 1).unwrap();
        run.played_seconds = row["total"].as_i64().unwrap() as i32;
        run.summary.time_tenths = row["time"].as_i64().unwrap() as i32;
        run.carry.alternate = row["oldflags"].as_u64().unwrap() & 2 != 0;
        let actor = &mut world.actors[world.player].actor;
        actor.health = row["health"].as_i64().unwrap() as i16;
        actor.flags = row["flags"].as_u64().unwrap() as u32;
        actor.breath = row["breath"].as_u64().unwrap() as u8;
        actor.inventory =
            Inventory::from_amounts(serde_json::from_value(row["inventory"].clone()).unwrap());
        world.mission.phase = Phase::Finished {
            outcome: Outcome::Arrived,
            finished_at: row["tick"].as_i64().unwrap() as u32,
        };
        assert_eq!(
            run.finish_area(world, campaign).unwrap(),
            Destination::Sequence(2)
        );
        let actual = serde_json::json!({
            "time": run.summary.time_tenths, "total": run.played_seconds,
            "health": run.carry.health, "breath": run.carry.breath, "inventory": run.carry.inventory,
            "flags": 0x80 | u8::from(run.carry.drawn) | (u8::from(run.carry.alternate)<<1) | (u8::from(run.carry.poisoned)<<2),
        });
        assert_eq!(actual, row["expected"], "exit case {case}");
        checked += 1;
    }
    assert!(checked >= 600);
    eprintln!("{checked} original JP campaign exits");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_CAMPAIGN_ROUTE_ORACLE; original next/skip/retry/failure dispatch"]
fn native_campaign_routes_match_original_dispatcher() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = crate::tenchu2::native::Session::load(&disc, 1, 1).unwrap();
    let campaign = &session.assets.campaign;
    let mut checked = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_CAMPAIGN_ROUTE_ORACLE").enumerate()
    {
        let sequence = row["sequence"].as_u64().unwrap() as usize;
        let mut run = Run::new(
            campaign,
            row["hero"].as_u64().unwrap() as usize,
            sequence,
            2,
            1,
        )
        .unwrap();
        assert_eq!(run.mission, row["mission"].as_u64().unwrap() as usize);
        let destination = match row["mode"].as_str().unwrap() {
            "next" => run.advance(campaign, false),
            "skip" => run.advance(campaign, true),
            "retry" => run.retry(campaign),
            "failure" => {
                session.world.sequence = sequence as u16;
                session.world.mission.phase = Phase::Finished {
                    outcome: Outcome::Dead,
                    finished_at: 100,
                };
                run.finish_area(&session.world, campaign)
            }
            _ => unreachable!(),
        }
        .unwrap();
        let actual = match destination {
            Destination::Sequence(index) => serde_json::json!(["sequence", index]),
            Destination::Failed => serde_json::json!(["failed", null]),
            Destination::Complete => serde_json::json!(["complete", null]),
        };
        assert_eq!(actual, row["expected"], "route case {case}: {row}");
        checked += 1;
    }
    assert!(checked > 400);
    eprintln!("{checked} original JP campaign routes");
}
