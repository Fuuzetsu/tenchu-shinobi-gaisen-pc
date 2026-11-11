use super::*;
use crate::tenchu2::native::{campaign::Content, tools::Tools};
use tenchu_core::tenchu2::input::{ATTACK, CROUCH, GUARD, JUMP, UP};

fn ready(disc: &Disc<impl ReadAt>, sequence: usize) -> Session {
    let mut session = Session::load(disc, sequence, 17).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    session
}

fn walk(session: &mut Session, held: bool) {
    session.world.controls.walking.as_mut().unwrap().held = held;
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; optional native walk clips preserve action transitions, exact checkpoint continuation and original defaults"]
fn native_walking_motions_and_checkpoints() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for (sequence, speed) in [(9, 38), (119, 31), (191, 38)] {
        let mut session = ready(&disc, sequence);
        let before = postcard::to_allocvec(&session.world).unwrap();
        session.configure_walking(&disc, true).unwrap();
        session.configure_walking(&disc, false).unwrap();
        assert!(
            before == postcard::to_allocvec(&session.world).unwrap(),
            "configuring walking mutated idle source state"
        );
        let initial = session.capture_state().unwrap();
        session.configure_walking(&disc, true).unwrap();
        let walking = session.world.controls.walking.unwrap().clip;
        assert_eq!(
            session.assets.resources.clips[&walking].forward_speed,
            speed
        );
        walk(&mut session, true);
        for _ in 0..12 {
            session.step(UP).unwrap();
            let actor = &session.world.actors[session.world.player].actor;
            assert_eq!(actor.motion.clip, Some(walking));
            let expected = session
                .assets
                .resources
                .math
                .velocity(actor.rotation.y, speed, 0);
            assert_eq!(
                [actor.velocity.x, actor.velocity.z],
                [expected[0], expected[2]]
            );
        }
        let saved = session.capture_state().unwrap();
        let mut restored = ready(&disc, sequence);
        restored.restore_state(&saved).unwrap();
        let mut recorder = crate::rewind::Recorder::default();
        recorder.begin();
        session.snapshot().capture(&mut recorder).unwrap();
        assert!(
            recorder.finish().unwrap().bytes().unwrap()
                == postcard::to_allocvec(&session.snapshot()).unwrap()
        );
        for tick in 0..25 {
            let held = tick < 10;
            walk(&mut session, held);
            walk(&mut restored, held);
            let buttons = if tick < 20 { UP } else { 0 };
            session.step(buttons).unwrap();
            restored.step(buttons).unwrap();
            assert!(
                postcard::to_allocvec(&session.snapshot()).unwrap()
                    == postcard::to_allocvec(&restored.snapshot()).unwrap()
            );
            if tick == 10 {
                let actor = &session.world.actors[session.world.player].actor;
                assert_eq!(
                    actor.motion.clip,
                    session
                        .assets
                        .resources
                        .registry
                        .clip(actor.definition_kind(), actor.motion.action)
                );
                assert_ne!(actor.motion.clip, Some(walking));
            }
        }
        for buttons in [UP | JUMP, ATTACK, GUARD, CROUCH] {
            session.restore_state(&initial).unwrap();
            session.configure_walking(&disc, true).unwrap();
            walk(&mut session, true);
            session.step(buttons).unwrap();
            let actor = &session.world.actors[session.world.player].actor;
            assert!(
                !matches!(actor.motion.action, 3 | 0x103),
                "{sequence}: {buttons:04x}"
            );
            assert_eq!(
                actor.motion.clip,
                session
                    .assets
                    .resources
                    .registry
                    .clip(actor.definition_kind(), actor.motion.action)
            );
        }
        session.restore_state(&initial).unwrap();
        session.configure_walking(&disc, true).unwrap();
        walk(&mut session, true);
        session.step(UP).unwrap();
        let random = session.world.random;
        let tick = session.world.tick;
        session.configure_walking(&disc, false).unwrap();
        let actor = &session.world.actors[session.world.player].actor;
        assert_eq!(
            actor.motion.clip,
            session
                .assets
                .resources
                .registry
                .clip(actor.definition_kind(), actor.motion.action)
        );
        assert_eq!(session.world.random, random);
        assert_eq!(session.world.tick, tick);
        session.restore_state(&initial).unwrap();
        restored.restore_state(&initial).unwrap();
        session.configure_walking(&disc, true).unwrap();
        walk(&mut session, true);
        session.world.script = 3;
        restored.world.script = 3;
        for _ in 0..10 {
            session.step(UP).unwrap();
            restored.step(UP).unwrap();
            let mut compared = session.world.clone();
            compared.controls.walking = None;
            assert!(
                postcard::to_allocvec(&compared).unwrap()
                    == postcard::to_allocvec(&restored.world).unwrap()
            );
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; shared native host input supports analog hysteresis, modifier walking, camera-relative movement and input suppression"]
fn native_walking_shared_host_input() {
    use crate::{controls::Controls, settings::Settings};
    use glam::Vec2;
    use winit::keyboard::KeyCode;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for relative in [false, true] {
        let mut session = ready(&disc, 9);
        session.configure_walking(&disc, true).unwrap();
        let mut content = Content::Gameplay(Box::new(session));
        let mut controls = Controls::default();
        let mut tools = Tools::default();
        let mut settings = Settings::default();
        settings.walking.enabled = true;
        settings.navigation.camera_relative = relative;
        tools.sync(&content, 0);
        for (magnitude, walking) in [
            (0.4, true),
            (0.64, true),
            (1.0, false),
            (0.64, false),
            (0.4, true),
        ] {
            let raw = f32::from(settings.navigation.deadzone) / 100.0;
            let raw = raw + (1.0 - raw) * magnitude;
            controls.touch(crate::touch::Sample {
                sticks: [Vec2::new(0.0, raw), Vec2::ZERO],
                ..Default::default()
            });
            let buttons = tools.input(&mut content, &mut controls, &settings, true);
            let Content::Gameplay(session) = &mut content else {
                unreachable!()
            };
            assert_eq!(
                session.world.controls.walking.unwrap().held,
                walking,
                "relative {relative}, magnitude {magnitude}"
            );
            session.step(buttons).unwrap();
            let actor = &session.world.actors[session.world.player].actor;
            assert!(matches!(actor.motion.action, 3 | 0x103));
        }
        controls.touch(Default::default());
        controls.key(KeyCode::KeyW, true);
        controls.key(KeyCode::AltLeft, true);
        assert_eq!(
            tools.input(&mut content, &mut controls, &settings, true),
            UP
        );
        let Content::Gameplay(session) = &content else {
            unreachable!()
        };
        assert!(session.world.controls.walking.unwrap().held);
        assert_eq!(
            tools.input(&mut content, &mut controls, &settings, false),
            0
        );
        let Content::Gameplay(session) = &content else {
            unreachable!()
        };
        assert!(!session.world.controls.walking.unwrap().held);
        controls.clear();
        assert_eq!(tools.input(&mut content, &mut controls, &settings, true), 0);
        let Content::Gameplay(session) = &content else {
            unreachable!()
        };
        assert!(!session.world.controls.walking.unwrap().held);
    }
}

#[test]
#[ignore = "requires both discs; walking retains imported weapons and restores each selected moveset's run"]
fn native_walking_with_imported_movesets() {
    use crate::{settings::Settings, visuals::appearance::library::Library};
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = super::super::models::donor(&gaisen).unwrap();
    for id in [
        "tenchu2-jp:rikimaru",
        "tenchu2-jp:ayamea",
        "tenchu2-jp:tatumaru",
        "disc:0",
    ] {
        let mut session = ready(&disc, 9);
        let settings = Settings {
            model_moveset: true,
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
        session.configure_walking(&disc, true).unwrap();
        let weapons = session.world.instances[session.world.player].rig.weapons;
        walk(&mut session, true);
        for _ in 0..10 {
            session.step(UP).unwrap();
        }
        let actor = &session.world.actors[session.world.player].actor;
        assert_eq!(
            actor.motion.clip,
            Some(session.world.controls.walking.unwrap().clip)
        );
        assert_eq!(
            weapons,
            session.world.instances[session.world.player].rig.weapons
        );
        walk(&mut session, false);
        session.step(UP).unwrap();
        let actor = &session.world.actors[session.world.player].actor;
        assert_eq!(
            actor.motion.clip,
            session
                .assets
                .resources
                .registry
                .clip(actor.definition_kind(), actor.motion.action)
        );
        session.configure_walking(&disc, false).unwrap();
        assert_eq!(
            weapons,
            session.world.instances[session.world.player].rig.weapons
        );
    }
}

#[test]
#[ignore = "requires both discs; native walking works after an imported T1 cinematic and retains its original director"]
fn native_walking_on_imported_stages() {
    let source = super::super::gaisen::Source::open(
        std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
        0,
        0,
        2,
        tenchu_assets::script::Language::English,
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut playback =
        super::super::campaign::Playback::gaisen(disc, source, 2, 2, 17, Some([0; 21]), 6000)
            .unwrap();
    let mut settings = crate::settings::Settings::default();
    settings.walking.enabled = true;
    playback.configure(&settings).unwrap();
    playback.step(0).unwrap();
    let Content::Gameplay(session) = &mut playback.content else {
        unreachable!()
    };
    let spawn = session.world.actors[session.world.player].actor.position;
    let scene = session.gaisen.as_ref().unwrap();
    let (event, target) = scene.director.source.events.active().next().unwrap();
    let event = &scene.director.source.mission.scripts.events[event];
    assert_eq!(event.mode, 1);
    let index = scene.director.native_actor(target).unwrap();
    session.world.actors[index].actor.position = event
        .bounds
        .map(|[low, high]| (i32::from(low) + i32::from(high)) / 2 * 1000)
        .into();
    for _ in 0..2000 {
        playback.step(0).unwrap();
        if playback.content.cutscene() {
            break;
        }
    }
    assert!(playback.content.cutscene());
    for _ in 0..6000 {
        playback.step(0).unwrap();
        if !playback.content.cutscene() {
            break;
        }
    }
    let Content::Gameplay(session) = &mut playback.content else {
        unreachable!()
    };
    assert!(!session.cinematic());
    session
        .world
        .teleport_player(spawn, true, &session.assets)
        .unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    walk(session, true);
    for _ in 0..10 {
        session.step(UP).unwrap();
    }
    let actor = &session.world.actors[session.world.player].actor;
    assert_eq!(
        actor.motion.clip,
        Some(session.world.controls.walking.unwrap().clip)
    );
    let before = session.capture_state().unwrap();
    session.step(0).unwrap();
    session.restore_state(&before).unwrap();
    assert!(session.gaisen.is_some());
    assert!(session.world.controls.walking.unwrap().held);
    assert!(session.capture_state().unwrap() == before);
}
