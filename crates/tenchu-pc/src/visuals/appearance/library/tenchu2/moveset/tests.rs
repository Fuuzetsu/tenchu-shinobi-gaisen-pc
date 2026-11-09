use super::*;
use tenchu_assets::game::GameAssets;
use tenchu_core::input::*;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; imports real hero motions and exercises them in Gaisen"]
fn imported_heroes_move_fight_and_restore_in_gaisen() {
    let disc = std::env::var_os("TENCHU2_DISC").expect("TENCHU2_DISC");
    let disc = crate::source::open_disc(disc.as_ref()).unwrap();
    let movesets = load(&disc).unwrap();
    let gaisen = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(gaisen.as_ref()).unwrap()).unwrap();
    for character in 0..2 {
        for (&kind, source) in movesets.iter().filter(|(kind, _)| **kind < 3) {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let definition = game.player().definition;
            let original_motions = game.mission.characters[definition].motions.clone();
            let random = game.random;
            let options = tenchu_core::game::moveset::Options {
                no_idle_taunts: true,
                ..Default::default()
            };
            game.set_imported_player_moveset(source, options).unwrap();
            assert_eq!(random, game.random);
            assert_eq!(game.player().kind, character);
            let start = game.player().position;
            for _ in 0..12 {
                game.step(UP).unwrap();
            }
            assert_ne!(game.player().position, start);
            for _ in 0..12 {
                game.step(0).unwrap();
            }
            let mut attacks = std::collections::BTreeSet::new();
            let mut contacts = std::collections::BTreeSet::new();
            let yaw = game.player().rotation[1];
            game.step(SQUARE).unwrap();
            for _ in 0..300 {
                let player = game.player();
                let battle = &game.mission.weapons.attacks[player.attack];
                let input = if player.status == tenchu_core::actor::State::Attack
                    && player.motion.count == battle.combo
                {
                    SQUARE
                } else {
                    0
                };
                game.step(input).unwrap();
                if let Some(clip) = game.player().motion.clip {
                    let id = game.mission.clips[clip].id;
                    if let Some(attack) = game.mission.weapons.imported_attacks.get(&id) {
                        attacks.insert(game.player().motion.action);
                        for contact in attack.contacts.iter().flatten() {
                            if let Some(slot) =
                                game.scene.models[game.player().parts[contact.part]].conflict
                            {
                                let slot = &game.collision.slots[slot];
                                assert_eq!(slot.class, tenchu_core::collision::HIT);
                                assert_eq!(slot.offset, contact.hitbox.center);
                                assert_eq!(slot.size, [contact.hitbox.radius; 3]);
                                contacts.insert(game.player().motion.action);
                            }
                        }
                    }
                }
            }
            assert!(attacks.contains(&MOT_ATTACK), "hero {kind}: {attacks:?}");
            assert!(
                attacks.contains(&MOT_ATTACK_SLASH2),
                "hero {kind}: {attacks:?}"
            );
            let expected: std::collections::BTreeSet<_> = [
                MOT_ATTACK,
                MOT_ATTACK_SLASH2,
                MOT_ATTACK_SLASH3,
                MOT_ATTACK_SLASH4,
                MOT_UNARMED_FINISH,
            ]
            .into_iter()
            .filter(|&action| source.character.motions.iter().any(|m| m.action == action))
            .collect();
            assert_eq!(attacks, expected, "hero {kind}");
            assert_eq!(contacts, expected, "contact windows for hero {kind}");
            for _ in 0..100 {
                game.step(0).unwrap();
            }
            assert_eq!(game.player().motion.action, MOT_ENGAGE_STANCE);
            assert_eq!(
                game.player().rotation[1],
                yaw,
                "forward combo must not turn hero {kind}"
            );
            game.step(SQUARE | LEFT).unwrap();
            for _ in 0..8 {
                game.step(0).unwrap();
            }
            let encoded = crate::savestate::encode(&game).unwrap();
            let mut restored: tenchu_core::game::Game = crate::savestate::decode(&encoded).unwrap();
            let mut recorder = crate::rewind::Recorder::default();
            recorder.begin();
            game.capture(&mut recorder).unwrap();
            let mut rewound: tenchu_core::game::Game = recorder.finish().unwrap().decode().unwrap();
            for input in [SQUARE, 0, 0, 0, UP, UP, 0] {
                game.step(input).unwrap();
                restored.step(input).unwrap();
                rewound.step(input).unwrap();
            }
            assert_eq!(
                postcard::to_allocvec(&game).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
            assert_eq!(
                postcard::to_allocvec(&game).unwrap(),
                postcard::to_allocvec(&rewound).unwrap()
            );
            game.set_imported_player_moveset(
                source,
                tenchu_core::game::moveset::Options {
                    player_movement: true,
                    ..options
                },
            )
            .unwrap();
            for action in [MOT_MOVE, MOT_CHASE, MOT_SQUAT, MOT_JUMP_FORWARD] {
                let original = original_motions
                    .iter()
                    .chain(&game.mission.common_motions)
                    .find(|m| m.action == action)
                    .map(|m| m.clip);
                let current = &game.mission.characters[game.player().definition].motions;
                assert_eq!(
                    current
                        .iter()
                        .chain(&game.mission.common_motions)
                        .find(|m| m.action == action)
                        .map(|m| m.clip),
                    original
                );
            }
            game.set_player_moveset(None, &[], Default::default())
                .unwrap();
            assert_eq!(game.player().definition, definition);
            assert_eq!(game.player().kind, character);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; verifies real damage from imported weapons/limbs and cleanup during attacks"]
fn imported_attacks_hit_gaisen_guards() {
    let disc = std::env::var_os("TENCHU2_DISC").expect("TENCHU2_DISC");
    let sources = load(&crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for source in sources.values() {
        for clip in &source.clips {
            assert!(
                ids.insert(clip.id),
                "overlapping actor motion {:#x}",
                clip.id
            );
            assert!(
                !tenchu_assets::tenchu2::CLIPS
                    .iter()
                    .any(|id| clip.id == 0x6000 + id),
                "overlaps standalone Tatsumaru motion {:#x}",
                clip.id
            );
        }
    }
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    for character in 0..2 {
        for (kind, source) in sources.iter().filter(|(kind, source)| {
            **kind < 3
                || matches!(
                    source.character.model.as_str(),
                    "busi" | "roukuno" | "tatu2"
                )
        }) {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            game.set_imported_player_moveset(source, Default::default())
                .unwrap();
            let p = game.player().position;
            let yaw = game.player().rotation[1];
            let forward = game.math.velocity(yaw, 1100, 0);
            let position = tenchu_core::math::Position::new(
                p.x + i32::from(forward[0]),
                p.y,
                p.z + i32::from(forward[2]),
            );
            let definition = game
                .mission
                .characters
                .iter()
                .position(|c| c.kind == 0x10)
                .unwrap();
            let target = game
                .place_actor(
                    tenchu_assets::stage::imports::Actor {
                        definition,
                        model: 0x10,
                    },
                    Default::default(),
                    position,
                    yaw.wrapping_add(2048),
                    true,
                )
                .unwrap();
            let enemy = game.roster.get_mut(target);
            enemy.attribute |= ATTR_WEAPON_DRAWN;
            enemy.health = 30000;
            enemy.max_health = 30000;
            game.roster.get_mut(game.player_id).invulnerable = true;
            game.step(SQUARE).unwrap();
            for _ in 0..150 {
                game.step(0).unwrap();
            }
            assert!(
                game.roster.get(target).health < 30000,
                "hero {kind} on {character} did not land a hit"
            );
            game.step(SQUARE | LEFT).unwrap();
            for _ in 0..12 {
                game.step(0).unwrap();
            }
            game.set_player_moveset(None, &[], Default::default())
                .unwrap();
            for &part in &game.player().parts[1..] {
                assert!(
                    game.scene.models[part].conflict.is_none(),
                    "imported hitbox remains after switching movesets"
                );
            }
            assert!(game.player().weapons[0].is_some());
        }
    }
}

#[test]
#[ignore = "requires both discs; optional TENCHU_MODEL_REVIEW directory renders imported bodies using their own attacks"]
fn imported_models_present_native_attacks_in_gaisen() {
    use crate::{
        settings::Graphics,
        visuals::{appearance, appearance::library::Library, models::Models, pack::Pack},
    };
    use glam::{Mat4, Vec3};
    let path = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let path = std::env::var_os("TENCHU2_DISC").expect("TENCHU2_DISC");
    let disc = crate::source::open_disc(path.as_ref()).unwrap();
    let edition = Edition::detect(&disc).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut game = crate::session::Options {
        character: 1,
        ..Default::default()
    }
    .load(&assets)
    .unwrap();
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    let graphics = Graphics {
        fog: false,
        ..Default::default()
    };
    let output = std::env::var_os("TENCHU_MODEL_REVIEW");
    let mut review = output
        .as_ref()
        .map(|_| crate::capture::review::Review::new(&game.mission).unwrap());
    if let Some(review) = &mut review {
        review.configure(graphics.clone());
    }
    for name in [
        "rikimaru", "ayame", "tatumaru", "rikia", "ayamea", "tatua", "tatua_m",
    ] {
        let id = format!("{}{name}", edition.model_prefix());
        let body = library.get(&id).unwrap();
        assert!(body.has_moveset());
        body.apply_moveset(&mut game, Default::default()).unwrap();
        let mut models = Models::default()
            .with_player(
                &game,
                &id,
                &Pack::default(),
                &Pack::default(),
                &graphics,
                Some(&library),
            )
            .unwrap();
        if let Some(review) = &mut review {
            review.renderer.bind_models(&mut models);
        }
        game.step(SQUARE).unwrap();
        for _ in 0..14 {
            game.step(0).unwrap();
        }
        let mut vertices = Vec::new();
        appearance::visit_parts(&game, true, Some(&models), None, |key, geometry, matrix| {
            if let crate::visuals::interpolation::Part::Actor { root, .. } = key
                && root == game.player().root_model
                && !models.visit_part(key, geometry, matrix, false, 0, |tri| {
                    vertices.extend(tri.into_iter().copied())
                })
            {
                crate::render_mesh::append_geometry(&mut vertices, geometry, matrix, None);
            }
        });
        assert!(!vertices.is_empty());
        let position = Vec3::from(game.player().position.to_array().map(|v| v as f32));
        assert!(
            vertices.iter().all(|v| Vec3::from(v.position).is_finite()
                && Vec3::from(v.position).distance(position) < 6000.0),
            "{id}"
        );
        if let (Some(output), Some(review)) = (&output, &mut review) {
            let output = std::path::Path::new(output);
            std::fs::create_dir_all(output).unwrap();
            let center = position + Vec3::new(0.0, -900.0, 0.0);
            let camera = Mat4::perspective_rh(42f32.to_radians(), 16.0 / 9.0, 10.0, 20000.0)
                * Mat4::look_at_rh(
                    center + Vec3::new(3300.0, -250.0, 1800.0),
                    center,
                    Vec3::NEG_Y,
                );
            let mesh = review.renderer.upload(&vertices);
            review
                .capture(
                    camera,
                    &[&mesh],
                    None,
                    false,
                    &output.join(format!("{name}-own-attack.png")),
                )
                .unwrap();
        }
        for _ in 0..80 {
            game.step(0).unwrap();
        }
    }
}
