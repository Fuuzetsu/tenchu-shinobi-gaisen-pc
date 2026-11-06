use super::*;
use tenchu_core::{game::sheathing::Settings as Sheathing, ids::*, input};

fn ready(character: i16) -> Application {
    let mut app = application(Options {
        character,
        ..Default::default()
    });
    app.campaign.game.events = Default::default();
    app.set_sheathing(Sheathing {
        enabled: true,
        ..Default::default()
    })
    .unwrap();
    for _ in 0..20 {
        app.campaign.game.step(0).unwrap();
    }
    app
}

fn press_sheathe(app: &mut Application) {
    app.controls.key(KeyCode::KeyH, true);
    for _ in 0..80 {
        let buttons = app.player_input();
        assert_eq!(buttons, 0);
        assert!(app.game_tick(buttons));
    }
    app.controls.key(KeyCode::KeyH, false);
    app.player_input();
}

#[test]
#[ignore = "requires TENCHU_DISC; real draw/sheath cycles, original locomotion and single-toggle input for both heroes"]
fn sheathing_controls_and_ground_attacks() {
    for character in 0..2 {
        let mut app = ready(character);
        let armed = app
            .campaign
            .game
            .player()
            .weapons
            .each_ref()
            .map(|w| w.as_ref().map(|w| w.model));
        press_sheathe(&mut app);
        assert_eq!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        assert_eq!(app.campaign.game.player().motion.action, MOT_NORMAL);
        assert_ne!(
            app.campaign
                .game
                .player()
                .weapons
                .each_ref()
                .map(|w| w.as_ref().map(|w| w.model)),
            armed
        );
        for _ in 0..20 {
            app.campaign.game.step(input::UP).unwrap();
        }
        assert_eq!(app.campaign.game.player().motion.action, MOT_CHASE);
        let base = app.campaign.game.player().velocity;
        app.set_sheathing(Sheathing {
            speed: 150,
            ..app.settings.sheathing
        })
        .unwrap();
        app.campaign.game.step(input::UP).unwrap();
        assert!(i32::from(app.campaign.game.player().velocity[0]).abs() > i32::from(base[0]).abs());
        app.set_walking(true).unwrap();
        app.campaign.game.walking.as_mut().unwrap().held = true;
        app.campaign.game.step(input::UP).unwrap();
        assert_eq!(
            app.campaign.game.player().motion.clip,
            Some(app.campaign.game.walking.unwrap().clip)
        );
        let slow = app.campaign.game.player().velocity;
        app.set_walking(false).unwrap();
        assert_ne!(app.campaign.game.player().velocity, slow);
        app.campaign.game.step(input::SQUARE).unwrap();
        assert_eq!(app.campaign.game.player().motion.action, MOT_STATE_DRAW);
        for _ in 0..30 {
            app.campaign.game.step(0).unwrap();
        }
        assert_ne!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        assert_eq!(
            app.campaign
                .game
                .player()
                .weapons
                .each_ref()
                .map(|w| w.as_ref().map(|w| w.model)),
            armed
        );
        app.campaign.game.step(input::SQUARE).unwrap();
        assert_eq!(app.campaign.game.player().motion.action, MOT_ATTACK);
        // A tap during a committed attack is discarded, not queued for recovery.
        app.controls.key(KeyCode::KeyH, true);
        let buttons = app.player_input();
        app.game_tick(buttons);
        assert_eq!(app.campaign.game.player().motion.action, MOT_ATTACK);
        for _ in 0..90 {
            let buttons = app.player_input();
            app.game_tick(buttons);
        }
        assert_ne!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        app.controls.clear();
        app.player_input();
        press_sheathe(&mut app);
        assert_eq!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        app.set_sheathing(Default::default()).unwrap();
        assert_ne!(app.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        assert!(app.campaign.game.player().sheathing.is_none());
        for _ in 0..30 {
            app.campaign.game.step(input::UP).unwrap();
        }
        assert_eq!(app.campaign.game.player().motion.action, MOT_CHASE);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; optional sheathing leaves armed gameplay unchanged and savestates retain a live sheathed stance"]
fn sheathing_armed_parity_and_saved_state() {
    for character in 0..2 {
        let mut on = ready(character);
        let mut off = ready(character);
        off.set_sheathing(Default::default()).unwrap();
        for tick in 0..180 {
            let buttons = match tick % 90 {
                0..20 => input::UP,
                20..30 => input::DOWN,
                30..45 => input::CIRCLE,
                45..60 => input::SQUARE,
                _ => 0,
            };
            on.campaign.game.step(buttons).unwrap();
            off.campaign.game.step(buttons).unwrap();
            if tick % 30 == 0 {
                let state = on
                    .campaign
                    .game
                    .roster
                    .get_mut(on.campaign.game.player_id)
                    .sheathing
                    .take();
                assert!(
                    postcard::to_allocvec(&on.campaign.game).unwrap()
                        == postcard::to_allocvec(&off.campaign.game).unwrap(),
                    "hero {character}, tick {tick}"
                );
                on.campaign
                    .game
                    .roster
                    .get_mut(on.campaign.game.player_id)
                    .sheathing = state;
            }
        }
        for _ in 0..90 {
            on.campaign.game.step(0).unwrap();
        }
        press_sheathe(&mut on);
        assert_eq!(on.campaign.game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        let bytes = on.capture_state().unwrap();
        let mut restored = application(Options {
            character,
            ..Default::default()
        });
        restored.restore_state(&bytes).unwrap();
        assert!(restored.settings.sheathing.enabled);
        for tick in 0..150 {
            let buttons = if tick < 60 {
                input::UP
            } else if tick == 80 {
                input::SQUARE
            } else {
                0
            };
            on.campaign.game.step(buttons).unwrap();
            restored.campaign.game.step(buttons).unwrap();
        }
        assert!(
            postcard::to_allocvec(&on.campaign.game).unwrap()
                == postcard::to_allocvec(&restored.campaign.game).unwrap()
        );
        on.settings.rewind_seconds = 5;
        for _ in 0..10 {
            on.game_tick(0);
        }
        let expected = postcard::to_allocvec(&on.campaign.game).unwrap();
        let index = on.rewind.entries.len() - 1;
        press_sheathe(&mut on);
        on.rewind_to(index).unwrap();
        assert!(postcard::to_allocvec(&on.campaign.game).unwrap() == expected);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; unarmed imports, menu resets, mid-kick saves and rewind retain the source attack"]
fn unarmed_controls_save_and_rewind() {
    for character in 0..2 {
        let mut app = ready(character);
        app.apply_item(Item::UnarmedAttacks, 1).unwrap();
        assert!(app.settings.sheathing.unarmed);
        assert!(
            !app.campaign
                .game
                .player()
                .sheathing
                .as_ref()
                .unwrap()
                .needs_unarmed_motions(Default::default())
        );
        press_sheathe(&mut app);
        app.controls.key(KeyCode::KeyJ, true);
        let input = app.player_input();
        app.game_tick(input);
        app.controls.key(KeyCode::KeyJ, false);
        for _ in 0..10 {
            app.game_tick(0);
        }
        let game = &app.campaign.game;
        assert_eq!(
            game.mission.clips[game.player().motion.clip.unwrap()].id,
            454
        );
        assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
        let bytes = app.capture_state().unwrap();
        let mut restored = application(Options {
            character,
            ..Default::default()
        });
        restored.restore_state(&bytes).unwrap();
        assert!(restored.settings.sheathing.unarmed);
        for tick in 0..90 {
            let buttons = if tick == 10 || tick == 22 {
                input::SQUARE
            } else {
                0
            };
            app.game_tick(buttons);
            restored.game_tick(buttons);
        }
        assert!(
            postcard::to_allocvec(&app.campaign.game).unwrap()
                == postcard::to_allocvec(&restored.campaign.game).unwrap()
        );
        app.settings.rewind_seconds = 5;
        app.game_tick(input::SQUARE);
        for _ in 0..10 {
            app.game_tick(0);
        }
        let expected = postcard::to_allocvec(&app.campaign.game).unwrap();
        let index = app.rewind.entries.len() - 1;
        for _ in 0..40 {
            app.game_tick(0);
        }
        app.rewind_to(index).unwrap();
        assert!(postcard::to_allocvec(&app.campaign.game).unwrap() == expected);
        app.reset_item(Item::UnarmedAttacks).unwrap();
        assert!(!app.settings.sheathing.unarmed);
        assert!(app.settings.sheathing.enabled);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_T2_MOVES; real four-part imported combo, collisions, saves and style changes on both heroes"]
fn tatsumaru_combo_contacts_and_saved_state() {
    use tenchu_core::game::sheathing::UnarmedStyle;
    use tenchu_core::{ai::programs::Programs, collision};
    for character in 0..2 {
        let mut app = ready(character);
        app.set_sheathing(Sheathing {
            unarmed: true,
            unarmed_style: UnarmedStyle::Tatsumaru,
            ..app.settings.sheathing
        })
        .unwrap();
        press_sheathe(&mut app);
        let game = &mut app.campaign.game;
        game.intelligence.ignore_player = true;
        let player = game.player();
        let offset = game.math.velocity(player.rotation[1], 900, 0);
        let position = player.position.wrapping_add(offset.map(i32::from).into());
        let yaw = player.rotation[1];
        let definition = game
            .mission
            .characters
            .iter()
            .position(|c| c.kind == 0x10)
            .unwrap();
        let enemy = game
            .place_actor(
                tenchu_assets::stage::imports::Actor {
                    definition,
                    model: 0x10,
                },
                Programs::default(),
                position,
                yaw,
                true,
            )
            .unwrap();
        let guard = game.roster.get_mut(enemy);
        guard.health = 9999;
        guard.max_health = 9999;
        guard.attribute |= ATTR_WEAPON_DRAWN;
        let mut actions = std::collections::BTreeSet::new();
        let mut contacts = std::collections::BTreeSet::new();
        let mut restored = None;
        app.game_tick(input::SQUARE);
        for tick in 0..180 {
            let player = app.campaign.game.player();
            let timing = &app.campaign.game.mission.weapons.attacks[player.attack];
            let buttons = if matches!(
                player.motion.action,
                MOT_ATTACK | MOT_ATTACK_SLASH2 | MOT_ATTACK_SLASH3
            ) && player.motion.count == timing.combo
            {
                input::SQUARE
            } else {
                0
            };
            assert!(app.game_tick(buttons));
            if let Some(copy) = &mut restored {
                let copy: &mut Application = copy;
                assert!(copy.game_tick(buttons));
                assert!(
                    postcard::to_allocvec(&app.campaign.game).unwrap()
                        == postcard::to_allocvec(&copy.campaign.game).unwrap()
                );
            }
            if tick == 7 {
                let mut copy = application(Options {
                    character,
                    ..Default::default()
                });
                copy.restore_state(&app.capture_state().unwrap()).unwrap();
                restored = Some(copy);
                assert!(
                    app.set_sheathing(Sheathing {
                        unarmed_style: UnarmedStyle::Onikage,
                        ..app.settings.sheathing
                    })
                    .is_err()
                );
            }
            let game = &app.campaign.game;
            let player = game.player();
            if player.status == tenchu_core::actor::State::Attack {
                actions.insert(player.motion.action);
                assert_eq!(player.attribute & ATTR_WEAPON_DRAWN, 0);
                let id = game.mission.clips[player.motion.clip.unwrap()].id - 0x6000;
                let expected = match id {
                    0xb8 => 14,
                    0xb9 => 13,
                    0x4bd | 0x4be => 12,
                    _ => panic!("Unexpected combo clip {id:x}"),
                };
                for part in [8, 9, 11, 12, 13, 14] {
                    if let Some(slot) = game.scene.models[player.parts[part]].conflict
                        && game.collision.slots[slot].class == collision::HIT
                    {
                        assert_eq!(part, expected);
                        contacts.insert(id);
                    }
                }
                if player.motion.count == game.mission.weapons.attacks[player.attack].start + 1
                    && let Some(path) = std::env::var_os("TENCHU_T2_REVIEW")
                {
                    let directory = std::path::Path::new(&path);
                    std::fs::create_dir_all(directory).unwrap();
                    crate::sheathing::tests::capture_pose(
                        game,
                        &directory.join(format!("hero-{character}-clip-{id:x}.png")),
                    );
                }
            }
        }
        assert_eq!(
            actions,
            [
                MOT_ATTACK,
                MOT_ATTACK_SLASH2,
                MOT_ATTACK_SLASH3,
                MOT_ATTACK_SLASH4
            ]
            .into()
        );
        assert_eq!(contacts, [0xb8, 0xb9, 0x4bd, 0x4be].into());
        assert!(
            app.campaign.game.roster.get(enemy).health < 9999,
            "Imported punches missed hero {character}'s target"
        );
        for part in [8, 9, 11, 12, 13, 14] {
            assert!(
                app.campaign.game.scene.models[app.campaign.game.player().parts[part]]
                    .conflict
                    .is_none()
            );
        }
        let clips = app.campaign.game.mission.clips.len();
        let attacks = app.campaign.game.mission.weapons.attacks.len();
        for style in [
            UnarmedStyle::Onikage,
            UnarmedStyle::Tatsumaru,
            UnarmedStyle::Onikage,
            UnarmedStyle::Tatsumaru,
        ] {
            app.set_sheathing(Sheathing {
                unarmed_style: style,
                ..app.settings.sheathing
            })
            .unwrap();
        }
        assert!(app.campaign.game.mission.clips.len() <= clips + 10);
        assert_eq!(app.campaign.game.mission.weapons.attacks.len(), attacks);
        press_sheathe(&mut app);
        app.game_tick(input::SQUARE);
        let game = &app.campaign.game;
        assert!(game.mission.clips[game.player().motion.clip.unwrap()].id < 0x6000);
    }
}
