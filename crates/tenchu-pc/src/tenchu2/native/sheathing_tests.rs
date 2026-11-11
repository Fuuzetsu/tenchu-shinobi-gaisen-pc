use super::{Session, campaign::Content, tools::Tools};
use crate::{controls::Controls, settings::Settings};
use tenchu_core::{
    game::sheathing::Settings as Sheathing,
    tenchu2::{actor::Velocity, input::*, rig::WeaponMode},
};

mod damage;

pub(super) fn disc() -> tenchu_assets::disc::Disc<crate::source::DiscSource> {
    crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap()
}

pub(super) fn ready(sequence: usize) -> Session {
    let mut session = Session::load(&disc(), sequence, 17).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    session
}

fn configure(session: &mut Session, settings: Sheathing) {
    session
        .world
        .set_player_sheathing(settings, &session.assets)
        .unwrap();
}

pub(super) fn weapon(session: &mut Session, drawn: bool) {
    let world = &mut session.world;
    let id = world.player;
    let actor = &mut world.actors[id].actor;
    actor
        .set_weapon(
            &mut world.instances[id].rig,
            id,
            &mut world.collision,
            &session.assets.resources,
            if drawn {
                WeaponMode::Draw
            } else {
                WeaponMode::Sheathe
            },
        )
        .unwrap();
    actor
        .change_motion(
            if drawn { 0x100 } else { 0 },
            Velocity::Stop,
            &session.assets.resources,
        )
        .unwrap();
    actor.pad = Default::default();
    world.publish(id, &session.assets);
}

