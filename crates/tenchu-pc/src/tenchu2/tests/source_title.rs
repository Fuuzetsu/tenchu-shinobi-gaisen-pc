use serde::Deserialize;
use tenchu_core::tenchu2::title::Title;

#[derive(Deserialize)]
struct Case {
    inputs: Vec<u16>,
    states: Vec<Expected>,
}

#[derive(Deserialize)]
struct Expected {
    fade: i16,
    active: bool,
    selection: i16,
    idle: u32,
    color: [u8; 3],
    finished: bool,
    sounds: Vec<u8>,
}

#[test]
#[ignore = "requires TENCHU2_TITLE_ORACLE; original title instructions, input edges, fades and attract timeout"]
fn title_matches_original_input_traces() {
    let mut count = 0;
    for (case_index, case) in
        super::source_control::cases::<Case>("TENCHU2_TITLE_ORACLE").enumerate()
    {
        let mut title = Title::default();
        for (tick, (buttons, expected)) in case.inputs.into_iter().zip(case.states).enumerate() {
            let sounds: Vec<_> = title.step(buttons).into_iter().collect();
            assert_eq!(
                (
                    title.fade,
                    title.active,
                    title.attract,
                    title.idle,
                    title.color,
                    title.finished,
                    sounds
                ),
                (
                    expected.fade,
                    expected.active,
                    expected.selection == -1,
                    expected.idle,
                    expected.color,
                    expected.finished,
                    expected.sounds
                ),
                "case {case_index} tick {tick}"
            );
            if !title.attract {
                assert_eq!(title.choice.index(), expected.selection as usize);
            }
            count += 1;
        }
    }
    assert!(count > 50_000);
    eprintln!("{count} original Japanese title updates match");
}
