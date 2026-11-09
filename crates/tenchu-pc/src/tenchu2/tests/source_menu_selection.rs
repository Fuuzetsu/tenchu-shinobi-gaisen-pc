use serde::Deserialize;
use tenchu_assets::tenchu2::selection::Rules;
use tenchu_core::tenchu2::{
    progress::Progress,
    selection::{Outcome, Selection},
};

#[derive(Deserialize)]
struct Case {
    unlocked: bool,
    records: Vec<i32>,
    inputs: Vec<u16>,
    states: Vec<Expected>,
}

#[derive(Deserialize)]
struct Expected {
    hero: usize,
    displayed: usize,
    mission: usize,
    page: usize,
    phase: u32,
    yaw: [i16; 3],
    fade: i16,
    cancel: bool,
    finished: bool,
    unlocked: bool,
    records: Vec<i32>,
    sounds: Vec<u8>,
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC and TENCHU2_MENU_SELECTION_ORACLE; original navigation, hero transitions, cheats, locking and exit timing"]
fn mission_selection_matches_original_input_traces() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let rules = Rules::load(&disc).unwrap();
    let mut count = 0;
    for (case_index, case) in
        super::source_control::cases::<Case>("TENCHU2_MENU_SELECTION_ORACLE").enumerate()
    {
        let mut progress = Progress {
            tatsumaru_unlocked: case.unlocked,
            ..Default::default()
        };
        for (record, &time) in progress.records.iter_mut().flatten().zip(&case.records) {
            record.time_tenths = time;
        }
        let mut state = Selection::new(progress, false);
        for (tick, expected) in case.states.iter().enumerate() {
            let sounds = state.step(case.inputs[tick], &rules);
            assert_eq!(
                sounds, expected.sounds,
                "sounds case {case_index} tick {tick}"
            );
            assert_eq!(
                (
                    state.hero,
                    state.displayed,
                    state.mission,
                    state.page,
                    state.phase,
                    state.yaw,
                    state.fade,
                    state.finished,
                    state.outcome == Some(Outcome::Cancel)
                ),
                (
                    expected.hero,
                    expected.displayed,
                    expected.mission,
                    expected.page,
                    expected.phase,
                    expected.yaw,
                    expected.fade,
                    expected.finished,
                    expected.cancel
                ),
                "state case {case_index} tick {tick}"
            );
            assert_eq!(state.progress.tatsumaru_unlocked, expected.unlocked);
            assert_eq!(
                state
                    .progress
                    .records
                    .iter()
                    .flatten()
                    .map(|r| r.time_tenths)
                    .collect::<Vec<_>>(),
                expected.records
            );
            if tick % 71 == 0 {
                state = serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
            }
            count += 1;
        }
    }
    assert!(count > 25_000);
    eprintln!("{count} original Japanese mission menu updates match");
}
