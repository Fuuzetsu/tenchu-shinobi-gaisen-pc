use super::*;
use glam::Vec3;

#[test]
#[ignore = "requires TENCHU_DISC; flying F2 away and returning must preserve AI and the Ki display"]
fn free_camera_returns_without_changing_radar_or_gameplay() {
    for mission in [1, 3, 10] {
        for (relative, all_active) in [(false, false), (false, true), (true, true)] {
            let options = Options {
                mission,
                ..Default::default()
            };
            let mut app = application(options);
            let mut reference = application(options);
            for host in [&mut app, &mut reference] {
                host.settings.frontend.auto_save = false;
                host.settings.navigation.camera_relative = relative;
                host.campaign.game.events = Default::default();
                host.campaign.game.roster.keep_all_active = all_active;
                for _ in 0..40 {
                    assert!(host.game_tick(0));
                }
            }
            app.apply_item(Item::FreeCamera, 1).unwrap();
            app.controls.key(KeyCode::KeyW, true);
            app.controls.key(KeyCode::KeyE, true);
            app.controls.key(KeyCode::ShiftLeft, true);
            for frame in 0..180 {
                if frame == 90 {
                    app.controls.clear();
                    app.apply_item(Item::FreeCamera, 1).unwrap();
                    assert!(app.debug.camera.is_none());
                }
                if let Some(camera) = &mut app.debug.camera {
                    camera.advance(&app.controls, 1.0 / 30.0, 120_000.0);
                    camera.look(13.0, -0.2);
                }
                for host in [&mut app, &mut reference] {
                    host.advance_simulation(Duration::from_nanos(33_333_334));
                }
                let game = &app.campaign.game;
                let expected = &reference.campaign.game;
                assert_eq!(game.tick, expected.tick);
                assert_eq!(
                    format!("{:?}", (&game.intelligence, &game.hud)),
                    format!("{:?}", (&expected.intelligence, &expected.hud)),
                    "M{mission}, relative {relative}, all-active {all_active}, frame {frame}"
                );
            }
            assert_eq!(
                crate::session::tests::state(&app.campaign.game),
                crate::session::tests::state(&reference.campaign.game)
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; follow survives death and can be selected in frozen/live death holds without altering gameplay"]
fn actor_follow_survives_player_death() {
    for character in 0..2 {
        for live in [false, true] {
            let options = Options {
                character,
                ..Default::default()
            };
            let mut app = application(options);
            let mut reference = application(options);
            for host in [&mut app, &mut reference] {
                host.settings.wait_after_death = true;
                host.settings.live_death_scene = live;
                host.settings.frontend.auto_save = false;
                host.campaign.game.roster.keep_all_active = true;
                let events = std::mem::take(&mut host.campaign.game.events);
                for _ in 0..30 {
                    assert!(host.game_tick(0));
                }
                host.campaign.game.events = events;
            }
            let game = &app.campaign.game;
            let subject = game
                .roster
                .order
                .iter()
                .copied()
                .find(|&id| id != game.player_id && game.roster.get(id).patrol.is_some())
                .unwrap();
            app.debug.selected_actor = subject;
            app.apply_item(Item::FollowActor, 1).unwrap();
            for host in [&mut app, &mut reference] {
                super::death::kill(&mut host.campaign.game);
            }
            for _ in 0..3000 {
                reference.game_tick(0);
                let running = app.game_tick(0);
                assert!(app.orbit_available(), "Follow disabled during death");
                if !running {
                    break;
                }
            }
            assert!(matches!(app.campaign.screen, Screen::DeathHold { .. }));
            assert!(!app.campaign.game.player_can_orbit());
            assert_eq!(app.simulating(), live);
            app.apply_item(Item::FollowActor, 1).unwrap();
            assert!(!app.orbit_available());
            app.apply_item(Item::FollowActor, 1).unwrap();
            assert!(app.orbit_available(), "Cannot enable follow after dying");
            let surfaces = crate::visuals::surfaces::Surfaces::default();
            for _ in 0..90 {
                assert_eq!(app.game_tick(0), live);
                assert_eq!(reference.game_tick(0), live);
                app.advance_navigation(1.0 / 60.0);
                assert_eq!(
                    crate::session::tests::state(&app.campaign.game),
                    crate::session::tests::state(&reference.campaign.game)
                );
                assert!(app.orbit_available());
                let actor = app.debug.followed_actor(&app.campaign.game).unwrap();
                assert_eq!(actor.id, subject);
                let target =
                    Vec3::from(actor.position.to_array().map(|v| v as f32)) - Vec3::Y * 1100.0;
                let view = app
                    .orbit
                    .camera(1.0, 1.0 / 60.0, &app.settings.navigation, &surfaces);
                let projected = view.matrix(16.0 / 9.0).project_point3(target);
                assert!(projected.x.abs() < 0.0001 && projected.y.abs() < 0.0001);
            }
            app.action(Action::Start);
            assert!(matches!(app.campaign.screen, Screen::GameOver));
            assert!(!app.orbit_available());
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; live camera ownership must not change controls, RNG, AI or the retail camera"]
fn actor_follow_preserves_play_and_returns_safely() {
    for (character, keep_all_active) in [(0, false), (1, false), (0, true), (1, true)] {
        let options = Options {
            character,
            ..Default::default()
        };
        let mut app = application(options);
        let mut reference = application(options);
        for host in [&mut app, &mut reference] {
            host.campaign.game.events = Default::default();
            host.campaign.game.roster.keep_all_active = keep_all_active;
            host.settings.frontend.auto_save = false;
            for _ in 0..40 {
                assert!(host.game_tick(0));
            }
        }
        let game = &app.campaign.game;
        let subject = game
            .roster
            .order
            .iter()
            .copied()
            .filter(|&id| {
                id != game.player_id
                    && game.roster.get(id).health > 0
                    && game.roster.get(id).patrol.is_some()
            })
            .min_by_key(|&id| {
                let p = game
                    .roster
                    .get(id)
                    .position
                    .wrapping_sub(game.player().position);
                i64::from(p.x).pow(2) + i64::from(p.z).pow(2)
            })
            .unwrap();
        app.debug.selected_actor = subject;
        app.apply_item(Item::FollowActor, 1).unwrap();
        assert!(app.debug.camera.is_none());
        assert!(app.orbit_enabled());
        assert!(!app.navigation_enabled());
        let surfaces = crate::visuals::surfaces::Surfaces::default();
        let settings = app.settings.navigation.clone();
        let mut previous = Vec3::ZERO;
        let mut traveled = 0.0;
        for tick in 0..90 {
            for host in [&mut app, &mut reference] {
                host.controls.key(KeyCode::KeyW, tick < 45);
                host.advance_simulation(Duration::from_nanos(33_333_334));
            }
            assert_eq!(
                crate::session::tests::state(&app.campaign.game),
                crate::session::tests::state(&reference.campaign.game)
            );
            let actor = app.debug.followed_actor(&app.campaign.game).unwrap();
            let target = Vec3::from(actor.position.to_array().map(|v| v as f32)) - Vec3::Y * 1100.0;
            let view = app.orbit.camera(1.0, 1.0 / 60.0, &settings, &surfaces);
            let projected = view.matrix(16.0 / 9.0).project_point3(target);
            assert!(projected.x.abs() < 0.0001 && projected.y.abs() < 0.0001);
            if tick > 0 {
                traveled += view.position.distance(previous);
            }
            previous = view.position;
        }
        if keep_all_active {
            assert!(traveled > 100.0, "Selected patrol must actually move");
        }
        app.settings.navigation.camera_relative = true;
        app.orbit.yaw = 0.7;
        let buttons = app.navigation_input(tenchu_core::input::UP);
        assert_ne!(buttons, 0);
        assert!(app.campaign.game.navigation.heading.is_some());
        assert!(
            app.campaign.game.navigation.camera.is_none(),
            "The core camera must never target the NPC"
        );
        app.apply_item(Item::Actor, 1).unwrap();
        assert_eq!(app.debug.camera_actor.unwrap().id, app.debug.selected_actor);
        app.apply_item(Item::FreeCamera, 1).unwrap();
        assert!(app.debug.camera_actor.is_none() && app.debug.camera.is_none());
        app.apply_item(Item::FollowActor, 1).unwrap();
        app.reset_item(Item::FollowActor).unwrap();
        assert!(app.debug.camera_actor.is_none());
        app.debug.selected_actor = subject;
        app.apply_item(Item::FollowActor, 1).unwrap();
        let removed = app.campaign.game.roster.actors[subject].take().unwrap();
        app.debug.update(&app.campaign.game);
        assert!(app.debug.camera_actor.is_none() && app.debug.follow_camera.is_none());
        app.campaign.game.roster.actors[subject] = Some(removed);
        app.apply_item(Item::FollowActor, 1).unwrap();
        app.campaign.game.roster.get_mut(subject).root_model += 1;
        app.debug.update(&app.campaign.game);
        assert!(
            app.debug.camera_actor.is_none(),
            "Do not follow a reused actor slot"
        );
        app.apply_item(Item::FollowActor, 1).unwrap();
        app.debug.reload(&app.campaign.game).unwrap();
        assert!(app.debug.camera_actor.is_none() && app.debug.follow_camera.is_none());
    }
}
