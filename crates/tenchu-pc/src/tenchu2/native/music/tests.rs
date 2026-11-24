use super::super::{
    Session,
    campaign::{Action, Content, Playback},
};
use super::*;
use crate::audio::output::{Command, Output};
use tenchu_assets::tenchu2::campaign::{Kind, Objective, Region, Sequence};
use tenchu_core::tenchu2::mission::{Mission, Outcome, Phase};

fn disc() -> Disc<crate::source::DiscSource> {
    crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap()
}

fn start(sequence: usize) -> Playback<crate::source::DiscSource> {
    Playback::load(disc(), sequence, 2, 17, Some([0; 21]), None).unwrap()
}

fn session(playback: &mut Playback<crate::source::DiscSource>) -> &mut Session {
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("not gameplay")
    };
    session
}

fn death_event(session: &mut Session, falling: bool) -> Option<u16> {
    use tenchu_core::tenchu2::world::controls::{Event, EventSource, Events};
    let listener = session.world.listener();
    let index = session.world.player;
    let mut output = super::super::Output {
        player: index,
        feedback: &mut session.feedback,
        voice: None,
        weapon_voice: None,
        actor_voices: &session.actor_voices,
        listener,
        requests: Vec::new(),
        wounds: Vec::new(),
        melee_hits: Default::default(),
        gunshots: Vec::new(),
        music: None,
        pending: &mut session.pending,
    };
    output
        .emit(
            EventSource {
                listener,
                voice: session.world.instances[index].voice,
                index,
                actor: &session.world.actors[index].actor,
                rig: &session.world.instances[index].rig,
                scene: &mut session.world.scene,
            },
            if falling {
                Event::FallDeathMusic
            } else {
                Event::DeathMusic
            },
            &mut session.world.random,
        )
        .unwrap();
    output.music
}

