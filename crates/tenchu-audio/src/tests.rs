use super::*;
use tenchu_assets::audio::{
    adpcm::Sample,
    bank::{Program, Tone},
};

fn bank(loop_start: Option<usize>) -> Arc<Bank> {
    Arc::new(Bank {
        volume: 127,
        pan: 64,
        programs: vec![Some(Program {
            volume: 127,
            pan: 64,
            tones: vec![Tone {
                mode: 4,
                volume: 127,
                pan: 64,
                center: 36,
                fine: 0,
                adsr: [0x000f, 0x1fc0],
                sample: 0,
            }],
        })],
        samples: vec![Sample {
            frames: (0..200).map(|i| (i * 127 - 12000) as i16).collect(),
            loop_start,
        }],
    })
}

#[test]
fn stage_ambience_keeps_reserved_voices_and_follows_pause_and_effect_volume() {
    let mut source = bank(Some(0));
    let bank = Arc::get_mut(&mut source).unwrap();
    let program = bank.programs[0].as_mut().unwrap();
    program.tones.resize(16, program.tones[0].clone());
    let source = tenchu_assets::tenchu2::music::Ambience {
        bank: source,
        interval: 142,
        ticks_per_second: 30,
        ship_variants: false,
    };
    for rate in [44100, 48000] {
        let setup = || {
            let mut mixer = Mixer::new(rate);
            mixer.set_bank(source.bank.clone());
            mixer.tenchu2_ambience(source.clone());
            mixer
        };
        let mut expected = vec![[0.0; 2]; rate as usize * 6];
        setup().render(&mut expected);
        assert!(expected.iter().flatten().any(|v| v.abs() > 0.01));
        let mut actual = setup();
        let mut offset = 0;
        while offset < expected.len() {
            let count = 347.min(expected.len() - offset);
            let mut frames = vec![[0.0; 2]; count];
            actual.paused = true;
            actual.render(&mut frames);
            assert!(frames.iter().all(|&s| s == [0.0; 2]));
            actual.paused = false;
            actual.render(&mut frames);
            assert_eq!(frames, expected[offset..offset + count]);
            // Exhausting effect voices cannot overwrite the stage's reserved voices.
            for _ in 0..48 {
                actual.play(0, 0, 64);
            }
            offset += count;
        }
        actual.effects_volume = 0.0;
        assert_eq!(actual.frame(), [0.0; 2]);
        actual.effects_volume = 1.0;
        actual.clear_effects();
        assert_eq!(actual.frame(), [0.0; 2]);
        actual.tenchu2_ambience(source.clone());
        let mut frames = vec![[0.0; 2]; 100];
        actual.render(&mut frames);
        assert!(frames.iter().flatten().any(|v| v.abs() > 0.01));
    }
}

#[test]
fn seeking_restores_stream_position_across_resampling_pause_and_looping() {
    for rate in [44100, 48000] {
        for repeat in [false, true] {
            for elapsed in [37, 223, 1971] {
                let setup = || {
                    let mut mixer = Mixer::new(rate);
                    mixer.music(
                        Some(Arc::new(Clip {
                            rate: 37800,
                            frames: (0..351).map(|i| [i * 31 - 1200, 4800 - i * 47]).collect(),
                        })),
                        repeat,
                    );
                    mixer
                };
                let mut uninterrupted = setup();
                for _ in 0..elapsed {
                    uninterrupted.frame();
                }
                let mut resumed = setup();
                resumed.seek_music(f64::from(elapsed) / f64::from(rate));
                resumed.paused = true;
                assert_eq!(resumed.frame(), [0.0; 2]);
                resumed.paused = false;
                for _ in 0..1000 {
                    let expected = uninterrupted.frame();
                    let actual = resumed.frame();
                    for channel in 0..2 {
                        assert!((expected[channel] - actual[channel]).abs() < 0.00001);
                    }
                }
            }
        }
    }
}

#[test]
fn audio_buffers_and_pauses_do_not_change_sample_order() {
    let setup = || {
        let mut mixer = Mixer::new(48000);
        mixer.set_bank(bank(Some(14)));
        mixer.play(0, 126, 21);
        mixer.music(
            Some(Arc::new(Clip {
                rate: 37800,
                frames: (0..351).map(|i| [i * 31 - 1200, 4800 - i * 47]).collect(),
            })),
            true,
        );
        mixer
    };
    let mut expected = vec![[0.0; 2]; 48000];
    setup().render(&mut expected);
    let mut mixer = setup();
    let mut actual = Vec::new();
    for size in [1, 101, 480, 63, 4096, 43259] {
        let mut buffer = vec![[0.0; 2]; size];
        mixer.paused = true;
        mixer.render(&mut buffer);
        assert!(buffer.iter().all(|&frame| frame == [0.0; 2]));
        mixer.paused = false;
        mixer.render(&mut buffer);
        actual.extend(buffer);
    }
    assert_eq!(actual, expected);
    assert!(actual.iter().flatten().any(|v| v.abs() > 0.1));
}

