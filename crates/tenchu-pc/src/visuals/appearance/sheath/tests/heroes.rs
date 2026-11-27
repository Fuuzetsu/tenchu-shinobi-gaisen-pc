use super::*;
use tenchu_core::{ids::*, input};

#[test]
#[ignore = "requires TENCHU_DISC; cross-hero sheath clips, retained movement choice and saved unarmed state"]
fn cross_hero_sheathing_follows_the_selected_moveset() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let library = Library::load(&assets).unwrap();
    for hero in 0..2 {
        for configure_first in [true, false] {
            let mut game = crate::session::Options {
                character: hero,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            game.intelligence.ignore_player = true;
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let mut settings = Settings {
                player_model: format!("disc:{}", 1 - hero),
                model_moveset: true,
                moveset_model: format!("disc:{}", 1 - hero),
                weapon_model: format!("disc:{}", 1 - hero),
                ..Default::default()
            };
            settings.model_moveset_options.player_movement = false;
            settings.sheathing.enabled = true;
            settings.sheathing.unarmed = true;
            if configure_first {
                crate::sheathing::configure(&mut game, &assets, settings.sheathing).unwrap();
            }
            library.apply_player(&mut game, &settings).unwrap();
            if !configure_first {
                crate::sheathing::configure(&mut game, &assets, settings.sheathing).unwrap();
            }
            let unarmed = serde_json::to_value(game.player().sheathing.as_ref().unwrap()).unwrap();
            let donor = library
                .get(if hero == 0 { "disc:34" } else { "disc:33" })
                .unwrap();
            let expected = |action| {
                let clip = donor
                    .character
                    .as_ref()
                    .unwrap()
                    .motions
                    .iter()
                    .find(|r| r.action == action)
                    .unwrap()
                    .clip;
                donor.clips.iter().find(|c| c.id == clip).unwrap()
            };
            toggle(&mut game);
            for _ in 0..10 {
                game.step(0).unwrap();
            }
            if hero == 0 && configure_first {
                capture_selected(&game, &library, &settings, "ayame-sheathing");
            }
            assert_eq!(
                &game.mission.clips[game.player().motion.clip.unwrap()],
                expected(MOT_STATE_SHEATHE)
            );
            for _ in 0..100 {
                game.step(0).unwrap();
            }
            assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
            assert_eq!(
                &game.mission.clips[game.player().motion.clip.unwrap()],
                expected(MOT_NORMAL)
            );
            game.step(input::LEFT).unwrap();
            assert_eq!(
                &game.mission.clips[game.player().motion.clip.unwrap()],
                expected(MOT_NORMAL_TURN_L)
            );
            for _ in 0..40 {
                game.step(0).unwrap();
            }
            // Refresh a fallback already in use when the menu switches heroes.
            settings.model_moveset = false;
            library.apply_player(&mut game, &settings).unwrap();
            assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
            game.step(input::LEFT).unwrap();
            let original = library
                .get(if hero == 0 { "disc:33" } else { "disc:34" })
                .unwrap();
            let original_turn = original
                .character
                .as_ref()
                .unwrap()
                .motions
                .iter()
                .find(|r| r.action == MOT_NORMAL_TURN_L)
                .unwrap()
                .clip;
            assert_eq!(
                game.mission.clips[game.player().motion.clip.unwrap()].id,
                original_turn
            );
            settings.model_moveset = true;
            library.apply_player(&mut game, &settings).unwrap();
            game.step(input::LEFT).unwrap();
            assert_eq!(
                &game.mission.clips[game.player().motion.clip.unwrap()],
                expected(MOT_NORMAL_TURN_L)
            );
            for _ in 0..40 {
                game.step(0).unwrap();
            }
            for player_movement in [false, true] {
                settings.model_moveset_options.player_movement = player_movement;
                library.apply_player(&mut game, &settings).unwrap();
                for _ in 0..12 {
                    game.step(input::UP).unwrap();
                }
                let body = library
                    .get(&format!(
                        "disc:{}",
                        if player_movement { hero } else { 1 - hero }
                    ))
                    .unwrap();
                let clip = body
                    .character
                    .as_ref()
                    .unwrap()
                    .motions
                    .iter()
                    .find(|r| r.action == MOT_CHASE)
                    .unwrap()
                    .clip;
                assert_eq!(
                    &game.mission.clips[game.player().motion.clip.unwrap()],
                    body.clips.iter().find(|c| c.id == clip).unwrap()
                );
                for _ in 0..40 {
                    game.step(0).unwrap();
                }
            }
            let saved = crate::savestate::encode(&game).unwrap();
            let mut restored: Game = crate::savestate::decode(&saved).unwrap();
            for host in [&mut game, &mut restored] {
                toggle(host);
                host.step(0).unwrap();
                assert_eq!(
                    &host.mission.clips[host.player().motion.clip.unwrap()],
                    expected(MOT_STATE_DRAW)
                );
                for _ in 0..100 {
                    host.step(0).unwrap();
                }
            }
            assert_eq!(
                crate::savestate::encode(&game).unwrap(),
                crate::savestate::encode(&restored).unwrap()
            );
            settings.model_moveset = false;
            library.apply_player(&mut game, &settings).unwrap();
            toggle(&mut game);
            game.step(0).unwrap();
            let original = library
                .get(if hero == 0 { "disc:33" } else { "disc:34" })
                .unwrap();
            let id = original
                .character
                .as_ref()
                .unwrap()
                .motions
                .iter()
                .find(|r| r.action == MOT_STATE_SHEATHE)
                .unwrap()
                .clip;
            assert_eq!(
                &game.mission.clips[game.player().motion.clip.unwrap()],
                original.clips.iter().find(|c| c.id == id).unwrap()
            );
            let after = serde_json::to_value(game.player().sheathing.as_ref().unwrap()).unwrap();
            for key in ["settings", "attacks", "loaded_style"] {
                assert_eq!(unarmed[key], after[key]);
            }
            assert_eq!(game.player().kind, hero);
        }
    }
}

fn toggle(game: &mut Game) {
    game.roster
        .get_mut(game.player_id)
        .sheathing
        .as_mut()
        .unwrap()
        .input(true);
    game.step(0).unwrap();
    game.roster
        .get_mut(game.player_id)
        .sheathing
        .as_mut()
        .unwrap()
        .input(false);
}

fn capture_selected(game: &Game, library: &Library, settings: &Settings, name: &str) {
    if std::env::var_os("TENCHU_SHEATH_REVIEW").is_none() {
        return;
    }
    let pack = Pack::default();
    let mut graphics = Graphics::default();
    graphics.set_preset(0);
    graphics.asset_pack = false;
    graphics.fog = false;
    let mut models = Models::build(&game.mission, &pack, &graphics)
        .unwrap()
        .with_player(
            game,
            &settings.player_model,
            &pack,
            &pack,
            &graphics,
            Some(library),
        )
        .unwrap();
    capture(game, &mut models, &graphics, name);
}
