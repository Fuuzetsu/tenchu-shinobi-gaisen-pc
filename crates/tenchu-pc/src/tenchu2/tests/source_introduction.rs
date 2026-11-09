use tenchu_core::tenchu2::introduction::{Introduction, Outcome, Phase};

#[test]
#[ignore = "requires TENCHU2_INTRO_ORACLE; original chapter-title, narration scrolling, fades and skip timing"]
fn native_introductions_match_original_draw_sequence() {
    let mut checked = 0;
    for (index, case) in
        super::source_control::cases::<serde_json::Value>("TENCHU2_INTRO_ORACLE").enumerate()
    {
        let mut state = Introduction::new(case["resource"] != 9);
        for expected in case["states"].as_array().unwrap() {
            let phase = expected["phase"].as_u64().unwrap();
            let tick = expected["tick"].as_u64().unwrap();
            let start = case["press_tick"].as_u64().unwrap();
            let buttons = if phase == case["press_phase"].as_u64().unwrap()
                && (start..start + case["hold"].as_u64().unwrap()).contains(&tick)
            {
                case["button"].as_u64().unwrap() as u16
            } else {
                0
            };
            assert_eq!(state.vblanks(), if phase == 0 { 1 } else { 4 });
            state.step(
                buttons,
                phase == 1 && tick >= case["audio_end"].as_u64().unwrap(),
            );
            assert_eq!(state.frame.title, phase == 0, "case {index} tick {tick}");
            assert_eq!(
                i64::from(state.frame.fade),
                expected["fade"].as_i64().unwrap_or(0),
                "case {index} tick {tick}"
            );
            if phase == 1 {
                let columns: Vec<_> = expected["sprites"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| {
                        (
                            15 - s["uv"][0].as_u64().unwrap() as usize / 16,
                            s["position"][0].as_i64().unwrap() as i16,
                            s["color"][0].as_u64().unwrap() as u8,
                        )
                    })
                    .collect();
                assert_eq!(state.frame.columns, columns, "case {index} tick {tick}");
            }
            if tick % 83 == 0 {
                state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
            }
            checked += 1;
        }
        assert_eq!(state.phase, Phase::Finished, "case {index}");
        assert_eq!(
            state.outcome,
            if case["result"] == -3 {
                Outcome::Quit
            } else {
                Outcome::Continue
            }
        );
    }
    eprintln!("{checked} original JP introduction draws");
}

#[test]
#[ignore = "requires TENCHU2_DISC; all Japanese chapter artwork/narration and campaign movie streams"]
fn native_introduction_and_movie_resources_load_from_disc() {
    use tenchu_assets::{
        audio::stream::Clip,
        tenchu2::{
            campaign::{Campaign, Kind},
            frontend::{Introduction, Movie},
        },
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut resources = std::collections::BTreeSet::new();
    for sequence in &campaign.sequences {
        if sequence.kind == Kind::Introduction {
            resources.insert(sequence.resource);
        }
    }
    // Unused zero rows in the sequence table are not actual introduction screens.
    resources.remove(&0);
    for resource in resources {
        let intro = Introduction::load(&disc, resource).unwrap();
        if let Some(narration) = intro.narration {
            let clip = Clip::load(&disc, &narration.audio).unwrap();
            assert!(!clip.frames.is_empty());
            eprintln!(
                "JP introduction {resource}: {:?} background, {:?} text, {:.2}s narration",
                narration.background.size,
                narration.text.size,
                clip.frames.len() as f64 / f64::from(clip.rate)
            );
        }
    }
    let movies: std::collections::BTreeSet<_> = campaign
        .sequences
        .iter()
        .filter(|s| s.kind == Kind::Movie)
        .map(|s| s.resource)
        .collect();
    for resource in movies {
        let movie = Movie::load(&disc, resource).unwrap().unwrap();
        let audio = Clip::load(&disc, &movie.audio).unwrap();
        assert!(!audio.frames.is_empty());
        let mut stream = tenchu_assets::movie::Stream::open(&disc, &movie.audio.path).unwrap();
        let first = stream.next(&disc).unwrap().unwrap();
        assert_eq!(first.size, [320, 240]);
        assert!(first.number <= 1);
        eprintln!(
            "JP movie {resource}: {} frames, {:.2}s audio, first {:?}",
            movie.last_frame,
            audio.frames.len() as f64 / f64::from(audio.rate),
            first.size
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_PROLOGUE_ORACLE; original MENU.EXE pre-training narration, fade, text sheets and skip timing"]
fn native_prologue_matches_original_draw_sequence() {
    use tenchu_core::tenchu2::introduction::Prologue;
    let mut checked = 0;
    for (case_index, case) in
        super::source_control::cases::<serde_json::Value>("TENCHU2_PROLOGUE_ORACLE").enumerate()
    {
        let mut state = Prologue::default();
        let frames = case["states"].as_array().unwrap();
        let audio_end = case["audio_end"].as_u64().unwrap() as usize;
        for (tick, expected) in frames.iter().enumerate() {
            state.step(
                case["inputs"][tick].as_u64().unwrap() as u16,
                tick >= audio_end,
            );
            assert!(!state.finished, "case {case_index} tick {tick}");
            assert_eq!(
                i64::from(state.frame.fade),
                expected["fade"].as_i64().unwrap(),
                "case {case_index} tick {tick}"
            );
            let columns: Vec<_> = expected["columns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    let index = row[0].as_u64().unwrap() as usize;
                    assert_eq!(
                        row[3].as_u64().unwrap() as usize,
                        if index < 16 { 15 - index } else { 19 - index } * 16
                    );
                    (
                        index,
                        row[1].as_i64().unwrap() as i16,
                        row[2].as_u64().unwrap() as u8,
                    )
                })
                .collect();
            assert_eq!(
                state.frame.columns, columns,
                "case {case_index} tick {tick}"
            );
            if tick % 101 == 0 {
                state = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
            }
            checked += 1;
        }
        state.step(
            case["inputs"][frames.len()].as_u64().unwrap() as u16,
            frames.len() >= audio_end,
        );
        assert!(state.finished, "case {case_index}");
    }
    eprintln!("{checked} original JP prologue draws");
}
