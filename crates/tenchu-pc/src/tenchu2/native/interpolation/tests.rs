use super::*;
use crate::{tenchu2::native::Session, visuals::interpolation::Frame};
use tenchu_core::tenchu2::input::{ATTACK, CROUCH, JUMP, RIGHT, UP};

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; three heroes, body/weapon poses, camera discontinuities and identical source continuation"]
fn native_interpolation_is_read_only_and_handles_pose_discontinuities() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    for hero in 0..3 {
        let sequence = layouts
            .iter()
            .find(|l| match hero {
                0 => l.sequence < 100,
                1 => (100..185).contains(&l.sequence),
                _ => l.sequence >= 185,
            })
            .unwrap()
            .sequence;
        let mut session = Session::load(&disc, sequence, 1).unwrap();
        let mut reference = Session::load(&disc, sequence, 1).unwrap();
        session.interpolate(true).unwrap();
        let mut body_changes = 0;
        let mut weapon_changes = 0;
        let mut camera_changes = 0;
        for tick in 0..180 {
            let input = match tick {
                0..30 => UP,
                30..60 => CROUCH | UP,
                60..70 => RIGHT,
                70 => JUMP,
                130 => ATTACK,
                _ => 0,
            };
            let sounds = session.step(input).unwrap();
            let expected = reference.step(input).unwrap();
            assert_eq!(
                sounds.iter().map(|s| s.sound).collect::<Vec<_>>(),
                expected.iter().map(|s| s.sound).collect::<Vec<_>>()
            );
            let source = postcard::to_allocvec(&session.world).unwrap();
            let current = gameplay(&session.world, &session.assets).unwrap();
            let history = session.interpolation.as_ref().unwrap();
            let middle = history.sample(current.tick, 0.5).unwrap();
            let actor = &session.world.actors[session.world.player].actor;
            let instance = &session.world.instances[session.world.player];
            for (&key, &matrix) in &current.parts {
                assert_eq!(
                    Frame::transform(history.sample(current.tick, 1.0).as_ref(), key, matrix),
                    matrix
                );
                if let Part::Actor { root, model, .. } = key
                    && root == instance.rig.root
                    && Frame::transform(Some(&middle), key, matrix) != matrix
                {
                    if instance.rig.joints.contains(&model) {
                        body_changes += 1;
                    } else {
                        weapon_changes += 1;
                    }
                }
            }
            camera_changes += usize::from(middle.view != current.view);
            assert!(
                history
                    .root(current.tick, instance.rig.root, 0.5)
                    .unwrap()
                    .is_finite()
            );
            assert_eq!(
                history.root(current.tick, instance.rig.root, 1.0).unwrap(),
                glam::Vec3::from(actor.position.to_array().map(|v| v as f32))
            );
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), source);
            assert_eq!(
                source,
                postcard::to_allocvec(&reference.world).unwrap(),
                "hero {hero}, tick {tick}"
            );
        }
        assert!(
            body_changes > 0 && weapon_changes > 0 && camera_changes > 0,
            "hero {hero}: body {body_changes}, weapons {weapon_changes}, camera {camera_changes}"
        );
        session.reset_interpolation().unwrap();
        assert!(
            session
                .interpolation
                .as_ref()
                .unwrap()
                .sample(u64::from(session.world.tick), 0.0)
                .is_none()
        );
        session.interpolate(false).unwrap();
        assert!(session.interpolation.is_none());
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; authored camera cuts stay exact while story poses blend without changing playback or facial patches"]
fn native_story_interpolation_preserves_camera_cuts_and_source_playback() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for (sequence, event) in [(0, 201), (10, 4), (197, 111)] {
        let mut story = Story::load(&disc, sequence, event, 17).unwrap();
        let mut reference = Story::load(&disc, sequence, event, 17).unwrap();
        story.interpolate(true).unwrap();
        let mut changes = 0;
        for tick in 0..200 {
            let draw = tick % 7 != 3;
            story.step(draw).unwrap();
            reference.step(draw).unwrap();
            let current = super::story(&story).unwrap();
            if let Some(middle) = story
                .interpolation
                .as_ref()
                .unwrap()
                .sample(current.tick, 0.5)
            {
                assert_eq!(middle.view, story.state.view);
                changes += current
                    .parts
                    .iter()
                    .filter(|&(&key, &matrix)| {
                        Frame::transform(Some(&middle), key, matrix) != matrix
                    })
                    .count();
            }
            assert_eq!(
                postcard::to_allocvec(&story.state).unwrap(),
                postcard::to_allocvec(&reference.state).unwrap()
            );
        }
        assert!(changes > 0, "event {event}: no interpolated poses");
        for (&id, model) in &story.resources.models {
            assert_eq!(model.vram.words, reference.resources.models[&id].vram.words);
        }
    }
}
