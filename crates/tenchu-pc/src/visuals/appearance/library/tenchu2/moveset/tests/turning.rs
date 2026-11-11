use super::*;

#[test]
#[ignore = "requires both discs; imported T2 standing turns match native updates, with the host rate retained only by Keep player movement"]
fn imported_t2_turning_matches_native_rate() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sources = load(&disc).unwrap();
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    for kind in 0..3 {
        let source = &sources[&kind];
        let mut native =
            crate::tenchu2::native::Session::load(&disc, [9, 119, 191][kind as usize], 17).unwrap();
        for _ in 0..30 {
            native.step(0).unwrap();
        }
        let player = native.world.player;
        for _ in 0..4 {
            native.step(tenchu_core::tenchu2::input::RIGHT).unwrap();
        }
        let mut expected = Vec::new();
        for _ in 0..8 {
            let before = native.world.actors[player].actor.rotation.y;
            native.step(tenchu_core::tenchu2::input::RIGHT).unwrap();
            expected.push(
                native.world.actors[player]
                    .actor
                    .rotation
                    .y
                    .wrapping_sub(before),
            );
        }
        assert!(expected.iter().all(|&turn| turn == source.character.turn));
        for character in 0..2 {
            for keep in [false, true] {
                let mut game = crate::session::Options {
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                for _ in 0..30 {
                    game.step(0).unwrap();
                }
                let original = game.mission.characters[game.player().definition].turn;
                game.set_imported_player_moveset(
                    source,
                    tenchu_core::game::moveset::Options {
                        player_movement: keep,
                        no_idle_taunts: true,
                    },
                )
                .unwrap();
                for _ in 0..4 {
                    game.step(RIGHT).unwrap();
                }
                let mut actual = Vec::new();
                for _ in 0..8 {
                    let before = game.player().rotation[1];
                    game.step(RIGHT).unwrap();
                    actual.push(game.player().rotation[1].wrapping_sub(before));
                }
                eprintln!(
                    "T1 hero {character}, T2 hero {kind}, keep {keep}: {actual:?}, native {expected:?}"
                );
                assert_eq!(
                    actual,
                    if keep {
                        vec![original; 8]
                    } else {
                        expected.clone()
                    }
                );
                game.set_player_moveset(None, &[], Default::default())
                    .unwrap();
                assert_eq!(
                    game.mission.characters[game.player().definition].turn,
                    original
                );
            }
        }
    }
}

#[test]
#[ignore = "requires both discs; sheathed T2 forward and backward turns use native rates and a straight backward walk stays straight"]
fn imported_t2_sheathed_turning_matches_native() {
    use tenchu_core::tenchu2::{actor::Velocity, rig::WeaponMode};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sources = load(&disc).unwrap();
    let source = &sources[&1];
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    for character in 0..2 {
        let mut game = crate::session::Options {
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        for id in game.roster.order.clone() {
            if id != game.player_id {
                game.roster.get_mut(id).attribute |= ATTR_SUSPEND;
            }
        }
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        game.set_imported_player_moveset(source, Default::default())
            .unwrap();
        crate::sheathing::configure(
            &mut game,
            &assets,
            tenchu_core::game::sheathing::Settings {
                enabled: true,
                ..Default::default()
            },
        )
        .unwrap();
        game.roster
            .get_mut(game.player_id)
            .sheathing
            .as_mut()
            .unwrap()
            .input(true);
        for _ in 0..100 {
            game.step(0).unwrap();
            game.roster
                .get_mut(game.player_id)
                .sheathing
                .as_mut()
                .unwrap()
                .input(false);
        }
        assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        let saved = postcard::to_allocvec(&game).unwrap();
        let mut native = crate::tenchu2::native::Session::load(&disc, 119, 17).unwrap();
        for _ in 0..30 {
            native.step(0).unwrap();
        }
        let player = native.world.player;
        native.world.actors[player]
            .actor
            .set_weapon(
                &mut native.world.instances[player].rig,
                player,
                &mut native.world.collision,
                &native.assets.resources,
                WeaponMode::Sheathe,
            )
            .unwrap();
        native.world.actors[player]
            .actor
            .change_motion(0, Velocity::Stop, &native.assets.resources)
            .unwrap();
        let saved_native = native.capture_state().unwrap();
        for (buttons, native_buttons) in [
            (RIGHT, tenchu_core::tenchu2::input::RIGHT),
            (
                UP | RIGHT,
                tenchu_core::tenchu2::input::UP | tenchu_core::tenchu2::input::RIGHT,
            ),
            (
                UP | LEFT,
                tenchu_core::tenchu2::input::UP | tenchu_core::tenchu2::input::LEFT,
            ),
            (
                DOWN | RIGHT,
                tenchu_core::tenchu2::input::DOWN | tenchu_core::tenchu2::input::RIGHT,
            ),
            (
                DOWN | LEFT,
                tenchu_core::tenchu2::input::DOWN | tenchu_core::tenchu2::input::LEFT,
            ),
            (DOWN, tenchu_core::tenchu2::input::DOWN),
        ] {
            game = postcard::from_bytes(&saved).unwrap();
            native.restore_state(&saved_native).unwrap();
            for _ in 0..4 {
                game.step(buttons).unwrap();
                native.step(native_buttons).unwrap();
            }
            for tick in 0..8 {
                let a = game.player().rotation[1];
                let b = native.world.actors[player].actor.rotation.y;
                game.step(buttons).unwrap();
                native.step(native_buttons).unwrap();
                assert_eq!(
                    game.player().rotation[1].wrapping_sub(a),
                    native.world.actors[player].actor.rotation.y.wrapping_sub(b),
                    "hero {character}, buttons {buttons:x}, tick {tick}"
                );
            }
        }
    }
}