#[test]
#[ignore = "requires TENCHU2_MUSIC_ORACLE; all target kinds, source health/suppression and XA busy branches"]
fn native_boss_music_matches_original_scheduler() {
    #[derive(serde::Deserialize)]
    struct Case {
        objective: u8,
        target: i16,
        health: i16,
        enabled: bool,
        idle: bool,
        cue: Option<u16>,
    }
    let text = std::fs::read_to_string(std::env::var_os("TENCHU2_MUSIC_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    for line in text.lines() {
        let case: Case = serde_json::from_str(line).unwrap();
        let objective = match case.objective {
            1 | 2 => Objective::Reach {
                region: Region {
                    center: [0; 3],
                    radius: 0,
                },
                unseen: case.objective == 2,
            },
            3 => Objective::Weaken {
                targets: [case.target, 0, 0],
            },
            4 => Objective::Defeat {
                targets: [case.target, 0, 0],
            },
            5 => Objective::EliminateUnseen { count: 1 },
            6 => Objective::EliminateAll,
            _ => unreachable!(),
        };
        let mission = Mission::new(
            Sequence {
                kind: Kind::Gameplay(objective),
                resource: 0,
                next: None,
                failure: None,
            },
            None,
            0,
        )
        .unwrap();
        assert_eq!(
            case.idle
                .then(|| mission.boss_music(case.enabled, case.health, case.target))
                .flatten(),
            case.cue,
            "source case {checked}"
        );
        checked += 1;
    }
    assert_eq!(checked, 18_432);
    eprintln!("{checked} original JP boss-music scheduler cases");
}

#[test]
#[ignore = "requires JP or EU TENCHU2_DISC; original cue descriptors retain the optional sandbox's repeat behavior"]
fn retail_music_cues_preserve_sandbox_selection() {
    use tenchu_assets::tenchu2::music::{THEMES, cues, tracks};
    let disc = disc();
    let cues = cues(&disc).unwrap();
    assert_eq!(cues.len(), 19);
    for (&id, cue) in &cues {
        assert_eq!(cue.repeat, id <= 3);
    }
    for ((id, _), track) in THEMES.into_iter().zip(tracks(&disc).unwrap()) {
        let mut expected = cues[&id].clone();
        expected.repeat = true;
        assert_eq!(track, expected);
    }
    for id in [9, 10] {
        let clip = Clip::load(&disc, &cues[&id]).unwrap();
        assert!(
            clip.frames
                .iter()
                .flatten()
                .any(|sample| sample.unsigned_abs() > 100)
        );
    }
    eprintln!(
        "{:?}: all 19 raw music cues, eight sandbox selections and two decoded battle themes",
        tenchu_assets::tenchu2::Edition::detect(&disc).unwrap()
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; all authored boss areas request their decoded retail theme through campaign audio"]
fn native_boss_music_covers_every_campaign_battle() {
    let disc = disc();
    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    let cues = tenchu_assets::tenchu2::music::cues(&disc).unwrap();
    let themes = tenchu_assets::tenchu2::music::tracks(&disc).unwrap();
    assert!(themes.iter().all(|track| track.repeat));
    assert!(!cues[&9].repeat && !cues[&10].repeat);
    let mut battles = Vec::new();
    for layout in layouts {
        let mut playback = start(layout.sequence);
        let update = playback.step(0).unwrap();
        let expected = match layout.sequence {
            96 | 179 | 203 | 245 => Some(10),
            11 | 22 | 30 | 41 | 49 | 58 | 66 | 74 | 89 | 94 | 112 | 121 | 139 | 147 | 160 | 175
            | 191 | 200 | 216 | 229 => Some(9),
            _ => None,
        };
        assert_eq!(
            session(&mut playback).music.playing.map(|p| p.0),
            expected,
            "sequence {}",
            layout.sequence
        );
        assert_eq!(update.music.is_some(), expected.is_some());
        if let Some(cue) = expected {
            let clip = update.music.as_ref().unwrap();
            assert!(
                clip.frames
                    .iter()
                    .flatten()
                    .any(|sample| sample.unsigned_abs() > 100)
            );
            let (output, receiver) = Output::recording();
            update.play_audio(&output, crate::settings::WeatherMode::Original);
            assert_eq!(
                receiver
                    .try_iter()
                    .filter(|command| matches!(command, Command::Music(Some(_), false)))
                    .count(),
                1
            );
            assert!(playback.step(0).unwrap().music.is_none());
            battles.push((layout.sequence, cue));
        }
    }
    assert_eq!(battles.len(), 24);
    assert_eq!(battles.iter().filter(|(_, cue)| *cue == 10).count(), 4);
    eprintln!("All 24 authored battles: {battles:?}");
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; retail end-sector restart timing, quiet tail, death handoff and mission-ending order"]
fn native_boss_music_restarts_at_source_duration_and_obeys_end_order() {
    let disc = disc();
    for sequence in [11, 96, 179, 245] {
        let mut session = Session::load(&disc, sequence, 17).unwrap();
        session.step(0).unwrap();
        let clip = session.music.audio(&disc).unwrap().unwrap();
        let (cue, start) = session.music.playing.unwrap();
        let track = &session.music.tracks[&cue];
        let duration = (track.end - track.start) / 5;
        let mut mixer = tenchu_audio::Mixer::new(44_100);
        mixer.music(Some(clip.clone()), false);
        mixer.seek_music(1.0);
        let mut samples = [[0.0; 2]; 2048];
        mixer.render(&mut samples);
        assert!(samples.iter().flatten().any(|sample| sample.abs() > 0.001));
        let decoded = clip.frames.len() as f64 / f64::from(clip.rate);
        assert!(decoded <= f64::from(duration) / 30.0);
        mixer.seek_music(decoded + 0.1);
        mixer.render(&mut samples);
        assert!(samples.iter().flatten().all(|sample| *sample == 0.0));
        for loop_number in 1..=3 {
            session.world.tick = start + duration * loop_number - 1;
            session.music.step(&session.world, None);
            assert!(session.music.audio(&disc).unwrap().is_none());
            session.world.tick += 1;
            session.music.step(&session.world, None);
            assert!(Arc::ptr_eq(
                &session.music.audio(&disc).unwrap().unwrap(),
                &clip
            ));
        }
        let track = session.music.tracks.get_mut(&cue).unwrap();
        let end = track.end;
        track.end = track.start + 31;
        session.music.playing = Some((cue, u32::MAX - 2));
        session.world.tick = 2;
        session.music.step(&session.world, None);
        assert!(session.music.audio(&disc).unwrap().is_none());
        session.world.tick = 3;
        session.music.step(&session.world, None);
        assert!(
            session.music.audio(&disc).unwrap().is_some(),
            "sector-to-tick division truncates; elapsed ticks wrap"
        );
        session.music.tracks.get_mut(&cue).unwrap().end = end;
        session.world.tick += 1;
        let death_cue = session.assets.hero as u16 * 2 + 14;
        session.world.actors[session.world.player].actor.health = 0;
        let random = session.world.random.seed;
        let event = death_event(&mut session, false);
        assert_eq!(event, Some(death_cue));
        assert_eq!(session.world.random.seed, random);
        session.music.step(&session.world, event);
        assert_eq!(session.music.playing, Some((death_cue, session.world.tick)));
        let death = session.music.audio(&disc).unwrap().unwrap();
        assert!(!Arc::ptr_eq(&death, &clip));
        assert!(
            death
                .frames
                .iter()
                .flatten()
                .any(|sample| sample.unsigned_abs() > 100)
        );
        let falling = death_event(&mut session, true);
        assert_eq!(falling, Some(death_cue + 1));
        session.music.step(&session.world, falling);
        let falling = session.music.audio(&disc).unwrap().unwrap();
        assert!(!Arc::ptr_eq(&falling, &death));
        assert!(
            falling
                .frames
                .iter()
                .flatten()
                .any(|sample| sample.unsigned_abs() > 100)
        );
        session.world.tick += 10_000;
        session.music.step(&session.world, None);
        assert!(session.music.audio(&disc).unwrap().is_none());
        eprintln!(
            "sequence {sequence}, cue {cue}: {decoded:.2}s decoded, {} ticks per retail restart; three loops and death handoff",
            duration
        );
    }
    let mut session = Session::load(&disc, 11, 17).unwrap();
    let target = session.world.mission.targets[0].unwrap();
    session.world.actors[target].actor.health = 0;
    session.step(0).unwrap();
    assert!(matches!(session.world.mission.phase, Phase::Ending { .. }));
    assert!(
        session.music.audio(&disc).unwrap().is_some(),
        "source schedules music before beginning the end countdown"
    );
    session.world.tick += 10_000;
    session.music.step(&session.world, None);
    assert!(
        session.music.audio(&disc).unwrap().is_none(),
        "ending countdown must not restart the battle theme"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; malformed music state is rejected before replacing live gameplay"]
fn native_boss_music_rejects_invalid_checkpoints() {
    let disc = disc();
    let mut session = Session::load(&disc, 11, 17).unwrap();
    session.step(0).unwrap();
    let pending = session.capture_state().unwrap();
    session.music.audio(&disc).unwrap().unwrap();
    session.restore_state(&pending).unwrap();
    assert!(session.music.audio(&disc).unwrap().is_some());
    assert!(session.music.audio(&disc).unwrap().is_none());
    let saved = session.capture_state().unwrap();
    for case in 0..5 {
        match case {
            0 => session.music.tracks.get_mut(&9).unwrap().end = 0,
            1 => session.music.tracks.get_mut(&9).unwrap().start = u32::MAX,
            2 => session.music.playing = Some((u16::MAX, 0)),
            3 => {
                session.music.playing = None;
                session.music.pending = true;
            }
            4 => Arc::make_mut(session.music.clip.as_mut().unwrap()).rate = 0,
            _ => unreachable!(),
        }
        let invalid = session.capture_state().unwrap();
        session.restore_state(&saved).unwrap();
        assert!(
            session.restore_state(&invalid).is_err(),
            "invalid music case {case}"
        );
        assert_eq!(session.capture_state().unwrap(), saved);
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; tutorial narration and death music retain the latest shared XA request"]
fn native_music_and_training_resume_the_latest_cue() {
    let mut playback = start(1);
    let disc = disc();
    let current = session(&mut playback);
    let training = current.world.tutorial.as_ref().unwrap();
    let lesson = &training.data.lessons[training.data.starts[0]];
    let point = lesson.position.map(|v| i32::from(v) * 1000);
    let cue = lesson.cue;
    current.world.tutorial.as_mut().unwrap().playing = Some((cue, current.world.tick));
    current.frame.tutorial.cue = Some(cue);
    current.training_step();
    current
        .training
        .as_mut()
        .unwrap()
        .audio(&disc, current.world.tutorial.as_ref().unwrap())
        .unwrap()
        .unwrap();
    let event = death_event(current, false);
    current.music.step(&current.world, event);
    let death = current.music.audio(&disc).unwrap().unwrap();
    let (output, receiver) = Output::recording();
    playback.content.resume_audio(&output);
    let last = receiver
        .try_iter()
        .filter_map(|command| match command {
            Command::Music(clip, _) => clip,
            _ => None,
        })
        .last()
        .unwrap();
    assert!(Arc::ptr_eq(&last, &death));
    let current = session(&mut playback);
    current.world.actors[current.world.player].actor.position = point.into();
    let training = current.world.tutorial.as_mut().unwrap();
    training.state.lesson = Some(training.data.starts[0]);
    training.state.spoken = None;
    training.playing = None;
    let mut narration = None;
    for _ in 0..5 {
        narration = playback.step(0).unwrap().music;
        if narration.is_some() {
            break;
        }
    }
    let narration = narration.unwrap();
    assert!(session(&mut playback).music.playing.is_none());
    playback.content.resume_audio(&output);
    let last = receiver
        .try_iter()
        .filter_map(|command| match command {
            Command::Music(clip, _) => clip,
            _ => None,
        })
        .last()
        .unwrap();
    assert!(Arc::ptr_eq(&last, &narration));
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; story-to-battle audio, checkpoint/rewind playhead and return to story"]
fn native_boss_music_survives_story_transitions_and_restores() {
    let mut playback = start(10);
    assert!(matches!(playback.content, Content::Story { .. }));
    for _ in 0..60 {
        playback.step(0).unwrap();
    }
    playback.action = Some(Action::Continue);
    for _ in 0..6000 {
        playback.step(0).unwrap();
        if playback.run.sequence != 10 {
            break;
        }
    }
    assert_eq!(playback.run.sequence, 11);
    let (output, receiver) = Output::recording();
    playback.content.start_audio(&output);
    assert!(
        receiver
            .try_iter()
            .any(|command| matches!(command, Command::Music(None, false)))
    );
    assert!(playback.step(0).unwrap().music.is_some());
    for _ in 0..11 {
        assert!(playback.step(0).unwrap().music.is_none());
    }
    let saved = playback.capture_state().unwrap();
    let mut recorder = crate::rewind::Recorder::default();
    recorder.begin();
    session(&mut playback)
        .snapshot()
        .capture(&mut recorder)
        .unwrap();
    assert_eq!(
        recorder.finish().unwrap().bytes().unwrap(),
        postcard::to_allocvec(&session(&mut playback).snapshot()).unwrap()
    );
    let mut restored = start(9);
    for _ in 0..2 {
        restored.restore_state(&saved).unwrap();
        assert_eq!(restored.capture_state().unwrap(), saved);
        restored.content.resume_audio(&output);
        let commands: Vec<_> = receiver.try_iter().collect();
        assert_eq!(
            commands
                .iter()
                .filter(|command| matches!(command, Command::Music(Some(_), false)))
                .count(),
            1
        );
        assert!(commands.iter().any(|command| matches!(command, Command::SeekMusic(seconds) if (*seconds - 11.0 / 30.0).abs() < f64::EPSILON)));
    }
    for _ in 0..12 {
        assert_eq!(
            playback.step(0).unwrap().music.is_some(),
            restored.step(0).unwrap().music.is_some()
        );
    }
    assert_eq!(
        playback.capture_state().unwrap(),
        restored.capture_state().unwrap()
    );
    let session = session(&mut playback);
    session.world.mission.phase = Phase::Finished {
        outcome: Outcome::TargetsDefeated,
        finished_at: session.world.tick,
    };
    assert!(playback.step(0).unwrap().music.is_none());
    assert!(matches!(playback.content, Content::Story { .. }));
    playback.content.start_audio(&output);
    assert!(matches!(
        receiver.try_iter().nth(1),
        Some(Command::Music(None, false))
    ));
}
