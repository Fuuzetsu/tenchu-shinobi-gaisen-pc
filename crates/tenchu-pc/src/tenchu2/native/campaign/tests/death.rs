use super::*;
use crate::settings::Settings;
use tenchu_core::tenchu2::{
    actor::{State, Velocity},
    mission::Outcome,
};

fn session(playback: &Playback<crate::source::DiscSource>) -> &Session {
    let Content::Gameplay(session) = &playback.content else {
        panic!("not gameplay")
    };
    session
}

fn start_death(playback: &mut Playback<crate::source::DiscSource>) {
    for _ in 0..30 {
        playback.step(0).unwrap();
    }
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("not gameplay")
    };
    let player = session.world.player;
    let actor = &mut session.world.actors[player].actor;
    actor.health = 0;
    actor.state = State::Dead;
    actor
        .change_motion(0xb01, Velocity::Motion, &session.assets.resources)
        .unwrap();
    actor.motion.frame = 0;
    actor.motion.loops = 0;
    session.world.publish(player, &session.assets);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; real death animations, frozen/live continuation, original failure and retry state"]
fn native_death_hold_preserves_the_end_pose_effects_and_retry_state() {
    for sequence in [9, 119, 191] {
        let mut held = start(sequence);
        let mut original = start(sequence);
        let mut settings = Settings {
            wait_after_death: true,
            ..Default::default()
        };
        held.death.configure(&settings);
        start_death(&mut held);
        start_death(&mut original);
        let generation = held.generation;
        for _ in 0..300 {
            held.step(0).unwrap();
            original.step(0).unwrap();
            assert_eq!(
                postcard::to_allocvec(&session(&held).world).unwrap(),
                postcard::to_allocvec(&session(&original).world).unwrap()
            );
            if held.death.waiting() {
                break;
            }
        }
        assert!(
            held.death.waiting(),
            "sequence {sequence} never finished dying"
        );
        let world = &session(&held).world;
        assert_eq!(world.actors[world.player].actor.motion.loops, i16::MIN);
        let frozen = postcard::to_allocvec(world).unwrap();
        let run = postcard::to_allocvec(&held.run).unwrap();
        let phase = world.mission.phase;
        let tick = world.tick;
        let has_visible_props = !session(&held).frame.scenery.models.is_empty();
        let retry_seed = world.random;
        let inventory = world.actors[world.player].actor.inventory.amounts();
        let statistics = serde_json::to_value(&world.statistics).unwrap();
        let failure = world.mission.failure;
        for _ in 0..180 {
            let update = held.step(0xffff).unwrap();
            assert!(!update.stop_music && update.music.is_none() && update.sounds.is_empty());
        }
        assert_eq!(
            postcard::to_allocvec(&session(&held).world).unwrap(),
            frozen
        );
        assert_eq!(postcard::to_allocvec(&held.run).unwrap(), run);
        assert_eq!(held.generation, generation);
        assert!(session(&held).frame.transition_fade.is_none());
        let input = super::super::super::transport::Input {
            buttons: 0,
            cutscene_speed: 1,
            simulation_speed: 100,
            fast_forward: false,
            paused: false,
        };
        assert_eq!(
            super::super::super::transport::Transport::default().interpolation(&held, input),
            1.0
        );

        settings.live_death_scene = true;
        held.death.configure(&settings);
        let mut scenery_frames = 0;
        let mut pool_frames = 0;
        let mut flame_phases = std::collections::BTreeSet::new();
        for _ in 0..700 {
            let update = held.step(0xffff).unwrap();
            assert!(!update.stop_music && update.music.is_none());
            let s = session(&held);
            assert_eq!(s.world.mission.phase, phase);
            assert!(s.frame.transition_fade.is_none());
            assert_eq!(s.world.actors[s.world.player].actor.pad.held, 0);
            assert_eq!(s.world.actors[s.world.player].actor.health, 0);
            scenery_frames += usize::from(!s.frame.scenery.models.is_empty());
            pool_frames += usize::from(s.frame.effects.quads.iter().any(|q| q.blood.is_some()));
            flame_phases.insert(s.world.environment.flame_frame);
        }
        assert_eq!(session(&held).world.tick, tick + 700);
        assert_eq!(held.generation, generation);
        assert_eq!(postcard::to_allocvec(&held.run).unwrap(), run);
        assert!(
            (!has_visible_props || scenery_frames > 100) && pool_frames > 100,
            "sequence {sequence}: scenery={scenery_frames} pools={pool_frames}"
        );
        assert_eq!(flame_phases.len(), 4);

        settings.live_death_scene = false;
        held.death.configure(&settings);
        let stopped = postcard::to_allocvec(&session(&held).world).unwrap();
        for _ in 0..20 {
            held.step(0).unwrap();
        }
        assert_eq!(
            postcard::to_allocvec(&session(&held).world).unwrap(),
            stopped
        );
        assert!(!held.controller_pause(0, false));
        assert!(!held.controller_pause(tenchu_core::input::CIRCLE, false));
        assert!(held.action.is_none());
        assert!(!held.controller_pause(tenchu_core::input::CROSS, false));
        assert!(held.step(0).unwrap().stop_music);
        assert_eq!(held.generation, generation + 1);
        assert!(!held.death.waiting());
        if let Some(sequence) = failure {
            assert_eq!(held.run.sequence, sequence);
        } else {
            assert!(matches!(held.content, Content::Failed));
        }
        assert_eq!(held.run.random, retry_seed);
        assert_eq!(held.run.carry.inventory, inventory);
        assert_eq!(
            serde_json::to_value(&held.run.summary.kills).unwrap(),
            statistics
        );
        next(&mut original);
        assert!(!original.death.waiting());
        assert_eq!(original.generation, generation + 1);
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; late enable, live-only default, option disable, Start/Enter and non-death transitions"]
fn native_death_options_handle_late_changes_and_do_not_hold_success_or_detection() {
    let mut held = start(119);
    start_death(&mut held);
    let mut settings = Settings {
        live_death_scene: true,
        ..Default::default()
    };
    held.death.configure(&settings);
    for _ in 0..400 {
        held.step(0).unwrap();
        if matches!(
            session(&held).world.mission.phase,
            Phase::Ending { remaining: 10, .. }
        ) {
            break;
        }
    }
    assert!(!held.death.waiting());
    assert!(session(&held).frame.transition_fade.is_some());
    settings.wait_after_death = true;
    held.death.configure(&settings);
    held.step(0).unwrap();
    assert!(held.death.waiting());
    assert!(session(&held).frame.transition_fade.is_none());
    assert_eq!(
        session(&held).world.actors[session(&held).world.player]
            .actor
            .health,
        0
    );
    for _ in 0..60 {
        held.step(0).unwrap();
    }
    settings.wait_after_death = false;
    held.death.configure(&settings);
    held.step(0).unwrap();
    assert!(matches!(held.content, Content::Failed));

    for (outcome, confirm) in [
        (Outcome::Dead, true),
        (Outcome::Dead, false),
        (Outcome::Detected, false),
        (Outcome::Arrived, false),
    ] {
        let mut playback = start(119);
        let Content::Gameplay(s) = &mut playback.content else {
            unreachable!()
        };
        s.world.mission.phase = Phase::Ending {
            outcome,
            remaining: 2,
            finished_at: 30,
        };
        if outcome == Outcome::Dead {
            s.world.actors[s.world.player].actor.health = 0;
            s.world.actors[s.world.player].actor.motion.loops = i16::MIN;
        }
        playback.death.configure(&Settings {
            wait_after_death: true,
            ..Default::default()
        });
        playback.step(0).unwrap();
        if outcome == Outcome::Dead {
            assert!(playback.death.waiting());
            if confirm {
                playback.confirm();
            } else {
                assert!(!playback.controller_pause(tenchu_core::input::START, true));
            }
            playback.step(0).unwrap();
            assert!(matches!(playback.content, Content::Failed));
        } else {
            assert!(!playback.death.waiting());
            next(&mut playback);
        }
    }
}
