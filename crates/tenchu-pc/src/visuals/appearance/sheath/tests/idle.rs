use super::*;
use tenchu_core::ids::*;

#[test]
#[ignore = "requires both discs; T2 boss bind poses fall back to the host sheathed idle while authored NPC idles and armed moves remain intact"]
fn t2_placeholder_idles_use_gaisen_sheathed_fallback() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = tenchu_assets::tenchu2::registry::Registry::load(&disc).unwrap();
    let prefix = registry.edition.model_prefix();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let definitions = tenchu_assets::executable::Executable::new(&assets.executable)
        .unwrap()
        .characters()
        .unwrap();
    let ninja_clips =
        tenchu_assets::motion::parse(&assets.asset("HUMAN/MOTION/STAGE2.AMD").unwrap()).unwrap();
    let pack = Pack::default();
    for hero in 0..2 {
        let ninja = definitions
            .iter()
            .find(|a| a.kind == if hero == 0 { 0x21 } else { 0x22 })
            .unwrap();
        let idle = ninja
            .motions
            .iter()
            .find(|m| m.action == MOT_NORMAL)
            .unwrap()
            .clip;
        let fallback = ninja_clips.iter().find(|c| c.id == idle).unwrap();
        for (name, moves) in [
            ("kagami", true),
            ("toda", true),
            ("yorojyu", true),
            ("todanin", true),
            ("kagami", false),
        ] {
            let selected = format!("{prefix}{name}");
            let body = library.get(&selected).unwrap();
            let source = body.imported_moveset.as_ref().unwrap();
            let source_normal = registry
                .clip(body.native.as_ref().unwrap().kind, 0)
                .unwrap();
            let expected = if !moves || source_normal == 1 {
                fallback
            } else {
                let id = source
                    .character
                    .motions
                    .iter()
                    .find(|m| m.action == MOT_NORMAL)
                    .unwrap()
                    .clip;
                source.clips.iter().find(|c| c.id == id).unwrap()
            };
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
                player_model: selected.clone(),
                model_moveset: moves,
                model_moveset_options: tenchu_core::game::moveset::Options {
                    no_idle_taunts: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            settings.graphics.set_preset(0);
            settings.graphics.asset_pack = false;
            settings.graphics.fog = false;
            settings.sheathing.enabled = true;
            crate::sheathing::configure(&mut game, &assets, settings.sheathing).unwrap();
            library.apply_player(&mut game, &settings).unwrap();
            game.roster
                .get_mut(game.player_id)
                .sheathing
                .as_mut()
                .unwrap()
                .input(true);
            for _ in 0..150 {
                game.step(0).unwrap();
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(false);
            }
            if hero == 0 && name == "kagami" && moves {
                let mut models = Models::build(&game.mission, &pack, &settings.graphics)
                    .unwrap()
                    .with_player(
                        &game,
                        &selected,
                        &pack,
                        &pack,
                        &settings.graphics,
                        Some(&library),
                    )
                    .unwrap();
                capture(&game, &mut models, &settings.graphics, "t1-kagami-idle");
            }
            assert_eq!(
                game.player().attribute & ATTR_WEAPON_DRAWN,
                0,
                "hero {hero}, {selected}"
            );
            assert_eq!(
                game.player().motion.action,
                MOT_NORMAL,
                "hero {hero}, {selected}"
            );
            let actual = &game.mission.clips[game.player().motion.clip.unwrap()];
            assert_eq!(
                actual, expected,
                "hero {hero}, {selected}, moves {moves}: sheathed idle"
            );
            let saved = crate::savestate::encode(&game).unwrap();
            let mut restored: tenchu_core::game::Game = crate::savestate::decode(&saved).unwrap();
            for _ in 0..30 {
                game.step(0).unwrap();
                restored.step(0).unwrap();
            }
            assert_eq!(
                postcard::to_allocvec(&game).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
            game.roster
                .get_mut(game.player_id)
                .sheathing
                .as_mut()
                .unwrap()
                .input(true);
            for _ in 0..150 {
                game.step(0).unwrap();
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(false);
            }
            assert_ne!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
            if moves {
                let engage = source
                    .character
                    .motions
                    .iter()
                    .find(|m| m.action == MOT_ENGAGE_STANCE)
                    .unwrap()
                    .clip;
                assert_eq!(
                    game.mission.clips[game.player().motion.clip.unwrap()].id,
                    engage,
                    "hero {hero}, {selected}: armed stance"
                );
                game.step(tenchu_core::input::SQUARE).unwrap();
                assert_eq!(game.player().status, tenchu_core::actor::State::Attack);
                let attack = source
                    .character
                    .motions
                    .iter()
                    .find(|m| m.action == game.player().motion.action)
                    .unwrap()
                    .clip;
                assert_eq!(
                    game.mission.clips[game.player().motion.clip.unwrap()].id,
                    attack
                );
            }
            settings.sheathing.enabled = false;
            crate::sheathing::configure(&mut game, &assets, settings.sheathing).unwrap();
            assert!(game.player().sheathing.is_none());
        }
    }
}
