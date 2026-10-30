use super::*;
use crate::render_mesh::Vertex;
use bytemuck::Zeroable;

#[test]
fn radial_input_preserves_direction_and_camera_response_is_time_based() {
    for angle in 0..360 {
        let angle = (angle as f32).to_radians();
        let v = Vec2::new(angle.cos(), angle.sin());
        assert!(stick(v * 0.15, 18) == Vec2::ZERO);
        assert!(stick(v * 0.4, 18).normalize().dot(v) > 0.99999);
        assert!((stick(v, 18).length() - 1.0).abs() < 0.00001);
    }
    assert_eq!(stick(Vec2::splat(f32::NAN), 18), Vec2::ZERO);
    let settings = Navigation::default();
    let run = |fps| {
        let mut orbit = Orbit::default();
        for _ in 0..fps {
            orbit.analog_look(Vec2::new(0.6, 0.2), 1.0 / fps as f32, &settings);
        }
        Vec2::new(orbit.yaw, orbit.pitch)
    };
    assert!((run(30) - run(144)).length() < 0.0001);
}

#[test]
fn follow_camera_stays_in_front_of_walls_and_eases_back_out() {
    let vertices: Vec<_> = [
        [-10000.0, -10000.0, -1500.0],
        [10000.0, -10000.0, -1500.0],
        [-10000.0, 10000.0, -1500.0],
        [10000.0, -10000.0, -1500.0],
        [10000.0, 10000.0, -1500.0],
        [-10000.0, 10000.0, -1500.0],
    ]
    .map(|position| Vertex {
        position,
        ..Vertex::zeroed()
    })
    .into();
    let wall = Surfaces::new(&vertices);
    let empty = Surfaces::default();
    let settings = Navigation::default();
    let mut orbit = Orbit::default();
    let blocked = orbit.camera(1.0, 1.0 / 60.0, &settings, &wall);
    assert!(blocked.position.z > -1500.0 && blocked.position.z < -1200.0);
    let first_clear = orbit.camera(1.0, 1.0 / 60.0, &settings, &empty);
    assert!(first_clear.position.z < blocked.position.z && first_clear.position.z > -3000.0);
    for _ in 0..90 {
        orbit.camera(1.0, 1.0 / 60.0, &settings, &empty);
    }
    assert!((orbit.distance - f32::from(settings.distance) * 100.0).abs() < 1.0);
    assert!(orbit.camera(1.0, 0.0, &settings, &wall).position.z > -1500.0);
}

#[test]
#[ignore = "requires TENCHU_DISC; verifies actual movement and committed actions for both heroes at non-cardinal angles"]
fn both_heroes_move_in_camera_space_without_steering_committed_actions() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let settings = Navigation::default();
    for character in 0..2 {
        let mut game = crate::session::Options {
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let start = game.player().position;
        for angle in [0.0_f32, 0.37, 0.86, 1.61, 2.83, 4.5, 5.92] {
            game.teleport_player(start, false).unwrap();
            let mut orbit = Orbit::default();
            orbit.observe(&game);
            orbit.yaw = 0.63;
            let analog = Vec2::new(angle.sin(), angle.cos());
            let expected = Vec2::new((angle + orbit.yaw).sin(), (angle + orbit.yaw).cos());
            for _ in 0..6 {
                let (buttons, heading) = orbit.movement(&game, 0, analog, &settings);
                assert!(
                    heading.is_some(),
                    "hero {character}, motion {:x}",
                    game.player().motion.action
                );
                game.navigation.heading = heading;
                game.step(buttons).unwrap();
            }
            let delta = game.player().position.wrapping_sub(start);
            let moved = Vec2::new(delta.x as f32, delta.z as f32);
            assert!(moved.length() > 50.0);
            assert!(
                moved.normalize().dot(expected) > 0.99,
                "hero {character} angle {angle}: {moved:?} vs {expected:?}"
            );
        }
        game.teleport_player(start, false).unwrap();
        let mut orbit = Orbit::default();
        orbit.observe(&game);
        assert!((orbit.pitch + 0.22).abs() < 0.001);
        let facing = game.player().rotation[1];
        for _ in 0..8 {
            let (buttons, heading) =
                orbit.movement(&game, input::R1 | input::DOWN, Vec2::ZERO, &settings);
            assert_eq!(buttons, input::DOWN);
            assert!(heading.is_none());
            game.step(buttons).unwrap();
        }
        assert_eq!(game.player().rotation[1], facing);
        assert!(matches!(
            game.player().motion.action,
            tenchu_core::ids::MOT_MOVE_BACK | tenchu_core::ids::MOT_CHASE_BACK
        ));
        game.teleport_player(start, false).unwrap();
        let mut jumped = false;
        for tick in 0..45 {
            let buttons = if tick == 5 {
                input::UP | input::CROSS
            } else {
                input::UP
            };
            let (buttons, heading) = orbit.movement(&game, buttons, Vec2::ZERO, &settings);
            if game.player().status == tenchu_core::actor::State::Jump {
                jumped = true;
                assert!(heading.is_none());
            }
            game.navigation.heading = heading;
            game.step(buttons).unwrap();
            orbit.yaw += 0.05;
        }
        assert!(jumped);
        let (buttons, heading) = orbit.movement(&game, input::L1 | input::UP, Vec2::X, &settings);
        assert!(heading.is_none());
        assert_eq!(buttons & input::L1, input::L1);
        game.camera.mode = tenchu_core::camera::Mode::Critical;
        game.camera.detail = tenchu_core::camera::Detail::Critical(0);
        assert!(!game.player_can_orbit());
        game.camera.mode = tenchu_core::camera::Mode::Normal;
        assert!(
            game.player_can_orbit(),
            "Stealth-kill camera detail persists after the kill"
        );
        game.roster.get_mut(game.player_id).status = tenchu_core::actor::State::AimShuriken;
        assert_eq!(
            orbit.movement(&game, 0, Vec2::X, &settings),
            (input::RIGHT, None),
            "Item aim must keep its own directional controls"
        );
        game.cinematic.active = true;
        assert!(!game.player_can_steer() && !game.player_can_orbit());
    }
}
