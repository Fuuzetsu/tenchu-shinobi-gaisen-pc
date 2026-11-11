use super::*;
use tenchu_core::tenchu2::input::{ATTACK, UP};

#[test]
#[ignore = "requires both discs; model/moveset swaps preserve real native melee blocking and its reaction clip"]
fn native_imported_movesets_block_incoming_melee() {
    use tenchu_core::tenchu2::{
        physics::HIT,
        world::controls::{Event, EventSource, Events},
    };
    #[derive(Default)]
    struct Recorded(Vec<Event>);
    impl tenchu_core::tenchu2::environment::Events for Recorded {
        fn emit(
            &mut self,
            _: tenchu_core::tenchu2::environment::Event,
            _: &mut tenchu_core::random::Random,
        ) -> Result<()> {
            Ok(())
        }
    }
    impl Events for Recorded {
        fn emit(
            &mut self,
            _: EventSource<'_>,
            event: Event,
            _: &mut tenchu_core::random::Random,
        ) -> Result<()> {
            self.0.push(event);
            Ok(())
        }
    }
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    for id in [
        "",
        "tenchu2-jp:rikimaru",
        "tenchu2-jp:ayamea",
        "tenchu2-jp:tatumaru",
        "disc:0",
    ] {
        let (world, assets, guard) = crate::tenchu2::tests::native_world::fixture();
        let bank = Arc::new(
            tenchu_assets::tenchu2::audio::Audio::load(&disc, 0, world.map)
                .unwrap()
                .bank,
        );
        let mut session = super::super::Session::from_parts(world, assets, bank);
        let settings = Settings {
            model_moveset: !id.is_empty(),
            moveset_model: id.into(),
            ..Default::default()
        };
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &Arc::default(),
                &settings,
            )
            .unwrap();
        let world = &mut session.world;
        let assets = &mut session.assets;
        for who in [0, guard] {
            let actor = &mut world.actors[who].actor;
            actor
                .set_weapon(
                    &mut world.instances[who].rig,
                    who,
                    &mut world.collision,
                    &assets.resources,
                    WeaponMode::Draw,
                )
                .unwrap();
            actor
                .change_motion(
                    if who == 0 { 0x110 } else { 0x900 },
                    actor::Velocity::Stop,
                    &assets.resources,
                )
                .unwrap();
            actor.motion.frame = 0;
            actor.flags |= 2;
            actor.pad.update(if who == 0 {
                tenchu_core::tenchu2::input::GUARD
            } else {
                0
            });
            actor.attack.index = assets
                .resources
                .weapons
                .attacks
                .iter()
                .position(|a| Some(a.clip) == actor.motion.clip);
            world.publish(who, assets);
        }
        world.finish_tick(assets, 6).unwrap();
        let before = world.actors[0].actor.health;
        let weapon = world
            .collision
            .slots()
            .iter()
            .position(|s| s.owner == Some(guard) && s.flags & 0x4000 != 0)
            .unwrap();
        world.actors[0].actor.flags |= HIT;
        world.actors[0].actor.hit = Some(weapon);
        let mut events = Recorded::default();
        world.control_actor(0, assets, &mut events).unwrap();
        assert_eq!(
            world.actors[0].actor.health, before,
            "moveset {id}: guard took melee damage"
        );
        assert_eq!(
            world.actors[0].actor.motion.action, 0x916,
            "moveset {id}: missing block reaction"
        );
        assert!(
            events.0.iter().any(|event| matches!(
                event,
                Event::Rumble {
                    action: tenchu_core::effects::RumbleAction::Block,
                    ..
                }
            )),
            "moveset {id}: no block impact"
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; native T2 buttons survive body/moveset swaps, with explicit crouch override and idle-taunt suppression"]
fn native_circle_stance_preserves_t2_buttons_and_repeats_steps() {
    use tenchu_core::input::CircleAction;
    use tenchu_core::tenchu2::input::{CROUCH, DOWN, GUARD, LEFT, RIGHT};
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    for hero in 0..3 {
        for moveset in ["", "disc:0", "tenchu2-jp:ayame"] {
            let mut session = super::super::Session::with_options(
                &disc,
                9,
                tenchu_core::tenchu2::world::Options {
                    hero,
                    difficulty: 2,
                    seed: 17,
                    inventory: Default::default(),
                },
                false,
            )
            .unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let settings = Settings {
                player_model: "disc:1".into(),
                moveset_model: moveset.into(),
                model_moveset: !moveset.is_empty(),
                model_moveset_options: Options {
                    no_idle_taunts: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Arc::default(),
                    &settings,
                )
                .unwrap();
            let initial = session.world.clone();
            let player = session.world.player;
            session.navigation = Some(super::super::navigation::Input {
                yaw: 0.63,
                pitch: 0,
                sticks: [glam::Vec2::ZERO; 2],
                deadzone: 18,
                align_aim: true,
            });
            for preference in [
                CircleAction::Moveset,
                CircleAction::Crouch,
                CircleAction::Guard,
            ] {
                session.world = initial.clone();
                session.circle_action = preference;
                let guards = preference.guards(true);
                for _ in 0..5 {
                    session.step(GUARD).unwrap();
                }
                assert_eq!(
                    session.world.actors[player].actor.motion.action,
                    if guards { 0x110 } else { 0x200 },
                    "hero {hero}, moveset {moveset}, preference {preference:?}"
                );
                session.world = initial.clone();
                for _ in 0..5 {
                    session.step(CROUCH).unwrap();
                }
                assert_eq!(
                    session.world.actors[player].actor.motion.action,
                    if guards { 0x200 } else { 0x110 }
                );
                let button = if guards { GUARD } else { CROUCH };
                for (direction, action) in
                    [(UP, 0x111), (DOWN, 0x112), (RIGHT, 0x113), (LEFT, 0x114)]
                {
                    session.world = initial.clone();
                    let mut starts = 0;
                    let yaw = session.world.actors[player].actor.rotation.y;
                    for tick in 0..140 {
                        let previous = session.world.actors[player].actor.motion.action;
                        session.step(button | direction).unwrap();
                        if tick < 2 {
                            assert_eq!(session.world.actors[player].actor.rotation.y, yaw);
                        }
                        starts += usize::from(
                            previous != action
                                && session.world.actors[player].actor.motion.action == action,
                        );
                    }
                    assert!(
                        starts >= 2,
                        "hero {hero}, moveset {moveset}, preference {preference:?}, action {action:x}: {starts}"
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; imported native draw/stow transitions keep source event timing"]
fn native_imported_sheathing_keeps_source_event_frames() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    for id in [
        "tenchu2-jp:rikimaru",
        "tenchu2-jp:ayame",
        "tenchu2-jp:tatumaru",
    ] {
        let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let settings = Settings {
            moveset_model: id.into(),
            model_moveset: true,
            model_moveset_options: Options {
                player_movement: true,
                ..Default::default()
            },
            ..Default::default()
        };
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &Arc::default(),
                &settings,
            )
            .unwrap();
        let source = library.get(id).unwrap().imported_moveset.as_ref().unwrap();
        let owner = session.world.player;
        for (drawn, action, host_action) in [
            (true, 0x700, tenchu_core::ids::MOT_STATE_DRAW),
            (false, 0x701, tenchu_core::ids::MOT_STATE_SHEATHE),
        ] {
            let source_id = source
                .character
                .motions
                .iter()
                .find(|m| m.action == host_action)
                .unwrap()
                .clip;
            let expected_frame = source
                .weapon_frames
                .iter()
                .find(|&&(id, _)| id == source_id)
                .unwrap()
                .1
                .frame;
            let actor = &mut session.world.actors[owner].actor;
            actor
                .set_weapon(
                    &mut session.world.instances[owner].rig,
                    owner,
                    &mut session.world.collision,
                    &session.assets.resources,
                    if drawn {
                        WeaponMode::Sheathe
                    } else {
                        WeaponMode::Draw
                    },
                )
                .unwrap();
            actor
                .change_motion(action, actor::Velocity::Motion, &session.assets.resources)
                .unwrap();
            assert!(actor.motion.clip.unwrap() < 0);
            let mut swaps = Vec::new();
            let mut audible = false;
            for _ in 0..150 {
                let actor = &session.world.actors[owner].actor;
                let before = actor.weapon_drawn();
                let frame = actor.motion.frame;
                for event in session.step(0).unwrap() {
                    if event.sound.id == u8::from(!drawn)
                        && let Some(bank) = event.bank
                    {
                        let mut pcm = tenchu_audio::Mixer::new(48000);
                        pcm.play_at_from(
                            &bank,
                            usize::from(event.sound.slot),
                            i16::from(event.sound.id),
                            event.sound.volume,
                            event.sound.note,
                        );
                        audible |= (0..24000).any(|_| pcm.frame().iter().any(|v| v.abs() > 0.001));
                    }
                }
                if before != session.world.actors[owner].actor.weapon_drawn() {
                    swaps.push(frame);
                }
            }
            assert_eq!(swaps, [expected_frame], "{id} action {action:x}");
            assert_eq!(
                audible,
                source
                    .weapon_frames
                    .iter()
                    .find(|&&(clip, _)| clip == source_id)
                    .unwrap()
                    .1
                    .sound,
                "{id} action {action:x}: weapon sound"
            );
            assert_eq!(
                session.world.actors[owner].actor.motion.action,
                if drawn { 0x100 } else { 0 }
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; cross-game motion, contacts, rig reuse and restoration"]
fn cross_game_models_move_attack_and_restore_in_native_t2() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = crate::session::Options::default().load(&gaisen).unwrap();
    for _ in 0..30 {
        donor.step(0).unwrap();
    }
    let mut session = super::super::Session::load(&disc, 9, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let owner = session.world.player;
    let original = session.world.actors[owner].actor.kind;
    let inventory = session.world.actors[owner].actor.inventory.clone();
    let onikage = library
        .bodies
        .iter()
        .find(|b| b.resource == "HUMAN/NINJA.MAD")
        .unwrap()
        .id
        .clone();
    let guard = library
        .bodies
        .iter()
        .find(|b| b.id.starts_with("tenchu2-jp:") && b.has_moveset() && b.id.ends_with(":busi"))
        .map(|b| b.id.clone());
    let mut choices = vec![
        "disc:0".into(),
        "disc:1".into(),
        onikage,
        "tenchu2-jp:rikimaru".into(),
        "tenchu2-jp:ayame".into(),
        "tenchu2-jp:tatumaru".into(),
    ];
    choices.extend(guard);
    choices.push("tenchu2-jp:tatu2".into());
    for id in &choices {
        let settings = Settings {
            player_model: id.clone(),
            model_moveset: true,
            model_moveset_options: Options {
                no_idle_taunts: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let rng = session.world.random;
        assert!(
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Arc::default(),
                    &settings
                )
                .unwrap(),
            "swap {id}"
        );
        assert_eq!(rng, session.world.random);
        let voice = library
            .get(id)
            .unwrap()
            .moveset_voice()
            .expect("moveset voice");
        assert!(std::sync::Arc::ptr_eq(
            session.models.voice.as_ref().unwrap(),
            voice
        ));
        assert_eq!(session.world.actors[owner].actor.kind, original);
        if id.starts_with("tenchu2-jp:") {
            for source in session.assets.weapon_imports.values() {
                assert!(
                    source.id.starts_with("tenchu2-jp:weapons/"),
                    "{id}: {}",
                    source.id
                );
                assert!(source.source_id.starts_with("tenchu2/weapons/"));
            }
        }
        assert_eq!(
            postcard::to_allocvec(&session.world.actors[owner].actor.inventory).unwrap(),
            postcard::to_allocvec(&inventory).unwrap()
        );
        let start = session.world.actors[owner].actor.position;
        for _ in 0..12 {
            session.step(UP).unwrap();
        }
        assert_ne!(
            session.world.actors[owner].actor.position, start,
            "move {id}"
        );
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let mut heard_voice = false;
        let mut seen = BTreeSet::new();
        let mut contacts = BTreeSet::new();
        let yaw = session.world.actors[owner].actor.rotation.y;
        // The first attack draws a sheathed weapon before the actual strike.
        session.step(ATTACK).unwrap();
        for tick in 0..400 {
            let player = &session.world.actors[owner].actor;
            let attack = player
                .motion
                .clip
                .and_then(|id| session.assets.resources.weapons.attack(id));
            let input = if attack.is_some_and(|a| player.motion.frame == i16::from(a.combo))
                || tick == 70
            {
                ATTACK
            } else {
                0
            };
            for event in session.step(input).unwrap() {
                if let Some(bank) = event.bank {
                    assert!(std::sync::Arc::ptr_eq(&bank, &voice.bank));
                    assert!(event.sound.id < 16);
                    heard_voice = true;
                }
            }
            let player = &session.world.actors[owner].actor;
            if player.imported_attack().is_some() {
                seen.insert(player.motion.action);
            }
            if player.attack.contacts != 0 {
                contacts.insert(player.motion.action);
            }
        }
        assert!(seen.contains(&0x900), "{id}: {seen:?}");
        assert!(contacts.contains(&0x900), "{id}: no contacts: {contacts:?}");
        assert!(heard_voice, "no imported voice for {id}");
        let turn =
            (session.world.actors[owner].actor.rotation.y - yaw + 2048).rem_euclid(4096) - 2048;
        // Gaisen commits authored root turns; T2's forward chain preserves yaw.
        if id.starts_with("disc:") {
            let mut reference = super::donor(&gaisen).unwrap();
            library
                .get(id)
                .unwrap()
                .apply_moveset(&mut reference, Options::default())
                .unwrap();
            let initial = reference.player().rotation[1];
            reference.step(tenchu_core::input::SQUARE).unwrap();
            for tick in 0..400 {
                let player = reference.player();
                let attack = player.motion.clip.and_then(|index| {
                    reference
                        .mission
                        .weapons
                        .attacks
                        .iter()
                        .find(|a| a.clip == reference.mission.clips[index].id)
                });
                let press = tick == 70 || attack.is_some_and(|a| player.motion.count == a.combo);
                reference
                    .step(if press { tenchu_core::input::SQUARE } else { 0 })
                    .unwrap();
            }
            let expected =
                (reference.player().rotation[1] - initial + 2048).rem_euclid(4096) - 2048;
            assert_eq!(turn, expected, "Gaisen combo recovery for {id}");
        } else {
            assert_eq!(turn, 0, "forward combo turns {id}");
        }
        assert!(
            matches!(
                session.world.actors[owner].actor.state,
                State::Normal | State::Ready
            ),
            "recovery {id}"
        );
        if let Some(dir) = std::env::var_os("TENCHU_MODEL_REVIEW") {
            use glam::{Mat4, Vec3};
            let path =
                std::path::Path::new(&dir).join(format!("native-{}.png", id.replace(':', "-")));
            std::fs::create_dir_all(&dir).unwrap();
            let graphics = crate::settings::Graphics {
                fog: false,
                ..Default::default()
            };
            let mut review = crate::capture::review::Review::new(&donor.mission).unwrap();
            review.configure(graphics.clone());
            let models = super::super::render::Models::new(
                &session.assets,
                &session.world,
                &graphics,
                &mut review.renderer,
            )
            .unwrap();
            let geometry = models
                .geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Default::default(),
                )
                .unwrap();
            let center = Vec3::from(
                session.world.actors[owner]
                    .actor
                    .position
                    .to_array()
                    .map(|v| v as f32),
            ) + Vec3::new(0.0, -900.0, 0.0);
            let camera = Mat4::perspective_rh(42f32.to_radians(), 16.0 / 9.0, 10.0, 20000.0)
                * Mat4::look_at_rh(
                    center + Vec3::new(3300.0, -250.0, 1800.0),
                    center,
                    Vec3::NEG_Y,
                );
            let mesh = review.renderer.upload(&geometry.actors);
            review
                .capture(camera, &[&mesh], None, false, &path)
                .unwrap();
        }
    }
    assert!(
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &Arc::default(),
                &Settings::default()
            )
            .unwrap()
    );
    assert!(session.world.actors[owner].actor.moveset.is_none());
    assert!(session.models.voice.is_none());
    for _ in 0..30 {
        session.step(UP).unwrap();
    }
    let size = session.world.scene.models.len();
    for _ in 0..40 {
        for model in ["disc:0", "disc:1", ""] {
            let settings = Settings {
                player_model: model.into(),
                model_moveset: !model.is_empty(),
                ..Default::default()
            };
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Arc::default(),
                    &settings,
                )
                .unwrap();
        }
    }
    assert_eq!(
        session.world.scene.models.len(),
        size,
        "repeated swaps leak scene slots"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; imported sword, dual blade and unarmed attacks inflict actual damage"]
fn imported_native_attacks_damage_guards() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    let onikage = library
        .bodies
        .iter()
        .find(|b| b.resource == "HUMAN/NINJA.MAD")
        .unwrap()
        .id
        .clone();
    for id in [
        "disc:0",
        "disc:1",
        &onikage,
        "tenchu2-jp:ayame",
        "tenchu2-jp:tatumaru",
        "tenchu2-jp:busi",
        "tenchu2-jp:tatu2",
    ] {
        let mut session = super::super::Session::load(&disc, 9, 1).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let settings = Settings {
            player_model: id.into(),
            model_moveset: true,
            ..Default::default()
        };
        assert!(
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Arc::default(),
                    &settings
                )
                .unwrap()
        );
        let owner = session.world.player;
        let player = &mut session.world.actors[owner].actor;
        player.health = 30000;
        let p = player.position;
        let yaw = player.rotation.y;
        let forward = session.assets.resources.math.velocity(yaw, 1000, 0);
        session.world.actors[1].behavior.has_suspicious_program = true;
        let guard = &mut session.world.actors[1].actor;
        guard.flags &= !(0x10 | 8);
        guard.velocity = Default::default();
        guard.position = p.wrapping_add(forward.map(i32::from).into());
        guard.previous = guard.position;
        guard.rotation.y = yaw.wrapping_add(2048);
        guard.health = 30000;
        guard
            .set_weapon(
                &mut session.world.instances[1].rig,
                1,
                &mut session.world.collision,
                &session.assets.resources,
                WeaponMode::Draw,
            )
            .unwrap();
        guard
            .set_body_collision(
                session.world.instances[1].rig.joints[0],
                1,
                &mut session.world.collision,
                true,
            )
            .unwrap();
        session.world.publish(1, &session.assets);
        // Draw first, then attack without a paired stealth kill.
        session.step(ATTACK).unwrap();
        let mut began_attack = false;
        for _ in 0..240 {
            let player = &session.world.actors[owner].actor;
            let attack = player
                .motion
                .clip
                .and_then(|id| session.assets.resources.weapons.attack(id));
            let press = (!began_attack && player.state == State::Ready && player.weapon_drawn())
                || attack.is_some_and(|a| player.motion.frame == i16::from(a.combo));
            began_attack |= press;
            session.step(if press { ATTACK } else { 0 }).unwrap();
        }
        assert!(
            session.world.actors[1].actor.health < 30000,
            "no damage from {id}, player {:?}, guard {:?}",
            session.world.actors[owner].actor.position,
            session.world.actors[1].actor.position
        );
        eprintln!(
            "{id}: {} damage",
            30000 - session.world.actors[1].actor.health
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; appearance/movement independence and all native heroes' weapon restoration"]
fn native_heroes_restore_their_weapons_and_movement() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let library = Library::load(&gaisen).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    for hero in 0..3 {
        for drawn in [false, true] {
            let mut session = super::super::Session::with_options(
                &disc,
                9,
                tenchu_core::tenchu2::world::Options {
                    hero,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
            )
            .unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let owner = session.world.player;
            let actor = &mut session.world.actors[owner].actor;
            actor
                .set_weapon(
                    &mut session.world.instances[owner].rig,
                    owner,
                    &mut session.world.collision,
                    &session.assets.resources,
                    if drawn {
                        WeaponMode::Draw
                    } else {
                        WeaponMode::Sheathe
                    },
                )
                .unwrap();
            let weapons = session.world.instances[owner].rig.weapons;
            let original = session
                .assets
                .resources
                .registry
                .actor(actor.kind)
                .unwrap()
                .clone();
            let rng = session.world.random;
            for settings in [
                Settings {
                    player_model: "disc:1".into(),
                    ..Default::default()
                },
                Settings {
                    player_model: "disc:0".into(),
                    model_moveset: true,
                    model_moveset_options: Options {
                        player_movement: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Settings::default(),
            ] {
                assert!(
                    session
                        .models
                        .sync(
                            &mut session.world,
                            &mut session.assets,
                            &mut donor,
                            &library,
                            &Arc::default(),
                            &settings
                        )
                        .unwrap()
                );
                let actor = &session.world.actors[owner].actor;
                let effective = session
                    .assets
                    .resources
                    .registry
                    .actor(actor.definition_kind())
                    .unwrap();
                assert_eq!(effective.turn, original.turn, "hero {hero}: turn speed");
                assert_eq!(
                    session
                        .assets
                        .resources
                        .registry
                        .clip(actor.definition_kind(), 3),
                    session.assets.resources.registry.clip(actor.kind, 3),
                    "hero {hero}: native running clip"
                );
                assert_eq!(actor.weapon_drawn(), drawn, "hero {hero}: weapon state");
                assert_eq!(rng, session.world.random);
                if !settings.model_moveset {
                    assert_eq!(effective.weapon, original.weapon);
                    assert_eq!(
                        session.world.instances[owner].rig.weapons, weapons,
                        "hero {hero}: native attachments"
                    );
                    assert!(actor.imported_attack().is_none());
                }
            }
            assert!(session.world.actors[owner].actor.moveset.is_none());
        }
    }
}