#[test]
fn pause_chimes_finish_without_advancing_game_sounds_or_music() {
    for rate in [44100, 48000] {
        let setup = || {
            let mut mixer = Mixer::new(rate);
            mixer.set_bank(bank(None));
            mixer.play(0, 126, 0);
            mixer.music(
                Some(Arc::new(Clip {
                    rate: 37800,
                    frames: (0..351).map(|i| [i * 31 - 1200, 4800 - i * 47]).collect(),
                })),
                true,
            );
            mixer.render(&mut [[0.0; 2]; 31]);
            mixer
        };
        let mut expected = [[0.0; 2]; 150];
        setup().render(&mut expected);
        let mut mixer = setup();
        mixer.paused = true;
        mixer.play_ui(0, 126, 127);
        let mut paused = [[0.0; 2]; 512];
        mixer.render(&mut paused);
        assert!(paused.iter().all(|frame| frame[0] == 0.0));
        assert!(paused.iter().any(|frame| frame[1].abs() > 0.1));
        assert!(paused[300..].iter().all(|&frame| frame == [0.0; 2]));
        mixer.paused = false;
        let mut actual = [[0.0; 2]; 150];
        mixer.render(&mut actual);
        assert_eq!(actual, expected);
    }
}

#[test]
fn optional_pause_music_keeps_playing_while_effects_stay_frozen() {
    let mut mixer = Mixer::new(48000);
    mixer.set_bank(bank(Some(14)));
    mixer.play(0, 126, 0);
    let clip = Arc::new(Clip {
        rate: 37800,
        frames: (0..351).map(|i| [i * 31 - 1200, 4800 - i * 47]).collect(),
    });
    mixer.music(Some(clip.clone()), true);
    mixer.paused = true;
    mixer.music_while_paused = true;
    let mut music_only = Mixer::new(48000);
    music_only.music(Some(clip), true);
    for _ in 0..3 {
        let mut actual = [[0.0; 2]; 701];
        let mut expected = [[0.0; 2]; 701];
        mixer.render(&mut actual);
        music_only.render(&mut expected);
        assert_eq!(actual, expected);
        mixer.music_while_paused = false;
        mixer.render(&mut actual);
        assert!(actual.iter().all(|&frame| frame == [0.0; 2]));
        mixer.music_while_paused = true;
    }
    mixer.music(None, false);
    mixer.paused = false;
    let mut effects_only = Mixer::new(48000);
    effects_only.set_bank(bank(Some(14)));
    effects_only.play(0, 126, 0);
    for _ in 0..1000 {
        assert_eq!(mixer.frame(), effects_only.frame());
    }
}

#[test]
fn effects_end_loop_pan_and_replace_the_oldest_of_twenty_four_voices() {
    let mut mixer = Mixer::new(44100);
    mixer.set_bank(bank(None));
    mixer.play(0, 126, 0);
    let mut frames = [[0.0; 2]; 210];
    mixer.render(&mut frames);
    assert!(frames.iter().all(|frame| frame[1] == 0.0));
    assert!(frames[200..].iter().all(|&frame| frame == [0.0; 2]));
    assert!(mixer.voices.iter().all(Option::is_none));
    mixer.set_bank(bank(Some(20)));
    for _ in 0..24 {
        mixer.play(0, 126, 64);
    }
    assert_eq!(mixer.voices.iter().flatten().count(), 24);
    mixer.render(&mut frames);
    assert_eq!(mixer.voices.iter().flatten().count(), 24);
    mixer.play(0, 126, 64);
    assert_eq!(
        mixer
            .voices
            .iter()
            .flatten()
            .filter(|v| v.position == 0.0)
            .count(),
        1
    );
    mixer.set_bank(bank(None));
    assert!(mixer.voices.iter().all(Option::is_none));
    mixer.music(
        Some(Arc::new(Clip {
            rate: 44100,
            frames: vec![[12000, -12000]; 10],
        })),
        false,
    );
    mixer.render(&mut frames);
    assert!(
        frames[..10]
            .iter()
            .all(|frame| frame[0] > 0.0 && frame[1] < 0.0)
    );
    assert!(frames[10..].iter().all(|&frame| frame == [0.0; 2]));
}

#[test]
fn movie_voice_pitch_and_independent_music_effects_balance() {
    let make = |note, music_volume, effects_volume, stereo| {
        let mut mixer = Mixer::new(44100);
        mixer.set_bank(bank(None));
        mixer.play_note(0, 126, 0, note);
        mixer.music(
            Some(Arc::new(Clip {
                rate: 44100,
                frames: vec![[6000, 12000]; 250],
            })),
            false,
        );
        mixer.music_volume = music_volume;
        mixer.effects_volume = effects_volume;
        mixer.stereo = stereo;
        let mut frames = [[0.0; 2]; 250];
        mixer.render(&mut frames);
        frames
    };
    let normal = make(36, 0.0, 1.0, true);
    let high = make(48, 0.0, 1.0, true);
    assert!(normal[100..200].iter().any(|v| v[0] != 0.0));
    assert!(high[100..].iter().all(|v| v[0] == 0.0));
    let music = make(48, 1.0, 0.0, true);
    let mixed = make(48, 0.25, 0.75, true);
    let mono = make(48, 0.25, 0.75, false);
    for i in 0..250 {
        for channel in 0..2 {
            assert!(
                (mixed[i][channel] - (high[i][channel] * 0.75 + music[i][channel] * 0.25)).abs()
                    < 1e-6
            );
        }
        assert_eq!(mono[i], [(mixed[i][0] + mixed[i][1]) * 0.5; 2]);
    }
}
