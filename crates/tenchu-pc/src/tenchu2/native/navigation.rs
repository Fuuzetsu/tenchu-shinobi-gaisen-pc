use super::Session;
use crate::navigation::{directions, heading, movement_vector, relative_movement};
use glam::Vec2;
use tenchu_core::input;

#[derive(Clone, Copy, serde::Serialize)]
pub struct Input {
    pub yaw: f32,
    pub pitch: i16,
    pub sticks: [Vec2; 2],
    pub deadzone: u16,
    pub align_aim: bool,
}

pub fn apply(session: &mut Session, buttons: u16) -> tenchu_assets::Result<u16> {
    let world = &mut session.world;
    let Some(navigation) = session.navigation else {
        world.camera.initial_pitch = None;
        return Ok(buttons);
    };
    let buttons = buttons.swap_bytes();
    let actor = &world.actors[world.player].actor;
    let pressed = buttons & !actor.pad.held.swap_bytes();
    let aim_item = actor
        .inventory
        .selected
        .is_some_and(|item| matches!(u8::from(item), 0 | 4 | 17));
    let analog = navigation.sticks[0]
        + if buttons & input::L1 != 0 || world.camera.mode == 20 {
            navigation.sticks[1]
        } else {
            Vec2::ZERO
        };
    let v = movement_vector(buttons, analog, navigation.deadzone);
    if !navigation.align_aim
        || (buttons & input::L1 == 0
            && actor.state != tenchu_core::tenchu2::actor::State::Item
            && !matches!(actor.motion.action, 0x514 | 0x515))
    {
        world.camera.initial_pitch = None;
    }
    let (buttons, heading) = if navigation.align_aim
        && world.player_can_steer()
        && (pressed & input::L1 != 0 || pressed & input::TRIANGLE != 0 && aim_item)
    {
        world.camera.initial_pitch = Some(navigation.pitch);
        (buttons, Some(heading(navigation.yaw)))
    } else if buttons & (input::L1 | input::CIRCLE) != 0 || !world.player_can_orbit() {
        (buttons | directions(v), None)
    } else {
        relative_movement(
            navigation.yaw,
            buttons,
            v,
            actor.rotation.y,
            world.player_can_steer(),
        )
    };
    if let Some(heading) = heading {
        world.steer_player(heading, &session.assets)?;
    }
    Ok(buttons.swap_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires Japanese TENCHU2_DISC; all heroes retain source motion/collision with camera-relative input and restore classic controls"]
    fn native_relative_controls_preserve_committed_actions_and_aim_from_camera() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        for sequence in [9, 119, 191] {
            let mut session = Session::load(&disc, sequence, 17).unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let original = session.world.clone();
            assert!(
                session.world.player_can_steer(),
                "sequence {sequence}: {:?}",
                session.world.actors[session.world.player].actor.motion
            );
            let mut input = Input {
                yaw: 0.63,
                pitch: 340,
                sticks: [Vec2::ZERO; 2],
                deadzone: 18,
                align_aim: true,
            };
            for angle in [0.1_f32, 0.6, 1.8, 3.2, 4.7, 5.9] {
                session.world = original.clone();
                input.sticks[0] = Vec2::new(angle.sin(), angle.cos());
                session.navigation = Some(input);
                let start = session.world.actors[session.world.player].actor.position;
                for _ in 0..8 {
                    session.step(0).unwrap();
                }
                let actor = &session.world.actors[session.world.player].actor;
                let delta = actor.position.wrapping_sub(start);
                let delta = Vec2::new(delta.x as f32, delta.z as f32).normalize_or_zero();
                let wanted = Vec2::new((input.yaw + angle).sin(), (input.yaw + angle).cos());
                assert!(
                    delta.dot(wanted) > 0.95,
                    "sequence {sequence}, angle {angle}: {delta:?} vs {wanted:?}, action {:x}",
                    actor.motion.action
                );
            }
            session.world = original.clone();
            input.sticks = [Vec2::ZERO; 2];
            session.navigation = Some(input);
            let yaw = session.world.actors[session.world.player].actor.rotation.y;
            let guarded = apply(&mut session, (input::CIRCLE | input::LEFT).swap_bytes()).unwrap();
            assert_eq!(guarded, (input::CIRCLE | input::LEFT).swap_bytes());
            assert_eq!(
                session.world.actors[session.world.player].actor.rotation.y,
                yaw
            );
            session.world.actors[session.world.player]
                .actor
                .motion
                .action = 0x300;
            session.world.actors[session.world.player].actor.state =
                tenchu_core::tenchu2::actor::State::Wall;
            apply(&mut session, input::L1.swap_bytes()).unwrap();
            assert_eq!(
                session.world.actors[session.world.player].actor.rotation.y,
                yaw
            );
            assert_eq!(session.world.camera.initial_pitch, None);
            session.world = original.clone();
            session.step(input::SQUARE.swap_bytes()).unwrap();
            for _ in 0..4 {
                let actor = &session.world.actors[session.world.player].actor;
                assert!(!session.world.player_can_steer());
                let yaw = actor.rotation.y;
                apply(&mut session, input::LEFT.swap_bytes()).unwrap();
                assert_eq!(
                    session.world.actors[session.world.player].actor.rotation.y,
                    yaw
                );
                session.step(0).unwrap();
            }
            for align in [false, true] {
                for held in [input::L1, input::TRIANGLE] {
                    session.world = original.clone();
                    input.align_aim = align;
                    session.navigation = Some(input);
                    session.world.actors[session.world.player]
                        .actor
                        .inventory
                        .selected = Some(tenchu_core::tenchu2::inventory::Item::GRAPPLE);
                    let before_yaw = session.world.actors[session.world.player].actor.rotation.y;
                    for _ in 0..60 {
                        session.step(held.swap_bytes()).unwrap();
                        if matches!(session.world.camera.mode, 6 | 20)
                            && session.world.camera.previous == session.world.camera.mode
                        {
                            break;
                        }
                    }
                    assert!(
                        matches!(session.world.camera.mode, 6 | 20),
                        "sequence {sequence}: aim failed"
                    );
                    let actor = &session.world.actors[session.world.player].actor;
                    assert_eq!(
                        actor.rotation.y & 4095,
                        if align {
                            heading(input.yaw)
                        } else {
                            before_yaw
                        } & 4095
                    );
                    assert_eq!(
                        session.world.camera.direction[0],
                        if align { input.pitch } else { 0 }
                    );
                    let yaw = actor.rotation.y;
                    session.step((held | input::RIGHT).swap_bytes()).unwrap();
                    session.step((held | input::RIGHT).swap_bytes()).unwrap();
                    assert_eq!(
                        session.world.actors[session.world.player].actor.rotation.y,
                        yaw
                    );
                    assert!(session.world.camera.direction[1] > 0);
                }
            }
            session.world = original;
            session.navigation = None;
            let before = postcard::to_allocvec(&session.world).unwrap();
            assert_eq!(apply(&mut session, 0x1234).unwrap(), 0x1234);
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
        }
    }
}
