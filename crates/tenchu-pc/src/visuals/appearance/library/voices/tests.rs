use super::*;
use crate::settings::Settings;

#[test]
#[ignore = "requires both discs; T1 language and explicit voice choices override T2 moveset vocals through saved continuation"]
fn localized_voices_override_imported_movesets_and_follow_language() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    for (id, _) in library.voice_choices().into_iter().skip(1) {
        assert!(library.voice(&id).unwrap().is_some(), "{id}");
    }
    let mut game = crate::session::Options {
        character: 1,
        ..Default::default()
    }
    .load(&assets)
    .unwrap();
    game.events = Default::default();
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    let mut settings = Settings {
        model_moveset: true,
        moveset_model: "tenchu2-jp:ayame".into(),
        voice_model: "disc:1".into(),
        ..Default::default()
    };
    let english = VoiceSet::extract(
        &Bank::load(&assets, 0, 1, Language::English).unwrap(),
        5,
        Dialect::Gaisen,
    )
    .unwrap();
    let japanese = VoiceSet::extract(
        &Bank::load(&assets, 0, 1, Language::Japanese).unwrap(),
        5,
        Dialect::Gaisen,
    )
    .unwrap();
    assert!(postcard::to_allocvec(&english).unwrap() != postcard::to_allocvec(&japanese).unwrap());
    for (language, _, _) in LANGUAGES {
        game.mission.scripts.language = language;
        let expected = VoiceSet::extract(
            &Bank::load(&assets, 0, 1, language).unwrap(),
            5,
            Dialect::Gaisen,
        )
        .unwrap();
        for (selected, expected) in [("disc:1", &expected), ("voice:en:disc:1", &english)] {
            settings.voice_model = selected.into();
            library.apply_player(&mut game, &settings).unwrap();
            assert!(!library.apply_player(&mut game, &settings).unwrap());
            let rng = game.random;
            game.intelligence.voices = [9, 6, 8, 15, 0]
                .map(|cue| game.voice_event(game.player_id, cue))
                .to_vec();
            let sounds = game.audio_with_burning([], None).unwrap();
            assert_eq!(sounds.len(), 5);
            for sound in &sounds[..4] {
                assert_source_voice(sound.bank.as_ref().unwrap(), &expected.bank);
            }
            assert!(Arc::ptr_eq(
                sounds[4].bank.as_ref().unwrap(),
                &library.voice("tenchu2-jp:ayame").unwrap().unwrap().bank
            ));
            assert_eq!(game.random, rng);
            let restored: tenchu_core::game::Game =
                postcard::from_bytes(&postcard::to_allocvec(&game).unwrap()).unwrap();
            for sound in &restored.audio_with_burning([], None).unwrap()[..4] {
                assert_source_voice(sound.bank.as_ref().unwrap(), &expected.bank);
            }
        }
    }
    settings.voice_model.clear();
    library.apply_player(&mut game, &settings).unwrap();
    game.intelligence.voices = vec![game.voice_event(game.player_id, 9)];
    assert!(Arc::ptr_eq(
        game.audio_with_burning([], None).unwrap()[0]
            .bank
            .as_ref()
            .unwrap(),
        &library.voice("tenchu2-jp:ayame").unwrap().unwrap().bank
    ));
    settings.moveset_model = "disc:1".into();
    for language in [Language::English, Language::Japanese, Language::English] {
        game.mission.scripts.language = language;
        library.apply_player(&mut game, &settings).unwrap();
        game.intelligence.voices = vec![game.voice_event(game.player_id, 9)];
        assert!(Arc::ptr_eq(
            game.audio_with_burning([], None).unwrap()[0]
                .bank
                .as_ref()
                .unwrap(),
            &library.voice_in("disc:1", language).unwrap().unwrap().bank
        ));
    }
}

fn assert_source_voice(actual: &Bank, expected: &Bank) {
    let mut original: Bank = postcard::from_bytes(&postcard::to_allocvec(actual).unwrap()).unwrap();
    original.programs.truncate(expected.programs.len());
    original.samples.truncate(expected.samples.len());
    assert_eq!(
        postcard::to_allocvec(&original).unwrap(),
        postcard::to_allocvec(expected).unwrap()
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; jump, fatal-fall and item vocals follow T1 voice selection without replacing world or NPC sounds"]
fn selected_hero_voice_includes_global_player_vocals() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let library = Library::load(&assets).unwrap();
    for hero in 0..2 {
        let mut game = crate::session::Options {
            character: hero,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let other = game
            .roster
            .order
            .iter()
            .copied()
            .find(|&id| id != game.player_id)
            .unwrap();
        for explicit in [false, true] {
            for language in [Language::English, Language::Japanese] {
                game.mission.scripts.language = language;
                let settings = Settings {
                    model_moveset: !explicit,
                    moveset_model: format!("disc:{}", 1 - hero),
                    voice_model: if explicit {
                        format!("disc:{}", 1 - hero)
                    } else {
                        String::new()
                    },
                    ..Default::default()
                };
                library.apply_player(&mut game, &settings).unwrap();
                let expected = Bank::load(&assets, 0, 1 - hero, language).unwrap();
                let mut npc = game.voice_event(other, 0x48);
                npc.position = game.player().position;
                game.intelligence.voices = vec![
                    game.voice_event(game.player_id, 0x48),
                    game.voice_event(game.player_id, 0x49),
                    npc,
                ];
                let effects = [
                    (game.player().position, 0x4c),
                    (game.player().position, 0x47),
                ];
                let rng = game.random;
                for host in [
                    &game,
                    &postcard::from_bytes::<tenchu_core::game::Game>(
                        &postcard::to_allocvec(&game).unwrap(),
                    )
                    .unwrap(),
                ] {
                    let sounds = host.audio_with_burning(effects, None).unwrap();
                    assert_eq!(sounds.len(), 5);
                    for (index, id) in [(0, 0x48usize), (1, 0x49), (3, 0x4c)] {
                        let bank = sounds[index]
                            .bank
                            .as_ref()
                            .unwrap_or_else(|| panic!("selected hero {hero}, explicit={explicit}, {language:?}, cue {id:#x} must carry its source bank"));
                        let tone = &bank.programs[id / 16].as_ref().unwrap().tones[id % 16];
                        let expected_tone =
                            &expected.programs[id / 16].as_ref().unwrap().tones[id % 16];
                        assert_eq!(sounds[index].sound.id as usize, id);
                        assert_eq!(
                            bank.samples[tone.sample].frames,
                            expected.samples[expected_tone.sample].frames
                        );
                    }
                    assert!(sounds[2].bank.is_none(), "NPC sounds keep their source");
                    assert!(
                        sounds[4].bank.is_none(),
                        "world sounds keep the mission bank"
                    );
                }
                assert_eq!(game.random, rng);
            }
        }
        library
            .apply_player(&mut game, &Settings::default())
            .unwrap();
        game.intelligence.voices = vec![game.voice_event(game.player_id, 0x48)];
        assert!(
            game.audio_with_burning([(game.player().position, 0x4c)], None)
                .unwrap()
                .iter()
                .all(|s| s.bank.is_none())
        );
    }
}
