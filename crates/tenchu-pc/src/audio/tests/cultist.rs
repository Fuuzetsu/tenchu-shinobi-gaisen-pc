use super::*;
use sha2::{Digest, Sha256};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_CULTIST_REPLAY; recorded Mission 6 sound positions, save/rewind and original channel reuse"]
fn recorded_cultist_death_retains_retail_sound_sequence() {
    let path = std::env::var_os("TENCHU_CULTIST_REPLAY").expect("Set TENCHU_CULTIST_REPLAY");
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "0683ad09558bf97cfe03fab6cd42aed028fba6a44242cc737ca168d73fe3cd56"
    );
    let tape: crate::retail::tape::Tape = serde_json::from_slice(&bytes).unwrap();
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let (mut audio, commands) = recording(disc.as_ref(), &assets);
    assert_eq!(tape.missions.len(), 1);
    let mission = &tape.missions[0];
    assert_eq!(mission.updates.len(), 359);
    let mut game = mission.initial.load(&assets).unwrap();
    let mut events = Vec::new();
    let mut digest = Sha256::new();
    for update in &mission.updates {
        update.apply(&mut game).unwrap();
        if game.clock == 38 {
            spatial_event_survives_restore(&game);
        }
        audio.tick(&game, None, Default::default(), []).unwrap();
        for command in commands.try_iter() {
            if let Command::Effect(event) = command {
                digest.update(game.clock.to_le_bytes());
                digest.update(event.sound.id.to_le_bytes());
                digest.update([event.sound.volume, event.sound.pan]);
                events.push((game.clock, event.sound));
            }
        }
    }
    assert_eq!(events.len(), 55);
    assert_eq!(
        format!("{:x}", digest.finalize()),
        "6bbd5a3043864b42e56eae773530f551a0a1106ebcd600f48ad9577f1ba1bd92"
    );
    let scream = events
        .iter()
        .position(|(_, sound)| sound.id == 0x68)
        .unwrap();
    assert_eq!(events[scream], (65, Sound::local(0x68)));
    let replacement = events[scream + 24];
    assert_eq!(replacement.0, 149);
    assert_eq!(replacement.1.id, 0x44);
}

fn spatial_event_survives_restore(game: &tenchu_core::game::Game) {
    use tenchu_core::{audio::Listener, camera::Mode};

    let voice = game
        .intelligence
        .voices
        .iter()
        .find(|v| v.id == 0x10)
        .unwrap();
    assert_eq!(voice.position, Position::new(-792, 4830, -35202));
    assert_eq!(
        voice.listener,
        Listener {
            position: Position::new(2698, 7300, -44583),
            yaw: 2221,
            direction: None,
        }
    );
    assert_eq!(game.player().position, Position::new(2829, 7060, -44101));
    let expected = [Sound {
        id: 0x10,
        volume: 41,
        pan: 39,
    }];
    assert_eq!(game.sounds_with_effects([]).unwrap(), expected);

    let saved = crate::savestate::encode(game).unwrap();
    let mut recorder = crate::rewind::Recorder::default();
    game.capture(&mut recorder).unwrap();
    let snapshot = recorder.finish().unwrap();
    for mut restored in [
        crate::savestate::decode::<tenchu_core::game::Game>(&saved).unwrap(),
        snapshot.decode().unwrap(),
    ] {
        assert_eq!(restored.intelligence.voices, game.intelligence.voices);
        assert_eq!(restored.sounds_with_effects([]).unwrap(), expected);
        restored.roster.get_mut(voice.actor).position = Position::new(100000, 0, 0);
        let player = restored.roster.get_mut(restored.player_id);
        player.position = Position::new(-100000, 0, 0);
        player.rotation[1] = 1024;
        restored.camera.mode = Mode::Direction;
        restored.camera.direction[1] = -1024;
        assert_eq!(restored.sounds_with_effects([]).unwrap(), expected);
        let listener = Listener {
            position: voice.position,
            yaw: 0,
            direction: None,
        };
        let sounds: Vec<_> = restored
            .audio_with_listener(listener, [], None)
            .unwrap()
            .into_iter()
            .map(|event| event.sound)
            .collect();
        assert_eq!(sounds, [Sound::local(0x10)]);
    }
}
