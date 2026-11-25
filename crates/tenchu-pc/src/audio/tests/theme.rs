use super::*;
use crate::audio::{State as AudioState, theme::Theme};
use crate::level_editor::custom::{Asset, tests::wav};

fn theme() -> Arc<Asset> {
    Theme::imported("Owned tone.wav".into(), wav())
        .unwrap()
        .asset
        .unwrap()
}

#[test]
fn sandbox_theme_capture_matches_serde_and_bounds_restored_identity() {
    let asset = theme();
    let mut state = AudioState {
        current: Some(Source::Custom(asset.id().into())),
        mission: Some(Source::Custom(asset.id().into())),
        playhead: 31.057,
        sandbox: Some(SandboxState {
            fallback: None,
            asset: Some(asset),
        }),
    };
    let mut recorder = crate::rewind::Recorder::default();
    for _ in 0..3 {
        recorder.begin();
        state.capture(&mut recorder).unwrap();
        let snapshot = recorder.finish().unwrap();
        assert_eq!(
            snapshot.bytes().unwrap(),
            postcard::to_allocvec(&state).unwrap()
        );
        let copy: AudioState = snapshot.decode().unwrap();
        copy.validate(true).unwrap();
        assert!(copy.validate(false).is_err());
        assert_eq!(copy.playhead, state.playhead);
        state.playhead += 0.037;
    }
    state.current = Some(Source::Custom("wrong-content-id".into()));
    assert!(
        state
            .validate(true)
            .unwrap_err()
            .contains("Missing custom theme")
    );
    state.current = None;
    state.sandbox.as_mut().unwrap().asset = None;
    assert!(
        state.validate(true).is_err(),
        "Mission identity also needs retained bytes"
    );
    state.mission = None;
    state.validate(true).unwrap();
    state.sandbox = None;
    state.validate(false).unwrap();
    recorder.begin();
    state.capture(&mut recorder).unwrap();
    assert_eq!(
        recorder.finish().unwrap().bytes().unwrap(),
        postcard::to_allocvec(&state).unwrap()
    );
}

#[test]
fn sandbox_theme_whole_file_loop_seek_pause_and_volume_use_existing_mixer() {
    let clip = theme().clip().unwrap();
    let mut mixer = tenchu_audio::Mixer::new(8000);
    mixer.volume = 1.0;
    mixer.music_volume = 1.0;
    mixer.music(Some(clip.clone()), true);
    let first: Vec<_> = (0..800).map(|_| mixer.frame()).collect();
    assert!(first.iter().any(|f| f[0].abs() > 0.1));
    for _ in 0..3 {
        assert_eq!((0..800).map(|_| mixer.frame()).collect::<Vec<_>>(), first);
    }
    mixer.seek_music(17.0625);
    // 17 whole seconds are 170 whole files; retain the 62.5 ms position.
    assert_eq!(mixer.frame(), first[500]);
    mixer.paused = true;
    assert_eq!(mixer.frame(), [0.0; 2]);
    assert_eq!(mixer.frame(), [0.0; 2]);
    mixer.music_while_paused = true;
    assert_eq!(mixer.frame(), first[501]);
    mixer.volume = 0.5;
    mixer.music_volume = 0.5;
    let sample = mixer.frame();
    assert_eq!(sample, first[502].map(|x| x * 0.25));
    mixer.music_volume = 0.0;
    assert_eq!(mixer.frame(), [0.0; 2]);
}

