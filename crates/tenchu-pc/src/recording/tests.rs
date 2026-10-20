use super::*;
use crate::session::parse_inputs;
use tenchu_core::{ai::Difficulty, input::*};

fn start() -> Start {
    Start {
        options: Options {
            character: 1,
            layout: 0,
            difficulty: Difficulty::Hard,
            language: Some(Language::Japanese),
            ..Options::default()
        },
        seed: 0x1234_5678,
        inventory: [
            255, 3, 0, 2, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 1,
        ],
    }
}

#[test]
fn recording_preserves_inputs_across_periodic_and_explicit_flushes() {
    let start = start();
    let inputs: Vec<_> = (0..97)
        .map(|tick| if tick < 61 { UP } else { LEFT | SQUARE })
        .collect();
    let mut bytes = Vec::new();
    let mut recording = Recorder::new(&mut bytes, start).unwrap();
    for (tick, &buttons) in inputs.iter().enumerate() {
        recording.push(buttons).unwrap();
        if tick == 36 {
            recording.flush().unwrap();
        }
    }
    recording.flush().unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let restored = Start::parse(&text).unwrap().unwrap();
    assert_eq!(restored.seed, start.seed);
    assert_eq!(restored.inventory, start.inventory);
    assert_eq!(restored.options.character, 1);
    assert_eq!(restored.options.layout, 0);
    assert_eq!(restored.options.difficulty, Difficulty::Hard);
    assert_eq!(restored.options.language, Some(Language::Japanese));
    let replayed: Vec<_> = parse_inputs(&text)
        .unwrap()
        .into_iter()
        .flat_map(|(count, buttons)| std::iter::repeat_n(buttons, count as usize))
        .collect();
    assert_eq!(replayed, inputs);
}

#[test]
fn malformed_recording_metadata_cannot_fall_back_to_default_game_state() {
    let mut bytes = Vec::new();
    start().write(&mut bytes).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    for invalid in [
        text.replace(HEADER, "# tenchu-recording 2"),
        text.replace("--mission 1", "--mission 11"),
        text.replace("--difficulty hard", "--layout 2"),
        text.replace("# seed 305419896", "# seed -1"),
        text.replace("# inventory 255", "# inventory 256"),
        text.lines().take(3).collect::<Vec<_>>().join("\n"),
    ] {
        assert!(Start::parse(&invalid).is_err(), "{invalid}");
    }
    assert!(
        Start::parse("# Old input tape\n30 1000\n")
            .unwrap()
            .is_none()
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; replays seeded, armoured and random-layout mission starts from recorded metadata"]
fn recorded_campaign_starts_reproduce_gameplay() {
    use crate::{
        campaign::{Action, Campaign, Screen},
        session::tests::state,
    };
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    for (mission, character, layout, language) in [
        (1, 0, 0, Language::Japanese),
        (2, 1, 2, Language::French),
        (0, 1, 0, Language::Italian),
    ] {
        let options = Options {
            mission,
            character,
            layout,
            language: Some(language),
            difficulty: Difficulty::Hard,
        };
        let mut campaign = Campaign::new(&assets, options, None).unwrap();
        campaign.seed = 0x3141_5926;
        campaign.progress.stock[character as usize][18] = 1;
        campaign.progress.stock[character as usize][19] = 1;
        campaign.action(Action::Confirm, &assets).unwrap();
        for item in [1, 18, 19] {
            while campaign.shop.order[campaign.cursor] != item {
                campaign.action(Action::Down, &assets).unwrap();
            }
            campaign.action(Action::Right, &assets).unwrap();
        }
        let Screen::Inventory(loadout) = &campaign.screen else {
            panic!("Expected equipment selection")
        };
        let start = Start {
            options: campaign.options,
            seed: campaign.seed,
            inventory: loadout.clone().finish().1,
        };
        campaign.action(Action::Confirm, &assets).unwrap();
        assert!(campaign.options.layout > 0);
        assert_eq!(start.options.layout, layout);
        assert_eq!(campaign.game.combat.armor, mission != 0);
        let mut bytes = Vec::new();
        let mut recording = Recorder::new(&mut bytes, start).unwrap();
        for tick in 0..240 {
            let buttons = match tick / 30 {
                1 | 2 => UP,
                3 => RIGHT,
                4 => CIRCLE | UP,
                5 => CROSS | UP,
                6 => SQUARE,
                _ => 0,
            };
            recording.push(buttons).unwrap();
            campaign.tick(buttons).unwrap();
            assert!(campaign.playing());
        }
        recording.flush().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let mut replayed = Start::parse(&text).unwrap().unwrap().load(&assets).unwrap();
        for (count, buttons) in parse_inputs(&text).unwrap() {
            for _ in 0..count {
                replayed.step(buttons).unwrap();
            }
        }
        assert_eq!(
            state(&campaign.game),
            state(&replayed),
            "mission {mission}, character {character}"
        );
    }
}
