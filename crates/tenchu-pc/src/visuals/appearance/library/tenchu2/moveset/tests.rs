use super::*;
use tenchu_assets::game::GameAssets;
use tenchu_core::input::*;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; all imported hero stances and steps use their source clips, with crouch override and save continuation"]
fn imported_circle_stance_follows_moveset_and_source_steps() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sources = load(&disc).unwrap();
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    for character in 0..2 {
        for kind in 0..3 {
            let source = &sources[&kind];
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            game.intelligence.ignore_player = true;
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            game.set_imported_player_moveset(source, Default::default())
                .unwrap();
            for _ in 0..5 {
                game.step(CIRCLE).unwrap();
            }
            assert_eq!(game.player().motion.action, MOT_ENGAGE_GUARD);
            let stance = source
                .character
                .motions
                .iter()
                .find(|m| m.action == MOT_ENGAGE_GUARD)
                .unwrap()
                .clip;
            assert_eq!(
                game.mission.clips[game.player().motion.clip.unwrap()].id,
                stance
            );
            let saved = crate::savestate::encode(&game).unwrap();
            for (direction, action) in [
                (UP, MOT_CHASE_DASH_FWD),
                (DOWN, MOT_CHASE_DASH_BACK),
                (RIGHT, MOT_CHASE_DASH_RIGHT),
                (LEFT, MOT_CHASE_DASH_LEFT),
            ] {
                let mut game: tenchu_core::game::Game = crate::savestate::decode(&saved).unwrap();
                let clip = source
                    .character
                    .motions
                    .iter()
                    .find(|m| m.action == action)
                    .unwrap()
                    .clip;
                let mut starts = 0;
                for _ in 0..130 {
                    let previous = game.player().motion.action;
                    game.step(CIRCLE | direction).unwrap();
                    if previous != action && game.player().motion.action == action {
                        starts += 1;
                        assert_eq!(
                            game.mission.clips[game.player().motion.clip.unwrap()].id,
                            clip
                        );
                    }
                }
                assert!(
                    starts >= 2,
                    "host {character}, source {kind}, action {action:x}: {starts}"
                );
            }
            game.step(CIRCLE | UP).unwrap();
            let save = crate::savestate::encode(&game).unwrap();
            let mut restored: tenchu_core::game::Game = crate::savestate::decode(&save).unwrap();
            for _ in 0..100 {
                game.step(CIRCLE | UP).unwrap();
                restored.step(CIRCLE | UP).unwrap();
            }
            assert_eq!(
                postcard::to_allocvec(&game).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
            for preference in [CircleAction::Crouch, CircleAction::Moveset] {
                let mut game: tenchu_core::game::Game = crate::savestate::decode(&saved).unwrap();
                game.intelligence.circle_action = preference;
                let held = if preference == CircleAction::Crouch {
                    CIRCLE
                } else {
                    R1
                };
                for _ in 0..20 {
                    game.step(held).unwrap();
                }
                assert_eq!(game.player().motion.action, MOT_SQUAT);
            }
            game.set_player_moveset(None, &[], Default::default())
                .unwrap();
            for _ in 0..20 {
                game.step(CIRCLE).unwrap();
            }
            assert_eq!(game.player().motion.action, MOT_SQUAT);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; source draw/stow clips, weapon event frames, setting order and save continuation"]
fn imported_sheathing_uses_source_motions_and_weapon_frames() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let weapons = Weapons::load(&disc).unwrap();
    let sources = load(&disc).unwrap();
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    for character in 0..2 {
        for kind in 0..3 {
            for configure_first in [false, true] {
                let source = &sources[&kind];
                let mut game = crate::session::Options {
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                game.intelligence.ignore_player = true;
                for _ in 0..30 {
                    game.step(0).unwrap();
                }
                let settings = tenchu_core::game::sheathing::Settings {
                    enabled: true,
                    ..Default::default()
                };
                if configure_first {
                    crate::sheathing::configure(&mut game, &assets, settings).unwrap();
                }
                game.set_imported_player_moveset(source, Default::default())
                    .unwrap();
                crate::sheathing::configure(&mut game, &assets, settings).unwrap();
                for (drawing, action, native) in [
                    (false, MOT_STATE_SHEATHE, 0x701),
                    (true, MOT_STATE_DRAW, 0x700),
                ] {
                    let id = source
                        .character
                        .motions
                        .iter()
                        .find(|m| m.action == action)
                        .unwrap()
                        .clip;
                    let expected = source.clips.iter().find(|c| c.id == id).unwrap();
                    let native_id = registry.clip(kind, native).unwrap();
                    let frame = registry.weapon_frame_30hz(native_id, expected.duration);
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
                    assert_eq!(game.player().motion.action, action);
                    let actual = &game.mission.clips[game.player().motion.clip.unwrap()];
                    assert_eq!(
                        actual, expected,
                        "hero {character}, source {kind}, action {action:x}"
                    );
                    let saved = crate::savestate::encode(&game).unwrap();
                    let mut restored: tenchu_core::game::Game =
                        crate::savestate::decode(&saved).unwrap();
                    let mut swaps = Vec::new();
                    let mut voices = 0;
                    for _ in 0..150 {
                        let before = game.player().attribute & ATTR_WEAPON_DRAWN;
                        let count = game.player().motion.count;
                        game.step(0).unwrap();
                        restored.step(0).unwrap();
                        if before != game.player().attribute & ATTR_WEAPON_DRAWN {
                            swaps.push(count);
                        }
                        voices += game
                            .intelligence
                            .voices
                            .iter()
                            .filter(|&&(actor, cue)| {
                                actor == game.player_id && cue == i16::from(!drawing)
                            })
                            .count();
                    }
                    assert_eq!(swaps, [frame], "source {kind}, action {action:x}");
                    assert_eq!(
                        voices,
                        usize::from(
                            weapons
                                .parts(registry.actor(kind).unwrap().weapon)
                                .next()
                                .is_some_and(|part| part.model.is_some())
                        ),
                        "source {kind}, action {action:x}"
                    );
                    assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN != 0, drawing);
                    assert_eq!(
                        postcard::to_allocvec(&game).unwrap(),
                        postcard::to_allocvec(&restored).unwrap()
                    );
                }
                game.set_player_moveset(None, &[], Default::default())
                    .unwrap();
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(true);
                game.step(0).unwrap();
                let fallback = game.mission.clips[game.player().motion.clip.unwrap()].id;
                assert!(!source.clips.iter().any(|clip| clip.id == fallback));
                assert_eq!(game.player().motion.action, MOT_STATE_SHEATHE);
            }
        }
    }
}

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
            let original_weapons = game
                .player()
                .weapons
                .each_ref()
                .map(|w| w.as_ref().map(|w| w.kind));
            let random = game.random;
            let options = tenchu_core::game::moveset::Options {
                no_idle_taunts: true,
                ..Default::default()
            };
            game.set_imported_player_moveset(source, options).unwrap();
            assert_eq!(random, game.random);
            assert_eq!(game.player().kind, character);
            let mut slots = game.player().weapons.clone();
            if game.player().attribute & ATTR_WEAPON_DRAWN != 0 {
                source.equipment.draw.swap(&mut slots);
            }
            for attachment in &source.equipment.attachments {
                let weapon = slots[attachment.slot].as_ref().unwrap();
                let imported = &game.mission.weapons.imported_models[&weapon.kind];
                assert_eq!(imported.id, attachment.model.id);
                assert_eq!(weapon.imported_contact, Some(attachment.melee));
                assert_eq!(
                    game.scene.models[weapon.model].parent,
                    Some(game.player().parts[attachment.part])
                );
                assert_eq!(
                    postcard::to_allocvec(&game.mission.weapons.models[&weapon.kind]).unwrap(),
                    postcard::to_allocvec(&imported.geometry).unwrap()
                );
            }
            assert!(slots[4].is_some(), "hero {kind}: permanent sheath");
            let scene_size = game.scene.models.len();
            let resource_size = game.mission.weapons.models.len();
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
            for _ in 0..4 {
                game.set_imported_player_moveset(source, options).unwrap();
                game.set_player_moveset(None, &[], Default::default())
                    .unwrap();
            }
            assert_eq!(
                game.scene.models.len(),
                scene_size,
                "repeated swaps leak scene slots"
            );
            assert_eq!(
                game.mission.weapons.models.len(),
                resource_size,
                "repeated swaps duplicate weapon assets"
            );
            assert_eq!(
                game.player()
                    .weapons
                    .each_ref()
                    .map(|w| w.as_ref().map(|w| w.kind)),
                original_weapons
            );
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
        let mut models = Models::build(&game.mission, &Pack::default(), &graphics)
            .unwrap()
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

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; mixes imported hero cues with host sounds and restores save/rewind audio"]
fn imported_voices_follow_the_moveset_and_survive_restore() {
    use tenchu_assets::audio::{bank::Bank, voice::Dialect};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let movesets = load(&disc).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    for kind in 0..3 {
        let source = &movesets[&kind];
        let voice = source.voice.as_ref().expect("hero source bank");
        assert_eq!(voice.dialect, Dialect::Tenchu2);
        assert!(
            voice
                .bank
                .samples
                .iter()
                .any(|s| s.frames.iter().any(|&f| f != 0))
        );
        let mut game = crate::session::Options {
            character: 1,
            ..Default::default()
        }
        .load(&gaisen)
        .unwrap();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        game.set_imported_player_moveset(source, Default::default())
            .unwrap();
        game.step(SQUARE).unwrap();
        let mut personal = std::collections::BTreeSet::new();
        let mut pcm = tenchu_audio::Mixer::new(48000);
        pcm.set_bank(Arc::new(
            Bank::load(&gaisen, 0, 1, tenchu_assets::script::Language::Japanese).unwrap(),
        ));
        let mut audible = false;
        for _ in 0..90 {
            game.step(0).unwrap();
            let rng = game.random;
            let sounds = game
                .audio_with_burning([(game.player().position, 0x39)], None)
                .unwrap();
            assert!(
                sounds
                    .iter()
                    .any(|s| s.bank.is_none() && s.sound.id == 0x39)
            );
            for event in sounds {
                let s = event.sound;
                if let Some(bank) = event.bank {
                    assert!(Arc::ptr_eq(&voice.bank, &bank));
                    personal.insert(s.id);
                    pcm.play_from(&bank, s.id, s.volume, s.pan);
                }
            }
            for _ in 0..1600 {
                audible |= pcm.frame().iter().any(|v| v.abs() > 0.001);
            }
            assert_eq!(game.random, rng);
        }
        assert!(
            personal.contains(&2),
            "hero {kind}: attack effect {personal:?}"
        );
        assert!(
            personal.contains(&10) || personal.contains(&11),
            "hero {kind}: attack voice {personal:?}"
        );
        assert!(audible, "hero {kind}: silent source");
        let bytes = postcard::to_allocvec(&game).unwrap();
        let mut restored: tenchu_core::game::Game = postcard::from_bytes(&bytes).unwrap();
        restored.intelligence.voices = vec![(restored.player_id, 9)];
        let events = restored.audio_with_burning([], None).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sound.id, 10);
        let restored_bank = events[0].bank.as_ref().unwrap();
        assert_eq!(
            postcard::to_allocvec(restored_bank).unwrap(),
            postcard::to_allocvec(&voice.bank).unwrap()
        );
        restored
            .set_player_moveset(None, &[], Default::default())
            .unwrap();
        let events = restored.audio_with_burning([], None).unwrap();
        assert!(events[0].bank.is_none());
        assert_eq!(events[0].sound.id, 0x59);
    }
}