#[test]
fn sandbox_theme_rewind_stores_one_retained_payload_across_frames() {
    let mut bytes = wav();
    bytes.truncate(44);
    let mut seed = 1u32;
    for _ in 0..65536 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        bytes.extend((seed as i16).to_le_bytes());
    }
    let length = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&(length - 8).to_le_bytes());
    bytes[40..44].copy_from_slice(&(length - 44).to_le_bytes());
    let asset = Theme::imported("Owned noise.wav".into(), bytes)
        .unwrap()
        .asset
        .unwrap();
    let mut state = AudioState {
        current: Some(Source::Custom(asset.id().into())),
        mission: Some(Source::Custom(asset.id().into())),
        playhead: 0.0,
        sandbox: Some(SandboxState {
            fallback: None,
            asset: Some(asset),
        }),
    };
    let payload = postcard::to_allocvec(&state).unwrap().len();
    let mut history = crate::rewind::History::default();
    for tick in 0..60 {
        state.playhead = tick as f64 / 30.0;
        history.recorder.begin();
        state.capture(&mut history.recorder).unwrap();
        let snapshot = history.recorder.finish().unwrap();
        history.push(tick, 30, snapshot, None).unwrap();
    }
    assert_eq!(history.entries.len(), 60);
    assert!(
        history.bytes < payload * 2,
        "{} history bytes for {payload}-byte payload",
        history.bytes
    );
    for index in [59, 0, 23] {
        let copy: AudioState = history.snapshot(index).unwrap().decode().unwrap();
        assert_eq!(copy.playhead, index as f64 / 30.0);
        copy.validate(true).unwrap();
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; custom/disc playback, cancellation, source cues, saved seek and default isolation"]
fn sandbox_theme_audio_transitions_keep_disc_identity_and_source_order() {
    use crate::campaign::Action;
    let gaisen = std::env::var_os("TENCHU_DISC").unwrap();
    let t2 = std::env::var_os("TENCHU2_DISC").unwrap();
    let assets = GameAssets::open(crate::source::open_disc(gaisen.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, Default::default(), None).unwrap();
    let mut sandbox = crate::tenchu2::Sandbox::open(t2.as_ref(), Some(0), None, None).unwrap();
    sandbox.music.selected = Some(3);
    let asset = theme();
    sandbox.music.custom = Some(asset.clone());
    campaign.start_sandbox(sandbox, &assets).unwrap();
    let (mut audio, commands) = recording(gaisen.as_ref(), &assets);
    audio.reload(&assets, &campaign).unwrap();
    commands.try_iter().for_each(drop);
    audio.screen(&campaign.screen);
    let source = Some(Source::Custom(asset.id().into()));
    assert_eq!(audio.current_track, source);
    let output: Vec<_> = commands.try_iter().collect();
    assert!(matches!(
        output.as_slice(),
        [
            Command::Clear,
            Command::Music(None, false),
            Command::Music(Some(_), true),
            Command::SeekMusic(0.0),
            Command::Tenchu2Ambience(_)
        ]
    ));
    if let Command::Music(Some(clip), true) = &output[2] {
        assert!(Arc::ptr_eq(clip, &asset.clip().unwrap()));
    }
    audio.pause(true, false);
    let mut state = audio.capture_state();
    state.playhead = 17.037;
    let bytes = postcard::to_allocvec(&state).unwrap();
    let mut saved: AudioState = postcard::from_bytes(&bytes).unwrap();
    saved.reuse_theme(&campaign);
    assert!(Arc::ptr_eq(
        saved.sandbox.as_ref().unwrap().asset.as_ref().unwrap(),
        &asset
    ));
    saved.validate(true).unwrap();
    saved.validate_session(&campaign).unwrap();
    let mut renamed: AudioState = postcard::from_bytes(&bytes).unwrap();
    Arc::make_mut(renamed.sandbox.as_mut().unwrap().asset.as_mut().unwrap()).name =
        "Renamed.wav".into();
    renamed.reuse_theme(&campaign);
    let renamed_asset = renamed.sandbox.as_ref().unwrap().asset.as_ref().unwrap();
    assert_eq!(renamed_asset.name, "Renamed.wav");
    assert!(Arc::ptr_eq(
        &renamed_asset.clip().unwrap(),
        &asset.clip().unwrap()
    ));
    renamed
        .sandbox
        .as_mut()
        .unwrap()
        .fallback
        .as_mut()
        .unwrap()
        .channel = 255;
    assert!(renamed.validate_session(&campaign).is_err());
    audio.restore_playback(&campaign, Some(saved));
    assert_eq!(audio.capture_state().playhead, 17.037);
    let output: Vec<_> = commands.try_iter().collect();
    let index = output
        .iter()
        .position(|c| matches!(c, Command::Music(Some(_), true)))
        .unwrap();
    assert!(matches!(output[index + 1], Command::SeekMusic(17.037)));
    audio.fast_forward(true);
    commands.try_iter().for_each(drop);
    audio.fast_forward(false);
    assert_eq!(audio.current_track, source);
    let output: Vec<_> = commands.try_iter().collect();
    assert!(
        output
            .iter()
            .any(|c| matches!(c, Command::SeekMusic(17.037)))
    );
    audio.volume(37);
    audio.preferences(&crate::frontend::Preferences {
        music: 93,
        effects: 42,
        stereo: false,
        ..Default::default()
    });
    audio.pause(true, true);
    assert!(matches!(commands.try_recv().unwrap(), Command::Volume(37)));
    assert!(matches!(
        commands.try_recv().unwrap(),
        Command::Balance(93, 42, false)
    ));
    assert!(matches!(
        commands.try_recv().unwrap(),
        Command::Pause {
            paused: true,
            music_while_paused: true
        }
    ));
    audio.pause(true, false);
    commands.try_iter().for_each(drop);

    let disc_track = campaign.sandbox.as_ref().unwrap().music.track().unwrap();
    // A queued XA request must never replace a newer custom clip, including when muted.
    for fast in [false, true] {
        audio.track(Some(disc_track.clone()));
        audio.fast_forward(fast);
        audio.play(source.clone(), 0.043);
        audio.fast_forward(false);
        commands.try_iter().for_each(drop);
        let deadline = Instant::now() + std::time::Duration::from_millis(50);
        while Instant::now() < deadline {
            audio.poll();
            assert!(commands.try_recv().is_err());
            std::thread::yield_now();
        }
        assert_eq!(audio.current_track, source);
    }
    // Source silence still wins. A later repeating source cue replaces the mission cue.
    campaign.game.music = vec![None];
    audio
        .tick(&campaign.game, None, Default::default(), [])
        .unwrap();
    assert!(audio.current_track.is_none());
    assert_eq!(audio.mission_track, source);
    audio.screen(&Screen::DeathHold { retry_seed: 1 });
    assert_eq!(audio.current_track, source);
    let cue = (0..=255)
        .find(|&id| audio.catalog.event(id).is_some_and(|t| t.repeat))
        .unwrap();
    let expected = audio.catalog.event(cue).cloned().map(Source::Disc);
    campaign.game.music = vec![Some(cue)];
    audio
        .tick(&campaign.game, None, Default::default(), [])
        .unwrap();
    assert_eq!(audio.current_track, expected);
    assert_eq!(audio.mission_track, expected);
    audio.screen(&Screen::GameOver);
    assert_eq!(
        audio.current_track,
        audio.catalog.music(11).cloned().map(Source::Disc)
    );
    campaign.action(Action::Replay, &assets).unwrap();
    audio.reload(&assets, &campaign).unwrap();
    audio.screen(&campaign.screen);
    assert_eq!(audio.current_track, source);
    audio.begin_tenchu2();
    assert!(audio.current_track.is_none());
    audio.fast_forward(true);
    audio.fast_forward(false);
    assert!(
        audio.current_track.is_none(),
        "Native handoff must not revive the sandbox theme"
    );
    campaign.sandbox = None;
    audio.reload(&assets, &campaign).unwrap();
    audio.screen(&Screen::Launcher);
    assert!(audio.capture_state().sandbox.is_none());
    assert_eq!(
        audio.current_track,
        audio.catalog.music(15).cloned().map(Source::Disc)
    );
}
