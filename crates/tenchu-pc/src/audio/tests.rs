use super::*;
use tenchu_core::{actor::State, ids::MOT_DEAD, math::Position};

mod cultist;

#[test]
#[ignore = "requires TENCHU_DISC; ordinary deaths retain music and boss failure cutscenes restore the last mission track"]
fn death_observation_preserves_audio_until_confirmation() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    for boss in [false, true] {
        let (mut audio, commands) = recording(disc.as_ref(), &assets);
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.step(0).unwrap();
        audio.tick(&game, None, Default::default(), []).unwrap();
        if boss {
            game.teleport_player(Position::new(1000, 0, 5000), false)
                .unwrap();
            game.step(0).unwrap();
            audio.tick(&game, None, Default::default(), []).unwrap();
            assert!(game.cinematic.active);
            for _ in 0..5000 {
                if !game.cinematic.active {
                    break;
                }
                game.step(0).unwrap();
                audio.tick(&game, None, Default::default(), []).unwrap();
            }
            assert!(!game.cinematic.active);
        }
        for _ in 0..30 {
            game.step(0).unwrap();
            audio.tick(&game, None, Default::default(), []).unwrap();
        }
        let music = audio.current_track.clone().unwrap();
        assert!(music.repeat);
        let player = game.roster.get_mut(game.player_id);
        player.health = 0;
        player.status = State::Dead;
        player.motion.change(
            MOT_DEAD,
            &game.mission.characters[player.definition].motions,
            &game.mission.common_motions,
            &game.mission.clips,
        );
        for _ in 0..5000 {
            game.step(0).unwrap();
            audio.tick(&game, None, Default::default(), []).unwrap();
            if game.death_wait_ticks() == Some(1) {
                break;
            }
        }
        assert_eq!(game.death_wait_ticks(), Some(1));
        assert_eq!(audio.current_track.is_none(), boss);
        commands.try_iter().for_each(drop);
        let hold = Screen::DeathHold {
            retry_seed: game.random.seed,
        };
        audio.screen(&hold);
        let entry: Vec<_> = commands.try_iter().collect();
        if boss {
            assert!(matches!(entry.as_slice(), [Command::Music(None, false)]));
        } else {
            assert!(entry.is_empty(), "Ordinary death restarted audio");
        }
        assert_eq!(audio.current_track, Some(music.clone()));
        for _ in 0..600 {
            game.observe_death().unwrap();
            audio.tick(&game, None, Default::default(), []).unwrap();
            audio.screen(&hold);
            assert!(commands.try_iter().all(|c| matches!(c, Command::Effect(_))));
        }
        assert_eq!(audio.current_track, Some(music));
        audio.fast_forward(true);
        commands.try_iter().for_each(drop);
        audio.screen(&hold);
        assert!(audio.fast_forward);
        assert!(commands.try_recv().is_err());
        audio.fast_forward(false);
        commands.try_iter().for_each(drop);
        audio.pause(true, false);
        assert!(matches!(
            commands.try_recv().unwrap(),
            Command::Pause {
                paused: true,
                music_while_paused: false
            }
        ));
        audio.pause(false, false);
        assert!(matches!(
            commands.try_recv().unwrap(),
            Command::Pause { paused: false, .. }
        ));
        audio.screen(&Screen::GameOver);
        assert!(matches!(commands.try_recv().unwrap(), Command::Clear));
        assert!(matches!(
            commands.try_recv().unwrap(),
            Command::Music(None, false)
        ));
        assert_eq!(audio.current_track, audio.catalog.music(11).cloned());
        let mut campaign = Campaign::new(&assets, Default::default(), None).unwrap();
        campaign.game = game;
        audio.reload(&assets, &campaign).unwrap();
        assert!(audio.mission_track.is_none());
    }
}

