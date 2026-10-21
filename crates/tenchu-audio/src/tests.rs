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
