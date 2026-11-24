use super::*;

#[test]
#[ignore = "requires both discs; Gaisen's hold-back shortcut preserves supported imported specials and their authored contacts, with unsupported back attacks retained"]
fn imported_back_special_matches_source_command_and_camera_direction() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sources = load(&disc).unwrap();
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let mut supported = 0;
    for character in 0..2 {
        for kind in 0..3 {
            let source = &sources[&kind];
            let special = source
                .character
                .motions
                .iter()
                .find(|m| m.action == MOT_ATTACK_LUNGE_BACK);
            let action = if special.is_some() {
                MOT_ATTACK_LUNGE_BACK
            } else {
                MOT_ATTACK_BACK
            };
            let expected_clip = source
                .character
                .motions
                .iter()
                .find(|m| m.action == action)
                .unwrap()
                .clip;
            let mut original = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            original.events = Default::default();
            for _ in 0..30 {
                original.step(0).unwrap();
            }
            original
                .set_imported_player_moveset(source, Default::default())
                .unwrap();
            if original.player().attribute & ATTR_WEAPON_DRAWN == 0 {
                original.step(SQUARE).unwrap();
                for _ in 0..120 {
                    original.step(0).unwrap();
                }
            }
            if special.is_some() {
                original.step(UP).unwrap();
                original.step(DOWN).unwrap();
                assert_eq!(original.player().motion.action, MOT_ENGAGE_STANCE);
                supported += 1;
            }
            original.roster.get_mut(original.player_id).motion.count = 5;
            let mut shortcut: tenchu_core::game::Game =
                postcard::from_bytes(&postcard::to_allocvec(&original).unwrap()).unwrap();
            shortcut.roster.get_mut(shortcut.player_id).pad = Default::default();
            shortcut.roster.get_mut(shortcut.player_id).pad.update(DOWN);
            shortcut.combat.back_special = true;
            let facing = f32::from(shortcut.player().rotation[1]) * std::f32::consts::TAU / 4096.0;
            let desired = glam::Vec2::new(facing.sin(), facing.cos());
            for quarter in 0..4 {
                let yaw = facing + quarter as f32 * std::f32::consts::FRAC_PI_2;
                let mut orbit = crate::navigation::Orbit::default();
                orbit.yaw = yaw;
                let analog = glam::Vec2::new(
                    desired.dot(glam::Vec2::new(yaw.cos(), -yaw.sin())),
                    desired.dot(glam::Vec2::new(yaw.sin(), yaw.cos())),
                );
                let (held, turn) = orbit.movement(
                    &shortcut,
                    SQUARE,
                    analog,
                    &crate::settings::Navigation {
                        camera_relative: true,
                        ..Default::default()
                    },
                );
                assert_eq!(held, DOWN | SQUARE, "quarter {quarter}");
                assert_eq!(turn, None);
            }
            original.step(DOWN | SQUARE).unwrap();
            shortcut.step(DOWN | SQUARE).unwrap();
            assert_eq!(
                original.player().motion.action,
                action,
                "source hero {kind}"
            );
            assert_eq!(
                shortcut.player().motion.action,
                action,
                "host {character}, source {kind}"
            );
            let clip = shortcut.player().motion.clip.unwrap();
            assert_eq!(shortcut.mission.clips[clip].id, expected_clip);
            let mut contacts = 0;
            for tick in 0..90 {
                assert_eq!(
                    trace(&original),
                    trace(&shortcut),
                    "host {character}, source {kind}, tick {tick}"
                );
                contacts += active(&shortcut).len();
                original.step(0).unwrap();
                shortcut.step(0).unwrap();
            }
            assert!(contacts > 0);
        }
    }
    assert!(supported > 0);
    eprintln!("{supported} imported special registrations verified on original T1 bodies");
}

fn active(game: &tenchu_core::game::Game) -> Vec<(usize, [i16; 3], [i16; 3])> {
    game.collision.slots[..game.collision.len]
        .iter()
        .filter(|slot| {
            slot.actor() == Some(game.player_id)
                && slot.class & tenchu_core::collision::HIT != 0
                && slot.model.is_some_and(|model| {
                    game.scene.models[model].attribute & tenchu_core::scene::COLLIDE != 0
                })
        })
        .map(|slot| (slot.model.unwrap(), slot.offset, slot.size))
        .collect()
}

fn trace(game: &tenchu_core::game::Game) -> Vec<u8> {
    let actor = game.player();
    postcard::to_allocvec(&(
        actor.motion.clone(),
        actor.position,
        actor.velocity,
        actor.rotation,
        active(game),
    ))
    .unwrap()
}