fn input(session: &mut Session, held: bool) {
    session.world.actors[session.world.player]
        .actor
        .sheathing
        .as_mut()
        .unwrap()
        .input(held);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; manual draw/stow uses native event frames and audible sounds, with exact checkpoint continuation for all heroes"]
fn native_sheathing_source_events_and_checkpoints() {
    for sequence in [9, 119, 191] {
        let mut session = ready(sequence);
        let before = postcard::to_allocvec(&session.world).unwrap();
        configure(
            &mut session,
            Sheathing {
                enabled: true,
                ..Default::default()
            },
        );
        configure(&mut session, Default::default());
        assert!(before == postcard::to_allocvec(&session.world).unwrap());
        for drawn in [true, false] {
            weapon(&mut session, !drawn);
            configure(
                &mut session,
                Sheathing {
                    enabled: true,
                    ..Default::default()
                },
            );
            let player = session.world.player;
            let actor = &session.world.actors[player].actor;
            let action = if drawn { 0x700 } else { 0x701 };
            let clip = session
                .assets
                .resources
                .registry
                .clip(actor.definition_kind(), action)
                .unwrap();
            let frame = session
                .assets
                .resources
                .registry
                .weapon_frame(clip, session.assets.resources.clips[&clip].duration);
            let sound = (session.world.instances[player].voice << 4) | u8::from(!drawn);
            input(&mut session, true);
            let mut restored = ready(sequence);
            restored
                .restore_state(&session.capture_state().unwrap())
                .unwrap();
            let mut swaps = Vec::new();
            let mut audible = false;
            for tick in 0..150 {
                let actor = &session.world.actors[player].actor;
                let before = actor.weapon_drawn();
                let motion_frame = actor.motion.frame;
                for event in session.step(0).unwrap() {
                    if event.sound.id == sound {
                        let bank = event.bank.as_ref().unwrap_or(&session.bank);
                        let mut mixer = tenchu_audio::Mixer::new(48000);
                        mixer.play_at_from(
                            bank,
                            usize::from(event.sound.slot),
                            i16::from(event.sound.id),
                            event.sound.volume,
                            event.sound.note,
                        );
                        audible |=
                            (0..24000).any(|_| mixer.frame().iter().any(|v| v.abs() > 0.001));
                    }
                }
                restored.step(0).unwrap();
                if before != session.world.actors[player].actor.weapon_drawn() {
                    swaps.push(motion_frame);
                }
                assert!(
                    postcard::to_allocvec(&session.world).unwrap()
                        == postcard::to_allocvec(&restored.world).unwrap(),
                    "{sequence} {action:x} tick {tick}"
                );
                if tick == 4 {
                    restored
                        .restore_state(&session.capture_state().unwrap())
                        .unwrap();
                    let mut recorder = crate::rewind::Recorder::default();
                    recorder.begin();
                    session.snapshot().capture(&mut recorder).unwrap();
                    assert!(
                        recorder.finish().unwrap().bytes().unwrap()
                            == postcard::to_allocvec(&session.snapshot()).unwrap()
                    );
                }
                // Repeated presentation samples of a held key must not retrigger.
                input(&mut session, true);
                input(&mut restored, true);
            }
            assert!(session.capture_state().unwrap() == restored.capture_state().unwrap());
            assert_eq!(swaps, [frame], "sequence {sequence}, action {action:x}");
            assert_eq!(
                audible,
                sequence != 191,
                "sequence {sequence}, action {action:x}"
            );
            assert_eq!(
                session.world.actors[player].actor.motion.action,
                if drawn { 0x100 } else { 0 }
            );
            input(&mut session, false);
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; shared host binding latches between ticks, clears inactive input and cannot interrupt committed actions or scripts"]
fn native_sheathing_binding_and_committed_actions() {
    use winit::keyboard::KeyCode;
    for relative in [false, true] {
        let mut session = ready(9);
        weapon(&mut session, true);
        let mut settings = Settings::default();
        settings.sheathing.enabled = true;
        settings.navigation.camera_relative = relative;
        configure(&mut session, settings.sheathing);
        let mut content = Content::Gameplay(Box::new(session));
        let mut tools = Tools::default();
        tools.sync(&content, 0);
        let mut controls = Controls::default();
        controls.key(KeyCode::KeyH, true);
        for _ in 0..3 {
            assert_eq!(tools.input(&mut content, &mut controls, &settings, true), 0);
        }
        controls.key(KeyCode::KeyH, false);
        assert_eq!(tools.input(&mut content, &mut controls, &settings, true), 0);
        let Content::Gameplay(session) = &mut content else {
            unreachable!()
        };
        session.step(0).unwrap();
        assert_eq!(
            session.world.actors[session.world.player]
                .actor
                .motion
                .action,
            0x701
        );
        weapon(session, true);
        controls.key(KeyCode::KeyH, true);
        tools.input(&mut content, &mut controls, &settings, true);
        tools.input(&mut content, &mut controls, &settings, false);
        let Content::Gameplay(session) = &mut content else {
            unreachable!()
        };
        session.step(0).unwrap();
        assert_eq!(
            session.world.actors[session.world.player]
                .actor
                .motion
                .action,
            0x100
        );
    }
    let mut session = ready(9);
    weapon(&mut session, true);
    configure(
        &mut session,
        Sheathing {
            enabled: true,
            ..Default::default()
        },
    );
    let saved = session.capture_state().unwrap();
    for (buttons, script) in [(ATTACK, 0), (UP | JUMP, 0), (0, 1), (0, 3)] {
        session.restore_state(&saved).unwrap();
        session.world.script = script;
        let mut reference = ready(9);
        reference
            .restore_state(&session.capture_state().unwrap())
            .unwrap();
        input(&mut session, true);
        for tick in 0..12 {
            session.step(if tick == 0 { buttons } else { 0 }).unwrap();
            reference.step(if tick == 0 { buttons } else { 0 }).unwrap();
            let mut actual = session.world.clone();
            actual.actors[actual.player].actor.sheathing = None;
            let mut expected = reference.world.clone();
            expected.actors[expected.player].actor.sheathing = None;
            assert!(
                postcard::to_allocvec(&actual).unwrap()
                    == postcard::to_allocvec(&expected).unwrap(),
                "buttons {buttons:x}, script {script}, tick {tick}"
            );
        }
    }
    for water in [false, true] {
        use tenchu_core::tenchu2::{
            mission::{Outcome, Phase},
            physics::{Depth, IN_WATER},
        };
        session.restore_state(&saved).unwrap();
        let player = session.world.player;
        if water {
            session.world.water.depth = Some(Depth::Swimming);
            session.world.actors[player].actor.flags |= IN_WATER;
        } else {
            session.world.mission.phase = Phase::Playing {
                pending: Some(Outcome::Arrived),
            };
        }
        input(&mut session, true);
        session
            .world
            .control_actor(
                player,
                &mut session.assets,
                &mut super::bleeding_tests::Recorded::default(),
            )
            .unwrap();
        assert!(!matches!(
            session.world.actors[player].actor.motion.action,
            0x700 | 0x701
        ));
        session.world.water.depth = None;
        session.world.actors[player].actor.flags &= !IN_WATER;
        session.world.mission.phase = Phase::Playing { pending: None };
        weapon(&mut session, true);
        session
            .world
            .control_actor(
                player,
                &mut session.assets,
                &mut super::bleeding_tests::Recorded::default(),
            )
            .unwrap();
        assert_eq!(
            session.world.actors[player].actor.motion.action, 0x100,
            "queued request survived input lock/water entry"
        );
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; stowed speed scales actual running/walking velocity once, restores immediately and leaves armed/committed actions unchanged"]
fn native_sheathing_movement_modifiers() {
    for sequence in [9, 119, 191] {
        let mut session = ready(sequence);
        weapon(&mut session, false);
        let saved = session.capture_state().unwrap();
        for walking in [false, true] {
            for speed in [50, 125, 200] {
                for buttons in [UP, DOWN] {
                    session.restore_state(&saved).unwrap();
                    session.configure_walking(&disc(), walking).unwrap();
                    if let Some(walk) = &mut session.world.controls.walking {
                        walk.held = true;
                    }
                    configure(
                        &mut session,
                        Sheathing {
                            enabled: true,
                            speed,
                            ..Default::default()
                        },
                    );
                    for _ in 0..5 {
                        session.step(buttons).unwrap();
                        let actor = &session.world.actors[session.world.player].actor;
                        let clip = &session.assets.resources.clips[&actor.motion.clip.unwrap()];
                        let expected = session.assets.resources.math.velocity(
                            actor.rotation.y,
                            (i32::from(clip.forward_speed) * i32::from(speed) / 100) as i16,
                            (i32::from(clip.side_speed) * i32::from(speed) / 100) as i16,
                        );
                        assert_eq!(
                            [actor.velocity.x, actor.velocity.z],
                            [expected[0], expected[2]]
                        );
                    }
                    let random = session.world.random;
                    let tick = session.world.tick;
                    if walking {
                        session.configure_walking(&disc(), false).unwrap();
                        let actor = &session.world.actors[session.world.player].actor;
                        let clip = &session.assets.resources.clips[&actor.motion.clip.unwrap()];
                        let expected = session.assets.resources.math.velocity(
                            actor.rotation.y,
                            (i32::from(clip.forward_speed) * i32::from(speed) / 100) as i16,
                            (i32::from(clip.side_speed) * i32::from(speed) / 100) as i16,
                        );
                        assert_eq!(
                            [actor.velocity.x, actor.velocity.z],
                            [expected[0], expected[2]],
                            "walking disabled with sheathed speed enabled"
                        );
                    }
                    configure(&mut session, Default::default());
                    let actor = &session.world.actors[session.world.player].actor;
                    let clip = &session.assets.resources.clips[&actor.motion.clip.unwrap()];
                    let expected = session.assets.resources.math.velocity(
                        actor.rotation.y,
                        clip.forward_speed,
                        clip.side_speed,
                    );
                    assert_eq!(
                        [actor.velocity.x, actor.velocity.z],
                        [expected[0], expected[2]]
                    );
                    assert_eq!(session.world.random, random);
                    assert_eq!(session.world.tick, tick);
                }
            }
        }
        for (drawn, buttons) in [
            (true, UP),
            (false, UP | JUMP),
            (false, CROUCH | UP),
            (false, GUARD | UP),
        ] {
            session.restore_state(&saved).unwrap();
            weapon(&mut session, drawn);
            let mut reference = ready(sequence);
            reference
                .restore_state(&session.capture_state().unwrap())
                .unwrap();
            configure(
                &mut session,
                Sheathing {
                    enabled: true,
                    speed: 200,
                    ..Default::default()
                },
            );
            for _ in 0..5 {
                session.step(buttons).unwrap();
                reference.step(buttons).unwrap();
                let mut actual = session.world.clone();
                actual.actors[actual.player].actor.sheathing = None;
                assert!(
                    postcard::to_allocvec(&actual).unwrap()
                        == postcard::to_allocvec(&reference.world).unwrap()
                );
            }
        }
        for script in [1, 3] {
            session.restore_state(&saved).unwrap();
            session.world.script = script;
            let player = session.world.player;
            session.world.actors[player]
                .actor
                .change_motion(3, Velocity::Motion, &session.assets.resources)
                .unwrap();
            session.world.actors[player].actor.velocity.x = 83;
            session.world.actors[player].actor.velocity.z = -19;
            let before = postcard::to_allocvec(&session.world).unwrap();
            configure(
                &mut session,
                Sheathing {
                    enabled: true,
                    speed: 200,
                    ..Default::default()
                },
            );
            configure(&mut session, Default::default());
            assert!(
                postcard::to_allocvec(&session.world).unwrap() == before,
                "configuration changed scripted velocity"
            );
            configure(
                &mut session,
                Sheathing {
                    enabled: true,
                    speed: 200,
                    ..Default::default()
                },
            );
            session.world.actors[session.world.player]
                .actor
                .change_motion(3, Velocity::Motion, &session.assets.resources)
                .unwrap();
            let mut reference = ready(sequence);
            reference
                .restore_state(&session.capture_state().unwrap())
                .unwrap();
            configure(&mut reference, Default::default());
            for _ in 0..10 {
                session.step(UP).unwrap();
                reference.step(UP).unwrap();
                let mut actual = session.world.clone();
                actual.actors[actual.player].actor.sheathing = None;
                assert!(
                    postcard::to_allocvec(&actual).unwrap()
                        == postcard::to_allocvec(&reference.world).unwrap(),
                    "script {script} movement was scaled"
                );
            }
        }
    }
}
