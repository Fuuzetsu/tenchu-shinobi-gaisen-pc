use serde::Deserialize;
use tenchu_core::tenchu2::{
    ai::status::Ki,
    hud::{Health, Hud},
    inventory::{Inventory, Item},
};

#[derive(Deserialize)]
struct Case {
    counts: [u8; 21],
    kinds: Vec<i16>,
    states: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Command {
    Health(usize),
    Pickup(usize),
    Select(bool),
    Selected(Option<u8>),
    Count((usize, u8)),
}

#[derive(Deserialize)]
struct Step {
    commands: Vec<Command>,
    health: Vec<(i16, i16, bool)>,
    breath: u8,
    ki: i16,
    state: serde_json::Value,
    selected: u8,
    reticle: Option<(i16, bool)>,
}

fn health((current, maximum, poisoned): (i16, i16, bool)) -> Health {
    Health {
        current,
        maximum,
        poisoned,
    }
}

#[test]
#[ignore = "requires TENCHU2_HUD_ORACLE; JP executable health/Ki/breath, carousel, pickups and expiry"]
fn native_hud_matches_original_draws_and_timing() {
    let mut checked = 0;
    for (index, case) in super::source_control::cases::<Case>("TENCHU2_HUD_ORACLE").enumerate() {
        let mut hud = Hud::default();
        let mut inventory = Inventory::from_amounts(case.counts);
        for (tick, expected) in case.states.into_iter().enumerate() {
            for command in expected.commands {
                match command {
                    Command::Health(actor) => hud.show_health(actor, case.kinds[actor]),
                    Command::Pickup(item) => hud.pickup(item),
                    Command::Select(previous) => hud.select(&mut inventory, previous),
                    Command::Selected(item) => {
                        inventory.selected = item.map(|i| Item::try_from(i).unwrap())
                    }
                    Command::Count((item, count)) => {
                        if item == 20 {
                            inventory.resurrections = count;
                        } else {
                            inventory.counts[item] = count;
                        }
                    }
                }
            }
            let ki = match expected.ki {
                i16::MIN => Ki::Clear,
                -2 => Ki::Searching,
                -1 => Ki::Suspicious,
                0 => Ki::Spotted,
                value => Ki::Nearby(value.try_into().unwrap()),
            };
            hud.step(
                health(expected.health[0]),
                expected.breath,
                &inventory,
                ki,
                |actor| expected.health.get(actor).copied().map(health),
            )
            .unwrap();
            hud.aim(
                expected
                    .reticle
                    .map(|(scale, ready)| tenchu_core::tenchu2::items::aim::Reticle {
                        scale,
                        ready,
                    }),
            );
            assert_eq!(
                inventory.selected.map(u8::from).unwrap_or(255),
                expected.selected,
                "selection case {index} tick {tick}"
            );
            let mut actual = serde_json::to_value(&hud).unwrap();
            let mut state = expected.state;
            let frame = actual.as_object_mut().unwrap().remove("frame").unwrap();
            let expected_frame = state.as_object_mut().unwrap().remove("frame").unwrap();
            assert_eq!(
                frame.as_array().unwrap().len(),
                expected_frame.as_array().unwrap().len(),
                "draw count case {index} tick {tick}"
            );
            for (draw, (actual, expected)) in frame
                .as_array()
                .unwrap()
                .iter()
                .zip(expected_frame.as_array().unwrap())
                .enumerate()
            {
                assert_eq!(actual, expected, "HUD draw {draw} case {index} tick {tick}");
            }
            assert_eq!(actual, state, "HUD state case {index} tick {tick}");
            if tick % 37 == 0 {
                hud = serde_json::from_slice(&serde_json::to_vec(&hud).unwrap()).unwrap();
            }
            checked += 1;
        }
    }
    eprintln!("{checked} original JP HUD updates");
}
