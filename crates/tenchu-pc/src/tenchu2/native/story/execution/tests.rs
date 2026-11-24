use super::*;
use crate::tenchu2::native::campaign::{Content, Playback};

fn discs() -> (Disc<crate::source::DiscSource>, std::path::PathBuf) {
    std::env::var_os("TENCHU_T2_CONFIG").expect("Use isolated disc preferences");
    let japan =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let europe = std::env::var_os("TENCHU2_EU_DISC").unwrap().into();
    (japan, europe)
}

fn sequence(disc: &Disc<impl ReadAt>) -> usize {
    tenchu_assets::tenchu2::campaign::Campaign::load(disc)
        .unwrap()
        .sequences
        .iter()
        .position(|row| {
            row.kind == tenchu_assets::tenchu2::campaign::Kind::Event && row.resource == 113
        })
        .unwrap()
}

fn shared_particle_tables(japan: &Disc<impl ReadAt>, europe: &Disc<impl ReadAt>) {
    fn executable(disc: &Disc<impl ReadAt>) -> Vec<u8> {
        let file = disc.find("TENCHU2/GAME.EXE").unwrap();
        disc.file(&file).read(0, file.size as usize).unwrap()
    }
    let jp = executable(japan);
    let eu = executable(europe);
    let jp = tenchu_assets::executable::Executable::new(&jp).unwrap();
    for (address, offset, size) in [
        (0x80083770, 0x70040, 192),
        (0x80083830, 0x70100, 48),
        (0x80012a18, 0x1218, 20),
        (0x80012a08, 0x1208, 14),
        (0x8008e928, 0x7b0a8, 6),
    ] {
        assert_eq!(jp.bytes(address, size).unwrap(), &eu[offset..offset + size]);
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_EU_DISC and isolated TENCHU_T2_CONFIG; source EU execution assets, 25 Hz timing, audio, default fallback and checkpoints"]
fn european_execution_preserves_authored_scene_and_restores() {
    let (disc, europe) = discs();
    shared_particle_tables(&disc, &crate::source::open_disc(&europe).unwrap());
    let sequence = sequence(&disc);
    let config = crate::storage::UserFile::Tenchu2.path().unwrap();
    crate::storage::atomic_write(&config, b"{}").unwrap();
    let fallback = Story::load_optional_execution(&disc, sequence, 113, 17, true).unwrap();
    assert_eq!(fallback.resources.edition, Edition::Japan);
    assert_eq!(fallback.resources.script.duration, 3000);
    let missing = config.with_file_name("missing-execution-data.cue");
    std::fs::write(
        &missing,
        "FILE \"missing-execution-data.bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n",
    )
    .unwrap();
    crate::storage::atomic_write(
        &config,
        &serde_json::to_vec(&serde_json::json!({"entries": [[Edition::Europe, missing]]})).unwrap(),
    )
    .unwrap();
    assert_eq!(
        Story::load_optional_execution(&disc, sequence, 113, 17, true)
            .unwrap()
            .resources
            .edition,
        Edition::Japan
    );
    std::fs::remove_file(missing).unwrap();
    crate::tenchu2::menu::remember(&europe).unwrap();
    let mut original = Story::load_optional_execution(&disc, sequence, 113, 17, false).unwrap();
    assert_eq!(original.resources.edition, Edition::Japan);
    assert!(!original.resources.models.contains_key(&274));
    let mut source = tenchu_core::tenchu2::story::Playback::new(&original.resources, 17);
    source.effects.particles.weather = original.state.effects.particles.weather.clone();
    for frame in 0..=3000 {
        let draw = frame % 7 != 3;
        original.step(draw).unwrap();
        source
            .step(
                &original.resources,
                &original.basis,
                &original.math,
                &original.rules,
                &original.level,
                draw,
            )
            .unwrap();
        assert_eq!(original.state.frame, frame);
        assert_eq!(original.phase, 0);
        assert_eq!(original.interpolation_alpha(0.25), 0.25);
    }
    assert_eq!(
        postcard::to_allocvec(&original.state).unwrap(),
        postcard::to_allocvec(&source).unwrap()
    );
    let mut story = Story::load_optional_execution(&disc, sequence, 113, 17, true).unwrap();
    assert_eq!(story.resources.edition, Edition::Europe);
    assert_eq!(story.resources.script.duration, 2000);
    assert_eq!(story.resources.script.actors.len(), 14);
    assert_eq!(story.resources.models[&93].name, "toda2");
    assert_eq!(story.resources.models[&274].name, "t_head");
    assert_eq!(story.resources.script.actors[3].model, 274);
    assert_eq!(story.resources.effects[&286].as_ref().unwrap().name, "fire");
    assert!(
        story
            .resources
            .script
            .commands
            .iter()
            .any(|c| c.frame == 410
                && matches!(
                    c.command,
                    tenchu_assets::tenchu2::story::Command::Effect {
                        kind: tenchu_assets::tenchu2::story::EffectKind::Blood,
                        ..
                    }
                ))
    );
    let clip = story.audio(&disc).unwrap().unwrap();
    assert!(
        clip.frames
            .iter()
            .flatten()
            .any(|sample| sample.unsigned_abs() > 100)
    );
    assert!((80.0..82.0).contains(&(clip.frames.len() as f64 / f64::from(clip.rate))));
    let mut presented = 0.0;
    let mut held = None;
    for tick in 0..=1201 {
        story.step(tick % 7 != 3).unwrap();
        assert_eq!(i32::from(story.state.frame), tick * 25 / 30);
        let before = f32::from(story.state.frame) - 1.0;
        if let Some(frame) = held {
            assert_eq!(before + story.interpolation_alpha(0.0), frame);
        }
        for alpha in [0.0, 0.25, 0.5, 0.75, 0.999] {
            let frame = (before + story.interpolation_alpha(alpha)).max(0.0);
            assert!(
                frame >= presented,
                "interpolation moved backwards at tick {tick}"
            );
            presented = frame;
        }
        held = (story.phase == 25).then_some(f32::from(story.state.frame));
    }
    assert_eq!(story.phase, 25);
    let saved = story.capture_state().unwrap();
    let mut recorder = crate::rewind::Recorder::default();
    recorder.begin();
    story.snapshot().capture(&mut recorder).unwrap();
    assert_eq!(
        recorder.finish().unwrap().bytes().unwrap(),
        postcard::to_allocvec(&story.snapshot()).unwrap()
    );
    let mut restored = Story::load(&disc, sequence, 113, 99).unwrap();
    restored.restore_state(&disc, &saved).unwrap();
    assert_eq!(restored.capture_state().unwrap(), saved);
    for tick in 0..120 {
        let draw = tick % 5 != 1;
        story.step(draw).unwrap();
        restored.step(draw).unwrap();
        assert_eq!(
            story.capture_state().unwrap(),
            restored.capture_state().unwrap()
        );
    }
    let mut nominal_tick = 1321;
    while !story.state.finished(&story.resources.script) {
        story.step(true).unwrap();
        nominal_tick += 1;
    }
    assert_eq!(nominal_tick, 2400);
    assert_eq!(story.seconds(), 80.0);
    assert_eq!(story.phase, 0);
    let mut playback = Playback::load(disc, sequence, 2, 17, Some([0; 21]), None).unwrap();
    let mut settings = crate::settings::Settings::default();
    settings.dismemberment.execution_scene = true;
    playback.configure(&settings).unwrap();
    let Content::Story { story, audio } = &playback.content else {
        panic!()
    };
    assert_eq!(story.resources.edition, Edition::Europe);
    assert!(audio.is_some());
    playback.content.interpolate(true).unwrap();
    let at_start = playback.capture_state().unwrap();
    settings.dismemberment.execution_scene = false;
    playback.configure(&settings).unwrap();
    playback.restore_state(&at_start).unwrap();
    settings.dismemberment.execution_scene = true;
    playback.configure(&settings).unwrap();
    assert_eq!(playback.capture_state().unwrap(), at_start);
    for _ in 0..121 {
        playback.step(0).unwrap();
    }
    let saved = playback.capture_state().unwrap();
    for _ in 0..31 {
        playback.step(0).unwrap();
    }
    playback.restore_state(&saved).unwrap();
    assert_eq!(playback.capture_state().unwrap(), saved);
    let (output, receiver) = crate::audio::output::Output::recording();
    playback.content.resume_audio(&output);
    assert!(receiver.try_iter().any(|command| matches!(command, crate::audio::output::Command::SeekMusic(seconds) if (seconds - 121.0 / 30.0).abs() < 1e-12)));
    crate::storage::atomic_write(&config, b"{}").unwrap();
    let before = playback.capture_state().unwrap();
    let mut fresh = Playback::load(
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap(),
        sequence,
        2,
        17,
        Some([0; 21]),
        None,
    )
    .unwrap();
    let fresh_before = fresh.capture_state().unwrap();
    assert!(
        fresh
            .restore_state(&saved)
            .unwrap_err()
            .contains("Reconnect")
    );
    assert_eq!(fresh.capture_state().unwrap(), fresh_before);
    assert_eq!(playback.capture_state().unwrap(), before);
    crate::tenchu2::menu::remember(&europe).unwrap();
    fresh.restore_state(&saved).unwrap();
    assert_eq!(fresh.capture_state().unwrap(), saved);
    settings.dismemberment.execution_scene = false;
    fresh.configure(&settings).unwrap();
    assert_eq!(
        fresh.capture_state().unwrap(),
        saved,
        "changing the preference mid-scene must not replace active source tracks"
    );
    assert!(fresh.step(0x2000).unwrap().stop_music);
    for _ in 0..30 {
        if fresh.run.sequence != sequence {
            break;
        }
        fresh.step(0).unwrap();
    }
    let next = fresh.campaign.sequences[sequence].next.unwrap();
    assert_eq!(fresh.run.sequence, next);
    let Content::Gameplay(session) = &fresh.content else {
        panic!()
    };
    let initial_tick = session.world.tick;
    for _ in 0..30 {
        fresh.step(0).unwrap();
    }
    let Content::Gameplay(session) = &fresh.content else {
        panic!()
    };
    assert_eq!(session.world.tick - initial_tick, 30);
    let other = Story::load_optional_execution(&fresh.disc, 202, 114, 17, true).unwrap();
    assert_eq!(other.resources.edition, Edition::Japan);
    assert_eq!(other.frames_per_second(), 30);
    assert_eq!(other.phase, 0);
    eprintln!(
        "E113 sequence {sequence}: JP3000/30 Hz unchanged; EU2000/25 Hz, 14 source tracks, English audio, full/reused restore, missing-disc fallback and seek verified"
    );
}