fn recording(
    disc: &Path,
    assets: &GameAssets<DiscSource>,
) -> (Audio, std::sync::mpsc::Receiver<Command>) {
    let (output, commands) = output::Output::recording();
    let audio = Audio {
        output,
        loader: loader::Loader::new(disc).unwrap(),
        catalog: Catalog::load(&assets.executable, Default::default()).unwrap(),
        sandbox_music: None,
        screen: Some(discriminant(&Screen::Playing)),
        paused: false,
        music_while_paused: false,
        trace: false,
        fast_forward: false,
        volume: 100,
        mission_track: None,
        current_track: None,
        ready_track: None,
        playhead: 0.0,
        playhead_updated: Instant::now(),
    };
    (audio, commands)
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; real XA decoding, sandbox playback, pause, save, retry and Gaisen transitions"]
fn tenchu2_music_uses_its_disc_and_survives_playback_transitions() {
    use crate::{campaign::Action, tenchu2::Sandbox};
    use tenchu_assets::audio::stream::Clip;
    let gaisen = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let t2 = std::env::var_os("TENCHU2_DISC").expect("Set TENCHU2_DISC");
    let assets = GameAssets::open(crate::source::open_disc(gaisen.as_ref()).unwrap()).unwrap();
    let disc = crate::source::open_disc(t2.as_ref()).unwrap();
    let sandbox = Sandbox::open(t2.as_ref(), Some(0), None, None).unwrap();
    assert_eq!(
        sandbox
            .music
            .tracks
            .iter()
            .map(|t| t.channel)
            .collect::<Vec<_>>(),
        [0, 2, 1, 3, 5, 6, 4, 7]
    );
    let mut campaign = Campaign::new(&assets, Default::default(), None).unwrap();
    campaign.start_sandbox(sandbox, &assets).unwrap();
    let (mut audio, commands) = recording(gaisen.as_ref(), &assets);
    audio.reload(&assets, &campaign).unwrap();
    audio.screen(&campaign.screen);
    assert!(
        audio.current_track.is_none(),
        "Default must not repeat a menu song"
    );
    assert!(
        commands
            .try_iter()
            .any(|c| matches!(c, Command::Tenchu2Ambience(_)))
    );
    let wait = |audio: &mut Audio| {
        let deadline = Instant::now() + std::time::Duration::from_secs(15);
        loop {
            audio.poll();
            for command in commands.try_iter() {
                if let Command::Music(Some(clip), repeat) = command {
                    assert!(repeat);
                    return clip;
                }
            }
            assert!(Instant::now() < deadline, "Music failed to load");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    };
    let mut signatures = std::collections::BTreeSet::new();
    for selected in 0..8 {
        campaign.sandbox.as_mut().unwrap().music.selected = Some(selected);
        audio.reload(&assets, &campaign).unwrap();
        audio.screen(&campaign.screen);
        let track = audio.current_track.clone().unwrap();
        let expected = Clip::load(&disc, &track).unwrap();
        let clip = wait(&mut audio);
        assert_eq!(clip.frames, expected.frames);
        assert_eq!(clip.rate, expected.rate);
        let duration = clip.frames.len() as f64 / f64::from(clip.rate);
        assert!(
            (50.0..100.0).contains(&duration),
            "Unexpected duration {duration}"
        );
        let peak = clip
            .frames
            .iter()
            .flatten()
            .map(|n| n.unsigned_abs())
            .max()
            .unwrap();
        assert!(peak > 1000, "Silent theme");
        let signature: i64 = clip
            .frames
            .iter()
            .flatten()
            .step_by(137)
            .map(|n| i64::from(*n))
            .sum();
        assert!(signatures.insert(signature), "Duplicate theme");
        eprintln!(
            "T2 theme {}: {:.2}s, {} Hz, peak {peak}",
            selected + 1,
            duration,
            clip.rate
        );
        for _ in 0..60 {
            campaign.game.step(tenchu_core::input::UP).unwrap();
            audio
                .tick(&campaign.game, None, Default::default(), [])
                .unwrap();
            audio.screen(&campaign.screen);
        }
        assert_eq!(audio.current_track.as_ref(), Some(&track));
        audio.pause(true, false);
        let state = audio.capture_state();
        assert_eq!(state.playhead, audio.capture_state().playhead);
        let mut state = state;
        state.playhead = 15.0;
        let bytes = postcard::to_allocvec(&state).unwrap();
        audio.restore_playback(&campaign, Some(postcard::from_bytes(&bytes).unwrap()));
        wait(&mut audio);
        assert_eq!(audio.capture_state().playhead, 15.0);
        audio.pause(false, false);
        audio.fast_forward(true);
        audio.fast_forward(false);
        assert_eq!(audio.current_track.as_ref(), Some(&track));
        wait(&mut audio);
        campaign.action(Action::Replay, &assets).unwrap();
        audio.reload(&assets, &campaign).unwrap();
        audio.screen(&campaign.screen);
        wait(&mut audio);
        assert_eq!(audio.current_track.as_ref(), Some(&track));
    }
    campaign.sandbox.as_mut().unwrap().music.selected = None;
    audio.reload(&assets, &campaign).unwrap();
    audio.screen(&campaign.screen);
    assert!(audio.current_track.is_none());
    assert!(audio.mission_track.is_none());
    let sent: Vec<_> = commands.try_iter().collect();
    assert!(
        sent.iter()
            .all(|c| !matches!(c, Command::Music(Some(_), _)))
    );
    assert!(
        sent.iter()
            .any(|c| matches!(c, Command::Tenchu2Ambience(_)))
    );
    audio.restore_playback(&campaign, None);
    assert!(
        commands
            .try_iter()
            .any(|c| matches!(c, Command::Tenchu2Ambience(_)))
    );
    audio.fast_forward(true);
    assert!(
        commands
            .try_iter()
            .all(|c| !matches!(c, Command::Tenchu2Ambience(_)))
    );
    audio.fast_forward(false);
    assert!(
        commands
            .try_iter()
            .any(|c| matches!(c, Command::Tenchu2Ambience(_)))
    );
    campaign.action(Action::Back, &assets).unwrap();
    audio.reload(&assets, &campaign).unwrap();
    audio.screen(&campaign.screen);
    assert!(audio.sandbox_music.is_none());
    assert_eq!(audio.current_track, audio.catalog.music(15).cloned());
    wait(&mut audio);
}
