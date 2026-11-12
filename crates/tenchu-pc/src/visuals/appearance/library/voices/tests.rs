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
                assert!(
                    postcard::to_allocvec(sound.bank.as_ref().unwrap()).unwrap()
                        == postcard::to_allocvec(&expected.bank).unwrap()
                );
            }
            assert!(Arc::ptr_eq(
                sounds[4].bank.as_ref().unwrap(),
                &library.voice("tenchu2-jp:ayame").unwrap().unwrap().bank
            ));
            assert_eq!(game.random, rng);
            let restored: tenchu_core::game::Game =
                postcard::from_bytes(&postcard::to_allocvec(&game).unwrap()).unwrap();
            for sound in &restored.audio_with_burning([], None).unwrap()[..4] {
                assert!(
                    postcard::to_allocvec(sound.bank.as_ref().unwrap()).unwrap()
                        == postcard::to_allocvec(&expected.bank).unwrap()
                );
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
