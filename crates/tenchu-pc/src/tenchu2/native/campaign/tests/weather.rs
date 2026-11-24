use super::*;
use tenchu_core::tenchu2::{effects::weather::Kind, mission::Outcome};

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; weather suppresses source dust before allocation/RNG while smoke and water trails remain"]
fn original_weather_suppresses_only_dust_without_consuming_random() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for sequence in [9, 28, 56] {
        let session = Session::load(&disc, sequence, 1).unwrap();
        for enabled in [false, true] {
            for hidden in [false, true] {
                let mut effects = tenchu_core::tenchu2::effects::Effects::default();
                effects.weather = session.world.effects.weather.clone();
                if let Some(weather) = &mut effects.weather {
                    weather.enabled = enabled;
                }
                let suppressed = hidden || (sequence != 9 && enabled);
                let mut random = tenchu_core::random::Random::new(123);
                let math = &session.assets.resources.math;
                let position = tenchu_core::math::Position::ZERO;
                effects
                    .smoke(position, 0, hidden, math, &mut random)
                    .unwrap();
                effects
                    .trail(position, Default::default(), 0, hidden, &mut random)
                    .unwrap();
                assert_eq!(random.calls, if suppressed { 0 } else { 40 });
                assert_eq!(effects.entries.len(), if suppressed { 0 } else { 2 });
                let calls = random.calls;
                let count = effects.entries.len();
                effects
                    .smoke(position, 1, hidden, math, &mut random)
                    .unwrap();
                effects
                    .trail(position, Default::default(), 1, hidden, &mut random)
                    .unwrap();
                effects
                    .trail(position, Default::default(), 2, hidden, &mut random)
                    .unwrap();
                assert_eq!(random.calls - calls, 32);
                assert_eq!(effects.entries.len() - count, 3);
            }
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; same-frame weather commands gate dust in source order, including XA catch-up"]
fn original_weather_story_commands_preserve_dust_random_order() {
    use tenchu_assets::tenchu2::story::{Command, EffectKind, TimedCommand};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    for draw in [false, true] {
        let mut calls = Vec::new();
        for dust in [false, true] {
            let mut story = Story::load(&disc, 29, campaign.sequences[29].resource, 1).unwrap();
            story.resources.script.commands.clear();
            for enabled in [true, false, true] {
                story.resources.script.commands.push(TimedCommand {
                    frame: 0,
                    command: Command::Weather(enabled),
                });
                if dust {
                    story.resources.script.commands.push(TimedCommand {
                        frame: 0,
                        command: Command::Effect {
                            kind: EffectKind::Dust,
                            position: [0; 3],
                            heading: 0,
                            value: 0,
                        },
                    });
                }
            }
            story.step(draw).unwrap();
            assert!(
                story
                    .state
                    .effects
                    .particles
                    .weather
                    .as_ref()
                    .unwrap()
                    .enabled
            );
            assert_eq!(
                story.state.effects.particles.entries.len(),
                usize::from(dust)
            );
            calls.push(story.state.random.calls);
        }
        assert_eq!(calls[1] - calls[0], 16);
    }
}

#[test]
fn original_weather_audio_override_keeps_unrelated_sound_ids() {
    use crate::{audio::output, settings::WeatherMode};
    let weather = Sound {
        id: 43,
        slot: 1,
        note: 36,
        volume: [127; 2],
    };
    let unrelated = Sound { slot: 2, ..weather };
    let update = Update {
        weather_sound: Some(weather),
        sounds: vec![unrelated.into()],
        ..Default::default()
    };
    for mode in WeatherMode::ALL {
        let (output, recorded) = output::Output::recording();
        update.play_audio(&output, mode);
        let sounds: Vec<_> = recorded
            .try_iter()
            .filter_map(|command| match command {
                output::Command::Tenchu2Effect(sound) => Some(sound.sound),
                _ => None,
            })
            .collect();
        assert_eq!(
            sounds,
            if mode == WeatherMode::Original {
                vec![unrelated, weather]
            } else {
                vec![unrelated]
            }
        );
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; source thunder advances identically under visual overrides while only Original plays its cue"]
fn original_weather_transport_filters_thunder_without_changing_source_state() {
    use crate::{audio::output, settings::WeatherMode};
    let mut expected = None;
    for mode in WeatherMode::ALL {
        let mut playback = start(28);
        let Content::Gameplay(session) = &mut playback.content else {
            unreachable!()
        };
        let weather = session.world.effects.weather.as_mut().unwrap();
        weather.thunder_time = 1;
        weather.thunder_delay = 1;
        let (output, recorded) = output::Output::recording();
        let mut transport = super::super::super::transport::Transport::default();
        transport.weather = mode;
        transport
            .frame_step(&mut playback, 0, Some(&output), |_, _| Ok(()))
            .unwrap();
        let sounds: Vec<_> = recorded
            .try_iter()
            .filter_map(|command| match command {
                output::Command::Tenchu2Effect(sound) if sound.sound.id == 43 => Some(sound.sound),
                _ => None,
            })
            .collect();
        assert_eq!(sounds.len(), usize::from(mode == WeatherMode::Original));
        let Content::Gameplay(session) = &mut playback.content else {
            unreachable!()
        };
        assert_eq!(session.weather_sound.unwrap().id, 43);
        let state = postcard::to_allocvec(&session.world).unwrap();
        if let Some(expected) = &expected {
            assert_eq!(&state, expected);
        } else {
            expected = Some(state);
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; original authored weather in every affected gameplay layout"]
fn original_weather_covers_the_authored_gameplay_areas() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for sequence in [28, 30, 56, 58, 119, 121, 128, 189, 191, 198, 200, 203] {
        let session = Session::load(&disc, sequence, 1).unwrap();
        let weather = session.world.effects.weather.as_ref().unwrap();
        let kind = if [56, 58].contains(&sequence) {
            Kind::Snow
        } else {
            Kind::Rain
        };
        assert_eq!(weather.kind, kind);
        assert_eq!(
            weather.thunder,
            [28, 30, 128, 198, 200, 203].contains(&sequence)
        );
        println!(
            "sequence {sequence}: hero {}, map {}, {kind:?}",
            session.assets.hero, session.world.map
        );
    }
    assert!(
        Session::load(&disc, 9, 1)
            .unwrap()
            .world
            .effects
            .weather
            .is_none()
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; original weather across gameplay, cutscenes, skipped draws and checkpoint restoration"]
fn original_weather_carries_thunder_and_obeys_story_draw_schedule() {
    for (sequence, kind) in [(28, Kind::Rain), (56, Kind::Snow)] {
        let mut playback = start(sequence);
        let Content::Gameplay(session) = &mut playback.content else {
            panic!("not gameplay")
        };
        let weather = session.world.effects.weather.as_mut().unwrap();
        assert_eq!(weather.kind, kind);
        weather.thunder_time = 5;
        weather.thunder_delay = 2;
        session.world.mission.phase = Phase::Finished {
            outcome: Outcome::Arrived,
            finished_at: 300,
        };
        playback.step(0).unwrap();
        assert_eq!(playback.run.sequence, sequence + 1);
        assert_eq!(playback.run.weather_thunder, [5, 2]);
        let Content::Story { story, .. } = &mut playback.content else {
            panic!("not cutscene")
        };
        let weather = story.state.effects.particles.weather.as_ref().unwrap();
        assert_eq!(weather.kind, kind);
        assert!(!weather.thunder);
        assert_eq!([weather.thunder_time, weather.thunder_delay], [5, 2]);
        let initial_blocks = postcard::to_allocvec(&weather.blocks).unwrap();
        // Authored commands run on XA catch-up frames even though DrawEffects does not.
        use tenchu_assets::tenchu2::story::{Command, TimedCommand};
        let frame = story.state.frame + 1;
        story.state.presentation = Default::default();
        story.resources.script.commands = vec![
            TimedCommand {
                frame,
                command: Command::Weather(false),
            },
            TimedCommand {
                frame: frame + 2,
                command: Command::Weather(true),
            },
        ];
        story.step(false).unwrap();
        let weather = story.state.effects.particles.weather.as_ref().unwrap();
        assert!(!weather.enabled);
        assert_eq!(
            postcard::to_allocvec(&weather.blocks).unwrap(),
            initial_blocks
        );
        story.step(true).unwrap();
        assert!(story.state.effects.frame.weather.lines.is_empty());
        assert!(story.state.effects.frame.weather.sprites.is_empty());
        assert_eq!(
            postcard::to_allocvec(
                &story
                    .state
                    .effects
                    .particles
                    .weather
                    .as_ref()
                    .unwrap()
                    .blocks
            )
            .unwrap(),
            initial_blocks
        );
        story.step(false).unwrap();
        let weather = story.state.effects.particles.weather.as_ref().unwrap();
        assert!(weather.enabled);
        assert_eq!(
            postcard::to_allocvec(&weather.blocks).unwrap(),
            initial_blocks
        );
        story.step(true).unwrap();
        assert_ne!(
            postcard::to_allocvec(
                &story
                    .state
                    .effects
                    .particles
                    .weather
                    .as_ref()
                    .unwrap()
                    .blocks
            )
            .unwrap(),
            initial_blocks
        );
        let saved = story.capture_state().unwrap();
        let commands = story.resources.script.commands.clone();
        for draw in [false, true, false, true, true] {
            story.step(draw).unwrap();
        }
        let expected = postcard::to_allocvec(&story.state).unwrap();
        story.restore_state(&playback.disc, &saved).unwrap();
        story.resources.script.commands = commands;
        for draw in [false, true, false, true, true] {
            story.step(draw).unwrap();
        }
        assert_eq!(postcard::to_allocvec(&story.state).unwrap(), expected);
        story
            .state
            .begin_exit(Some(playback.campaign.sequences[sequence + 2]), false, 16);
        story.state.ending.as_mut().unwrap().complete = true;
        playback.step(0).unwrap();
        assert_eq!(playback.run.sequence, sequence + 2);
        let Content::Gameplay(session) = &playback.content else {
            panic!("not boss area")
        };
        let weather = session.world.effects.weather.as_ref().unwrap();
        assert_eq!(weather.kind, kind);
        assert_eq!([weather.thunder_time, weather.thunder_delay], [5, 2]);
    }
}
