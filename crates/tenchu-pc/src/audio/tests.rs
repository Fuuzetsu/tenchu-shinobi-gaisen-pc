use super::*;
use tenchu_core::{actor::State, ids::MOT_DEAD, math::Position};

#[test]
#[ignore = "requires TENCHU_DISC; ordinary deaths retain music and boss failure cutscenes restore the last mission track"]
fn death_observation_preserves_audio_until_confirmation() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    for boss in [false, true] {
        let (output, commands) = output::Output::recording();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        let mut audio = Audio {
            output,
            loader: loader::Loader::new(disc.as_ref()).unwrap(),
            catalog: Catalog::load(&assets.executable, Default::default()).unwrap(),
            screen: Some(discriminant(&Screen::Playing)),
            paused: false,
            music_while_paused: false,
            trace: false,
            fast_forward: false,
            volume: 100,
            mission_track: None,
            current_track: None,
            ready_track: None,
        };
        game.step(0).unwrap();
        audio.tick(&game).unwrap();
        if boss {
            game.teleport_player(Position::new(1000, 0, 5000), false)
                .unwrap();
            game.step(0).unwrap();
            audio.tick(&game).unwrap();
            assert!(game.cinematic.active);
            for _ in 0..5000 {
                if !game.cinematic.active {
                    break;
                }
                game.step(0).unwrap();
                audio.tick(&game).unwrap();
            }
            assert!(!game.cinematic.active);
        }
        for _ in 0..30 {
            game.step(0).unwrap();
            audio.tick(&game).unwrap();
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
            audio.tick(&game).unwrap();
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
            audio.tick(&game).unwrap();
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
        audio.reload(&assets, &game).unwrap();
        assert!(audio.mission_track.is_none());
    }
}
