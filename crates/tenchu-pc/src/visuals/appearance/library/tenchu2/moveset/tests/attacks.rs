use super::*;

#[test]
#[ignore = "requires both discs; imported basic melee swings retain native active-contact duration on original T1 bodies"]
fn imported_t2_attack_windows_match_native() {
    use tenchu_core::tenchu2::{collision, input};
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
        if !native.world.actors[player].actor.weapon_drawn() {
            native.step(input::ATTACK).unwrap();
            for _ in 0..120 {
                native.step(0).unwrap();
            }
        }
        let mut expected = Vec::new();
        for tick in 0..90 {
            native
                .step(if tick == 0 { input::ATTACK } else { 0 })
                .unwrap();
            let actor = &native.world.actors[player].actor;
            let contacts: Vec<_> = native
                .world
                .collision
                .slots()
                .iter()
                .filter(|slot| slot.owner == Some(player) && slot.flags == collision::WEAPON)
                .map(|slot| (slot.offset.to_array(), slot.size.to_array()))
                .collect();
            if !contacts.is_empty() {
                expected.push((tick, actor.motion.frame, contacts));
            }
        }
        eprintln!("T2 hero {kind}: {expected:?}");
        assert!(!expected.is_empty());
        for character in 0..2 {
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
            game.set_imported_player_moveset(source, Default::default())
                .unwrap();
            if game.player().attribute & ATTR_WEAPON_DRAWN == 0 {
                game.step(SQUARE).unwrap();
                for _ in 0..120 {
                    game.step(0).unwrap();
                }
            }
            let mut actual = Vec::new();
            for tick in 0..90 {
                game.step(if tick == 0 { SQUARE } else { 0 }).unwrap();
                let contacts: Vec<_> = game.collision.slots[..game.collision.len]
                    .iter()
                    .filter(|slot| {
                        slot.actor() == Some(game.player_id)
                            && slot.class & tenchu_core::collision::HIT != 0
                            && slot.model.is_some_and(|model| {
                                game.scene.models[model].attribute & tenchu_core::scene::COLLIDE
                                    != 0
                            })
                    })
                    .map(|slot| (slot.offset, slot.size))
                    .collect();
                if !contacts.is_empty() {
                    actual.push((tick, game.player().motion.count, contacts));
                }
            }
            eprintln!("T1 hero {character}, T2 moves {kind}: {actual:?}");
            assert_eq!(actual, expected, "T1 hero {character}, T2 moves {kind}");
        }
    }
}
