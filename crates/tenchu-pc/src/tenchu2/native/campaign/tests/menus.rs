use super::*;
use crate::{controls::Controls, settings::Settings, tenchu2::native::tools::Tools};
use tenchu_core::input;

fn buttons(content: &mut Content, settings: &Settings, buttons: u16) -> u16 {
    let mut controls = Controls::default();
    controls.touch(crate::touch::Sample {
        buttons: buttons.into(),
        ..Default::default()
    });
    Tools::default().input(content, &mut controls, settings, true)
}

#[test]
#[ignore = "requires TENCHU2_DISC; both menu conventions traverse real selection, equipment, results and retry without changing gameplay/cutscene controls"]
fn native_menu_convention_applies_at_the_input_boundary() {
    let disc =
        || crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for circle_confirm in [false, true] {
        let settings = Settings {
            circle_confirm,
            ..Default::default()
        };
        let (confirm, back) = if circle_confirm {
            (input::CIRCLE, input::CROSS)
        } else {
            (input::CROSS, input::CIRCLE)
        };
        for cancel in [false, true] {
            let mut playback = Playback::menu(disc(), 2, 17, None).unwrap();
            for _ in 0..15 {
                playback.step(0).unwrap();
            }
            let mut controls = Controls::default();
            let mut tools = Tools::default();
            controls.touch(crate::touch::Sample {
                buttons: u32::from(if cancel { back } else { confirm }),
                ..Default::default()
            });
            let pad = tools.input(&mut playback.content, &mut controls, &settings, true);
            playback.step(pad).unwrap();
            let Content::MissionSelection(selection) = &playback.content else {
                unreachable!()
            };
            assert_eq!(
                selection.state.outcome == Some(tenchu_core::tenchu2::selection::Outcome::Cancel),
                cancel
            );
            if cancel {
                let mut quit = false;
                for _ in 0..20 {
                    quit |= playback.step(0).unwrap().quit;
                }
                assert!(!quit);
                assert!(matches!(playback.content, Content::Title(_)));
            } else {
                next(&mut playback);
                assert!(matches!(
                    playback.content,
                    Content::Prologue(_) | Content::Story { .. }
                ));
                assert_eq!(
                    tools.input(&mut playback.content, &mut controls, &settings, true),
                    0,
                    "holding confirm across the menu must not skip the opening scene"
                );
                controls.touch(Default::default());
                controls.touch(crate::touch::Sample {
                    buttons: u32::from(confirm),
                    ..Default::default()
                });
                assert_eq!(
                    tools.input(&mut playback.content, &mut controls, &settings, true),
                    confirm.swap_bytes()
                );
                assert_eq!(
                    buttons(&mut playback.content, &settings, input::CROSS),
                    0x4000
                );
                assert_eq!(
                    buttons(&mut playback.content, &settings, input::CIRCLE),
                    0x2000
                );
            }
        }
        let mut playback = Playback::load(disc(), 9, 2, 17, None, None).unwrap();
        for _ in 0..15 {
            playback.step(0).unwrap();
        }
        let Content::Equipment(equipment) = &playback.content else {
            unreachable!()
        };
        let original = equipment.state.slots.clone();
        let stock = equipment.state.stock.items;
        let pad = buttons(&mut playback.content, &settings, confirm);
        playback.step(pad).unwrap();
        let Content::Equipment(equipment) = &playback.content else {
            unreachable!()
        };
        assert_ne!(equipment.state.slots, original);
        playback.step(0).unwrap();
        let pad = buttons(&mut playback.content, &settings, back);
        playback.step(pad).unwrap();
        let Content::Equipment(equipment) = &playback.content else {
            unreachable!()
        };
        assert_eq!(equipment.state.slots, original);
        assert_eq!(equipment.state.stock.items, stock);
        playback.confirm();
        assert!(matches!(playback.action, Some(Action::Continue)));
        playback.step(0).unwrap();
        next(&mut playback);
        assert!(matches!(playback.content, Content::Gameplay(_)));
        for physical in [input::CROSS, input::CIRCLE, input::R1] {
            assert_eq!(
                buttons(&mut playback.content, &settings, physical),
                physical.swap_bytes()
            );
        }

        for (physical, expected) in [
            (confirm, Action::Continue),
            (back, Action::Quit),
            (input::START, Action::Retry),
        ] {
            let mut playback = start(13);
            let pad = buttons(&mut playback.content, &settings, physical);
            playback.step(pad).unwrap();
            let Content::Results(result) = &playback.content else {
                unreachable!()
            };
            assert!(matches!(
                (result.exit, expected),
                (Some((Action::Continue, 30)), Action::Continue)
                    | (Some((Action::Quit, 30)), Action::Quit)
                    | (Some((Action::Retry, 30)), Action::Retry)
            ));
            for _ in 0..29 {
                assert!(!playback.step(0).unwrap().quit);
            }
            let update = playback.step(0).unwrap();
            match expected {
                Action::Continue => assert_eq!(playback.run.sequence, 15),
                Action::Retry => assert_eq!(playback.run.sequence, 9),
                Action::Quit => assert!(update.quit),
                _ => unreachable!(),
            }
        }
        for complete in [false, true] {
            for from_menu in [false, true] {
                for retry in [false, true] {
                    let mut playback = start(13);
                    playback.from_menu = from_menu;
                    playback.content = if complete {
                        Content::Complete
                    } else {
                        Content::Failed
                    };
                    let pad = buttons(
                        &mut playback.content,
                        &settings,
                        if retry { confirm } else { back },
                    );
                    let update = playback.step(pad).unwrap();
                    if retry {
                        assert!(!update.quit);
                        assert_eq!(playback.run.sequence, 9);
                    } else if from_menu {
                        assert!(
                            !update.quit
                                && matches!(playback.content, Content::MissionSelection(_))
                        );
                    } else {
                        assert!(update.quit);
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; title, mission selection/back, options, editor boundary and both attract movies"]
fn title_menu_enters_campaign_and_returns_without_changing_progress() {
    use tenchu_core::tenchu2::title::Choice;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut playback = Playback::title(disc, 2, 17, None).unwrap();
    let progress = postcard::to_allocvec(&playback.run.progress).unwrap();
    for _ in 0..15 {
        playback.step(0).unwrap();
    }
    playback.confirm();
    playback.step(0).unwrap();
    for _ in 0..20 {
        playback.step(0x2000).unwrap();
    }
    assert!(
        matches!(playback.content, Content::Title(_)),
        "held start must only reveal the menu"
    );
    playback.step(0).unwrap();
    playback.step(0x2000).unwrap();
    next(&mut playback);
    assert!(matches!(playback.content, Content::MissionSelection(_)));
    for _ in 0..15 {
        playback.step(0).unwrap();
    }
    playback.action = Some(Action::Quit);
    playback.step(0).unwrap();
    next(&mut playback);
    assert!(matches!(playback.content, Content::Title(_)));
    for choice in [Choice::Options, Choice::Editor] {
        for _ in 0..15 {
            playback.step(0).unwrap();
        }
        playback.action = Some(Action::Title(choice));
        playback.step(0).unwrap();
        let generation = playback.generation;
        let mut settings = false;
        for _ in 0..20 {
            let update = playback.step(0).unwrap();
            assert!(!update.quit);
            settings |= update.settings;
            if playback.generation != generation {
                break;
            }
        }
        assert_eq!(settings, choice == Choice::Options);
        let Content::Title(title) = &playback.content else {
            panic!("not title")
        };
        assert_eq!(title.editor_notice, choice == Choice::Editor);
    }
    playback.step(0).unwrap();
    playback.step(0x4000).unwrap();
    assert!(matches!(&playback.content, Content::Title(title) if !title.editor_notice));
    next(&mut playback);
    assert!(matches!(
        playback.content,
        Content::TitleMovie { opening: true, .. }
    ));
    for opening in [true, false] {
        let Content::TitleMovie {
            movie,
            opening: actual,
        } = &playback.content
        else {
            panic!("not attract movie")
        };
        assert_eq!(*actual, opening);
        assert!(movie.frame.is_some());
        assert!(
            movie.audio.as_ref().is_some_and(|audio| audio
                .frames
                .iter()
                .flatten()
                .any(|&sample| sample != 0))
        );
        playback.step(8).unwrap();
        assert!(matches!(&playback.content, Content::TitleMovie { movie, .. } if !movie.finished));
        playback.step(0).unwrap();
        playback.step(8).unwrap();
        next(&mut playback);
    }
    assert!(matches!(&playback.content, Content::Title(title) if title.state.fade == 0));
    assert_eq!(
        postcard::to_allocvec(&playback.run.progress).unwrap(),
        progress
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; taps between logic ticks survive redraws, while pause and scene changes discard pending menu input"]
fn native_menu_taps_survive_render_cadence() {
    use crate::tenchu2::native::transport::{Input, Transport};
    use std::time::Duration;
    let mut playback = start(13);
    let mut transport = Transport::default();
    let mut input = Input {
        buttons: 0x2000,
        cutscene_speed: 1,
        simulation_speed: 100,
        fast_forward: false,
        paused: false,
    };
    transport
        .advance(&mut playback, Duration::ZERO, input, None, |_, _| Ok(()))
        .unwrap();
    let Content::Results(results) = &playback.content else {
        unreachable!()
    };
    assert!(results.exit.is_none());
    input.buttons = 0;
    transport
        .advance(
            &mut playback,
            Duration::from_millis(34),
            input,
            None,
            |_, _| Ok(()),
        )
        .unwrap();
    let Content::Results(results) = &playback.content else {
        unreachable!()
    };
    assert!(matches!(results.exit, Some((Action::Continue, 30))));
    next(&mut playback);
    assert_eq!(playback.run.sequence, 15);

    let mut playback = start(13);
    input.buttons = 0x4000;
    transport
        .advance(&mut playback, Duration::ZERO, input, None, |_, _| Ok(()))
        .unwrap();
    input.paused = true;
    transport
        .advance(
            &mut playback,
            Duration::from_millis(34),
            input,
            None,
            |_, _| Ok(()),
        )
        .unwrap();
    input.buttons = 0;
    input.paused = false;
    transport
        .advance(
            &mut playback,
            Duration::from_millis(34),
            input,
            None,
            |_, _| Ok(()),
        )
        .unwrap();
    let Content::Results(results) = &playback.content else {
        unreachable!()
    };
    assert!(results.exit.is_none());
}
