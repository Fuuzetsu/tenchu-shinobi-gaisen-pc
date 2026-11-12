use super::*;
use tenchu_core::{
    game::moveset::{SELECTED_VOICE, SELECTED_WEAPON_VOICE},
    ids::*,
};

#[test]
#[ignore = "requires both discs; independent equipment/voice selection preserves moves, controls and save/rewind resources"]
fn independent_equipment_and_voice_in_gaisen() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
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
        for moves in [
            "",
            "tenchu2-jp:rikimaru",
            "tenchu2-jp:ayame",
            "tenchu2-jp:tatumaru",
        ] {
            let mut settings = Settings {
                model_moveset: !moves.is_empty(),
                moveset_model: moves.into(),
                ..Default::default()
            };
            library.apply_player(&mut game, &settings).unwrap();
            let definition =
                postcard::to_allocvec(&game.mission.characters[game.player().definition]).unwrap();
            let attacks = postcard::to_allocvec(&game.mission.weapons.imported_attacks).unwrap();
            let clip = game.player().motion.clip;
            let controls = game.player().circle_guards(&game.intelligence);
            let health = game.player().health;
            let rng = game.random;
            for selected in [
                "tenchu2-jp:kagami",
                "tenchu2-jp:yorojyu",
                "tenchu2-jp:tatu2",
                "disc:1",
            ] {
                settings.weapon_model = selected.into();
                settings.voice_model = "tenchu2-jp:ayame".into();
                assert!(library.apply_player(&mut game, &settings).unwrap());
                assert!(!library.apply_player(&mut game, &settings).unwrap());
                assert_eq!(game.player().motion.clip, clip);
                assert_eq!(game.player().health, health);
                assert_eq!(game.random, rng);
                assert_eq!(game.player().circle_guards(&game.intelligence), controls);
                assert_eq!(controls, !moves.is_empty());
                assert!(
                    definition
                        == postcard::to_allocvec(
                            &game.mission.characters[game.player().definition]
                        )
                        .unwrap()
                );
                assert!(
                    attacks
                        == postcard::to_allocvec(&game.mission.weapons.imported_attacks).unwrap()
                );
                let source = library.weapon(selected).unwrap().unwrap();
                let equipment = source.equipment.as_ref().unwrap();
                assert!(
                    !equipment.attachments.is_empty(),
                    "{selected}: no weapon meshes"
                );
                let actual = game
                    .player_equipment(&Arc::new(game.mission.vram.clone()))
                    .unwrap();
                for a in &actual.attachments {
                    let b = equipment
                        .attachments
                        .iter()
                        .find(|b| b.slot == a.slot)
                        .unwrap();
                    assert_eq!(
                        (a.part, a.slot, a.position, &a.model.id),
                        (b.part, b.slot, b.position, &b.model.id)
                    );
                }
                assert_eq!(actual.attachments.len(), equipment.attachments.len());
                assert!(Arc::ptr_eq(
                    &game.mission.moveset_voices[&SELECTED_VOICE],
                    library.voice(&settings.voice_model).unwrap().unwrap()
                ));
                assert!(Arc::ptr_eq(
                    &game.mission.moveset_voices[&SELECTED_WEAPON_VOICE],
                    source.moveset_voice().unwrap()
                ));
                game.intelligence.voices = vec![
                    game.voice_event(game.player_id, 2),
                    game.voice_event(game.player_id, 9),
                ];
                let sounds = game.audio_with_burning([], None).unwrap();
                assert_eq!(sounds.len(), 2);
                assert!(Arc::ptr_eq(
                    sounds[0].bank.as_ref().unwrap(),
                    &source.moveset_voice().unwrap().bank
                ));
                assert!(Arc::ptr_eq(
                    sounds[1].bank.as_ref().unwrap(),
                    &library.voice(&settings.voice_model).unwrap().unwrap().bank
                ));
                let bytes = postcard::to_allocvec(&game).unwrap();
                let restored: Game = postcard::from_bytes(&bytes).unwrap();
                assert_eq!(restored.moveset.weapon_model, selected);
                assert_eq!(restored.moveset.voice_model, settings.voice_model);
                assert!(
                    restored
                        .audio_with_burning([], None)
                        .unwrap()
                        .iter()
                        .all(|s| s.bank.is_some())
                );
                assert_eq!(
                    game.player().attribute & ATTR_WEAPON_DRAWN,
                    restored.player().attribute & ATTR_WEAPON_DRAWN
                );
            }
            settings.weapon_model.clear();
            settings.voice_model.clear();
            assert!(library.apply_player(&mut game, &settings).unwrap());
            assert!(!game.mission.moveset_voices.contains_key(&SELECTED_VOICE));
            assert!(
                !game
                    .mission
                    .moveset_voices
                    .contains_key(&SELECTED_WEAPON_VOICE)
            );
            assert_eq!(game.player().circle_guards(&game.intelligence), controls);
        }
    }
}
