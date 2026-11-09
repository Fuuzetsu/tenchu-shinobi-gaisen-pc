use serde::Deserialize;
use tenchu_assets::tenchu2::equipment::{Images, Rules};
use tenchu_core::tenchu2::{
    equipment::{Equipment, Outcome, Slot, training},
    progress::Stock,
};

#[derive(Deserialize)]
struct Case {
    map: u16,
    double: bool,
    stock: [u8; 21],
    inputs: Vec<u16>,
    states: Vec<Expected>,
    result: Result,
}

#[derive(Deserialize)]
struct Expected {
    selected: usize,
    description: usize,
    remaining: u8,
    slots: Vec<Slot>,
    stock: [u8; 21],
    fade: i16,
    sounds: Vec<u8>,
}

#[derive(Deserialize)]
struct Result {
    inventory: [u8; 21],
    stock: [u8; 21],
    alternate: bool,
    cancel: bool,
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC and TENCHU2_EQUIPMENT_ORACLE; original equipment limits, input edges, supplies, cheats and exits"]
fn equipment_matches_original_input_traces() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let rules = Rules::load(&disc).unwrap();
    let images = Images::load(&disc).unwrap();
    assert_eq!(images.items.len(), 21);
    let mut total = 0;
    for (case_index, case) in
        super::source_control::cases::<Case>("TENCHU2_EQUIPMENT_ORACLE").enumerate()
    {
        if let Some(inventory) = training(case.map) {
            assert_eq!(inventory, case.result.inventory);
            assert_eq!(case.stock, case.result.stock);
            continue;
        }
        let mut state = Equipment::new(
            Stock {
                items: case.stock.map(|count| (count != 255).then_some(count)),
            },
            case.double,
        );
        for (tick, expected) in case.states.iter().enumerate() {
            let sounds = state.step(case.inputs[tick], &rules);
            assert_eq!(
                sounds, expected.sounds,
                "sounds case {case_index} tick {tick}"
            );
            assert_eq!(
                state.selected, expected.selected,
                "selection case {case_index} tick {tick}"
            );
            assert_eq!(
                state.description, expected.description,
                "description case {case_index} tick {tick}"
            );
            assert_eq!(
                state.remaining, expected.remaining,
                "capacity case {case_index} tick {tick}"
            );
            assert_eq!(
                state.slots, expected.slots,
                "slots case {case_index} tick {tick}"
            );
            assert_eq!(
                state.fade, expected.fade,
                "fade case {case_index} tick {tick}"
            );
            // The final fade is drawn before the original function commits/returns the slots.
            if !state.finished {
                assert_eq!(
                    state.stock.items.map(|n| n.unwrap_or(255)),
                    expected.stock,
                    "stock case {case_index} tick {tick}"
                );
            }
            if tick % 29 == 0 {
                state = serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
            }
            total += 1;
        }
        assert!(state.finished, "unfinished case {case_index}");
        assert_eq!(
            state.inventory(),
            case.result.inventory,
            "final inventory case {case_index}"
        );
        assert_eq!(
            state.stock.items.map(|n| n.unwrap_or(255)),
            case.result.stock,
            "final stock case {case_index}"
        );
        assert_eq!(state.alternate(), case.result.alternate);
        assert_eq!(state.outcome == Some(Outcome::Cancel), case.result.cancel);
    }
    eprintln!("{total} original JP equipment updates");
}
