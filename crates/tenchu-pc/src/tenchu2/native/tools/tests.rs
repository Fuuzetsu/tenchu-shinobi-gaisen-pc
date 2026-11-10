use super::*;
use winit::keyboard::KeyCode;

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; force-kill tools retain source corpses and reject stale or scripted targets"]
fn kill_tools_complete_native_deaths_and_cancel_pending_clicks() {
    use tenchu_core::tenchu2::{actor::State, collision::BODY};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = crate::tenchu2::native::Session::load(&disc, 9, 1).unwrap();
    session.world.keep_all_active = true;
    session.world.actors[session.world.player]
        .actor
        .invulnerable = true;
    for _ in 0..60 {
        session.step(0).unwrap();
    }
    let victim = session
        .world
        .actors
        .iter()
        .enumerate()
        .find(|(id, a)| {
            *id != session.world.player
                && (0x20..0x60).contains(&a.actor.kind)
                && a.actor.health > 0
        })
        .unwrap()
        .0;
    session.world.actors[victim].actor.invulnerable = true;
    session.world.actors[victim].actor.inventory.resurrections = 1;
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    tools.debug.selected_actor = victim;
    tools.change(Item::ClickTeleport, 1).unwrap();
    tools.change(Item::ClickKill, 1).unwrap();
    assert!(!tools.debug.click_teleport && tools.debug.click_kill);
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    let slot = session
        .world
        .collision
        .slots()
        .iter()
        .find(|s| s.owner == Some(victim) && s.flags & BODY != 0)
        .unwrap();
    tools.debug.camera = Some(Camera {
        position: glam::Vec3::from(slot.position.to_array().map(|v| v as f32))
            - glam::Vec3::Z * 1500.0,
        yaw: 0.0,
        pitch: 0.0,
        fov: 50.0,
    });
    let before = postcard::to_allocvec(&session.world).unwrap();
    // A miss must not act on the previously selected actor.
    tools.click = Some((-10000.0, -10000.0));
    assert!(tools.apply(&mut content, [800, 600]).is_err());
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
    tools.debug.photo_mode = true;
    tools.click = Some((400.0, 300.0));
    tools.apply(&mut content, [800, 600]).unwrap();
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
    tools.debug.photo_mode = false;
    tools.click = Some((400.0, 300.0));
    tools.apply(&mut content, [800, 600]).unwrap();
    assert!(tools.debug.actors_edited);
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    let actor = &session.world.actors[victim].actor;
    assert_eq!((actor.health, actor.state), (0, State::Dead));
    assert_eq!(session.world.tick, 60);
    assert_eq!(session.world.statistics.killed, 1);
    assert_eq!(
        actor.inventory.resurrections, 1,
        "force kill bypasses resurrection"
    );
    let after = postcard::to_allocvec(&session.world).unwrap();
    assert!(session.world.kill_actor(victim, &session.assets).is_err());
    assert!(
        session
            .world
            .kill_actor(session.world.player, &session.assets)
            .is_err()
    );
    assert!(
        session
            .world
            .kill_actor(usize::MAX, &session.assets)
            .is_err()
    );
    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), after);
    for _ in 0..240 {
        session.step(0).unwrap();
    }
    let actor = &session.world.actors[victim].actor;
    assert_eq!(
        (actor.health, actor.state, actor.motion.loops),
        (0, State::Dead, i16::MIN)
    );
    assert_eq!(
        session.world.statistics.killed, 1,
        "corpse must not score twice"
    );
    tools.reset(Item::ClickKill).unwrap();
    assert!(!tools.debug.click_kill);
    tools.sync(&content, 0);
    tools.change(Item::KillActor, 1).unwrap();
    tools.change(Item::ClickKill, 1).unwrap();
    tools.click = Some((400.0, 300.0));
    tools.sync(&Content::Failed, 1);
    assert!(tools.kill.is_none() && tools.click.is_none());
    assert!(!tools.debug.click_kill && !tools.debug.actors_edited);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; forced boss deaths traverse the original objective gate for every campaign boss encounter"]
fn kill_tools_advance_native_boss_objectives() {
    use tenchu_assets::tenchu2::campaign::{Campaign, Kind, Objective};
    use tenchu_core::tenchu2::mission::{Outcome, Phase};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut count = 0;
    for (sequence, entry) in campaign.sequences.iter().enumerate() {
        let expected = match entry.kind {
            Kind::Gameplay(Objective::Defeat { .. }) => Outcome::TargetsDefeated,
            Kind::Gameplay(Objective::Weaken { .. }) => Outcome::BossWeakened,
            _ => continue,
        };
        let mut session = crate::tenchu2::native::Session::load(&disc, sequence, 17).unwrap();
        session.world.actors[session.world.player]
            .actor
            .invulnerable = true;
        for _ in 0..60 {
            session.step(0).unwrap();
        }
        let targets = session.world.mission.targets;
        let mut content = Content::Gameplay(Box::new(session));
        let mut tools = Tools::default();
        tools.sync(&content, 0);
        for index in targets.into_iter().flatten() {
            let Content::Gameplay(session) = &mut content else {
                unreachable!()
            };
            session.world.actors[index].actor.invulnerable = true;
            let before = postcard::to_allocvec(&session.world).unwrap();
            let player = session.world.player;
            let action = session.world.actors[player].actor.motion.action;
            session.world.actors[player].actor.motion.action = 0x920;
            assert!(session.world.kill_actor(index, &session.assets).is_err());
            session.world.actors[player].actor.motion.action = action;
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
            tools.debug.selected_actor = index;
            tools.change(Item::KillActor, 1).unwrap();
            tools
                .apply(&mut content, [800, 600])
                .unwrap_or_else(|e| panic!("sequence {sequence}, actor {index}: {e}"));
            count += 1;
        }
        let Content::Gameplay(session) = &mut content else {
            unreachable!()
        };
        for _ in 0..600 {
            session.step(0).unwrap();
            if !session.world.mission.input_allowed() {
                break;
            }
        }
        assert!(
            matches!(session.world.mission.phase,
            Phase::Ending { outcome, .. } | Phase::Finished { outcome, .. } if outcome == expected),
            "sequence {sequence}: {:?}",
            session.world.mission.phase
        );
    }
    assert!(count >= 20, "only {count} boss targets exercised");
    eprintln!("Force-killed {count} authored campaign boss targets through F1");
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; native camera-relative tools handle input gating, follow/aim cameras and scene boundaries"]
fn relative_controls_share_orbit_tools_and_do_not_leak_input_through_menus() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = crate::tenchu2::native::Session::load(&disc, 119, 17).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    let mut controls = Controls::default();
    controls.key(KeyCode::KeyD, true);
    let settings = crate::settings::Navigation {
        camera_relative: true,
        ..Default::default()
    };
    let buttons = tools.input(&mut content, &mut controls, &settings, true);
    assert_eq!(buttons, tenchu_core::tenchu2::input::RIGHT);
    assert_eq!(tools.orbit_actor().unwrap().id, tools.scene.player);
    tools.change(Item::RecenterOrbit, 1).unwrap();
    tools.orbit.mouse(200.0, 60.0, &settings);
    let yaw = tools.orbit.yaw;
    tools.input(&mut content, &mut controls, &settings, true);
    if let Content::Gameplay(session) = &mut content {
        assert_eq!(session.navigation.unwrap().yaw, yaw);
        session.step(buttons).unwrap();
    }
    tools.sync(&content, 0);
    assert_eq!(
        tools.orbit.yaw, yaw,
        "steering must not spin the orbit camera"
    );
    for active in [false, true] {
        if active {
            tools.change(Item::FreeCamera, 1).unwrap();
        }
        assert_eq!(
            tools.input(&mut content, &mut controls, &settings, active),
            0
        );
        if let Content::Gameplay(session) = &content {
            assert!(session.navigation.is_none());
        }
    }
    tools.debug.return_camera();
    tools.input(&mut content, &mut controls, &Default::default(), true);
    assert!(tools.orbit_actor().is_none());
    if let Content::Gameplay(session) = &content {
        assert!(session.navigation.is_none());
    }
    tools.sync(&Content::Failed, 1);
    assert!(tools.orbit_actor().is_none());
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; shared Experiment commands edit the native inventory without interrupting items"]
fn experiment_edits_native_quantities_and_protection_without_refilling_health() {
    use tenchu_core::tenchu2::inventory::Item as NativeItem;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = crate::tenchu2::native::Session::load(&disc, 9, 1).unwrap();
    session.world.actors[0].actor.health = 21;
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    let rows = crate::settings::menu::rows(17, &Default::default(), &tools.debug, false);
    assert!(
        rows.iter()
            .any(|r| r.item == Item::Inventory(4) && r.label == "Shuriken")
    );
    assert!(!tools.debug.inventory_unlimited);
    tools.change(Item::GodMode, 1).unwrap();
    tools.change(Item::Armor, 1).unwrap();
    tools.change(Item::GiveItems, 1).unwrap();
    tools.apply(&mut content, [1280, 720]).unwrap();
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    let actor = &mut session.world.actors[0].actor;
    assert_eq!(actor.health, 21);
    assert!(actor.invulnerable && session.world.controls.armor);
    assert_eq!(actor.inventory.counts[4], 10);
    assert_eq!(actor.inventory.counts[3], 0);
    assert_eq!(actor.inventory.counts[19], 0);
    assert_eq!(actor.inventory.resurrections, 10);
    let shuriken = NativeItem::try_from(4).unwrap();
    actor.inventory.selected = Some(shuriken);
    assert_eq!(actor.inventory.take().unwrap(), shuriken);
    tools.sync(&content, 0);
    tools.change(Item::ClearItems, 1).unwrap();
    tools.apply(&mut content, [1280, 720]).unwrap();
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    let inventory = &session.world.actors[0].actor.inventory;
    assert_eq!(inventory.amounts(), [0; 21]);
    assert_eq!(inventory.active, shuriken);
    assert_eq!(inventory.selected, None);
    tools.sync(&content, 0);
    tools.change(Item::Inventory(4), 255).unwrap();
    tools.reset(Item::GodMode).unwrap();
    tools.reset(Item::Armor).unwrap();
    tools.apply(&mut content, [1280, 720]).unwrap();
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    let actor = &mut session.world.actors[0].actor;
    assert_eq!(actor.health, 21);
    assert!(!actor.invulnerable && !session.world.controls.armor);
    assert_eq!(actor.inventory.selected, Some(shuriken));
    assert_eq!(actor.inventory.counts[4], 255);
    assert_eq!(actor.inventory.take().unwrap(), shuriken);
    assert_eq!(actor.inventory.counts[4], 254);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; live sight edits preserve source thresholds and reset without cumulative scaling"]
fn sight_controls_preserve_native_ranges_and_default_continuation() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut reference = crate::tenchu2::native::Session::load(&disc, 119, 17).unwrap();
    let session = crate::tenchu2::native::Session::load(&disc, 119, 17).unwrap();
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    for (delta, expected) in [(4, 200), (1000, 400), (-1000, 0)] {
        tools.change(Item::SightDistance, delta).unwrap();
        tools.apply(&mut content, [1280, 720]).unwrap();
        tools.sync(&content, 0);
        assert_eq!(tools.debug.sight_percent, expected);
    }
    tools.reset(Item::SightDistance).unwrap();
    tools.change(Item::Stealth, 1).unwrap();
    tools.apply(&mut content, [1280, 720]).unwrap();
    tools.sync(&content, 0);
    assert!(tools.debug.stealth);
    tools.reset(Item::Stealth).unwrap();
    tools.apply(&mut content, [1280, 720]).unwrap();
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    for _ in 0..120 {
        assert_eq!(
            postcard::to_allocvec(&session.world).unwrap(),
            postcard::to_allocvec(&reference.world).unwrap()
        );
        session.step(0).unwrap();
        reference.step(0).unwrap();
    }
    tools.sync(&Content::Failed, 1);
    assert_eq!(tools.debug.sight_percent, 100);
    assert!(!tools.debug.stealth);
    assert!(tools.change(Item::SightDistance, -1).is_err());
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; debug teleport must repair the live native world"]
fn teleport_repairs_pose_contacts_and_camera_without_advancing_gameplay() {
    use tenchu_core::{math::Position, tenchu2::actor::State};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let session = crate::tenchu2::native::Session::load(&disc, 9, 1).unwrap();
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    tools.change(Item::ClickTeleport, 1).unwrap();
    tools.reset(Item::ClickTeleport).unwrap();
    assert!(!tools.debug.click_teleport);
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    let player = session.world.player;
    let start = session.world.actors[player].actor.position;
    let before = serde_json::to_vec(&session.world).unwrap();
    assert!(
        session
            .world
            .teleport_player(Position::new(i32::MAX, 0, i32::MIN), true, &session.assets)
            .is_err()
    );
    assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
    // Find a separate usable floor in the actual mission, through the click picker.
    let mut destination = None;
    for dx in [-3000, 3000, -1500, 1500] {
        let from = glam::Vec3::new(
            (start.x + dx) as f32,
            (start.y - 5000) as f32,
            start.z as f32,
        );
        if let Some(point) =
            super::super::inspection::pick(&session.world.terrain, from, glam::Vec3::Y)
            && session
                .world
                .teleport_player(point, false, &session.assets)
                .is_ok()
        {
            destination = Some(point);
            break;
        }
    }
    let destination = destination.expect("No nearby usable floor");
    let actor = &session.world.actors[player].actor;
    assert_ne!(actor.position, start);
    assert_eq!(actor.position, destination);
    assert_eq!(actor.previous, destination);
    assert_eq!(actor.velocity.to_array(), [0; 3]);
    assert_eq!(session.world.tick, 0);
    let saved: serde_json::Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(
        serde_json::to_value(session.world.random).unwrap(),
        saved["random"]
    );
    let root = session.world.instances[player].rig.root;
    assert_eq!(
        session.world.scene.world(root).unwrap().translation,
        destination
    );
    for _ in 0..4 {
        session.step(0).unwrap();
    }
    assert_eq!(session.world.actors[player].actor.position.x, destination.x);
    assert_eq!(session.world.actors[player].actor.position.z, destination.z);
    session.world.actors[player].actor.state = State::Hurt;
    let before = serde_json::to_vec(&session.world).unwrap();
    assert!(
        session
            .world
            .teleport_player(start, false, &session.assets)
            .is_err()
    );
    assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
    session.world.actors[player].actor.state = State::Normal;
    tools.debug.target = start;
    tools.change(Item::Teleport, 1).unwrap();
    tools.apply(&mut content, [1280, 720]).unwrap();
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    let returned = session.world.actors[player].actor.position;
    assert_eq!([returned.x, returned.z], [start.x, start.z]);
    assert_eq!(
        Some(returned.y),
        session.world.actors[player].actor.terrain.floor
    );
    for _ in 0..12 {
        session.step(tenchu_core::tenchu2::input::UP).unwrap();
    }
    assert_ne!(session.world.actors[player].actor.position, start);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; camera input must not change native simulation"]
fn inspection_is_independent_of_native_gameplay_and_scene_transitions() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let session = crate::tenchu2::native::Session::load(&disc, 9, 1).unwrap();
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    let before = serde_json::to_vec(&session.world).unwrap();
    tools.change(Item::FreeCamera, 1).unwrap();
    let start = tools.debug.camera.unwrap().position;
    let mut controls = Controls::default();
    controls.key(KeyCode::KeyW, true);
    for _ in 0..60 {
        tools.advance(&mut controls, 1.0 / 60.0, true, &Default::default());
    }
    assert!(tools.debug.camera.unwrap().position.distance(start) > 11000.0);
    tools.advance(&mut controls, 1.0, false, &Default::default());
    tools.change(Item::Actor, 1).unwrap();
    tools.change(Item::FocusActor, 1).unwrap();
    tools.change(Item::SaveCamera, 1).unwrap();
    tools.change(Item::Fov, 1).unwrap();
    tools.change(Item::RestoreCamera, 1).unwrap();
    assert_eq!(
        tools.debug.camera.unwrap().fov,
        tools.debug.bookmark.unwrap().fov
    );
    for item in [
        Item::Hitboxes,
        Item::Labels,
        Item::InspectActor,
        Item::Sight,
        Item::Patrols,
        Item::Events,
        Item::Collision,
    ] {
        tools.change(item, 1).unwrap();
    }
    let matrix = tools.debug.camera.unwrap().matrix(16.0 / 9.0);
    let overlay = super::super::inspection::overlay(session, &tools.debug, matrix, 16.0 / 9.0);
    assert!(!overlay.geometry.vertices.is_empty());
    assert!(overlay.labels.iter().any(|label| label.1.contains("HP")));
    assert!(
        overlay
            .geometry
            .vertices
            .iter()
            .all(|vertex| vertex.position.iter().all(|v| v.is_finite()))
    );
    tools.change(Item::PhotoMode, 1).unwrap();
    let hidden = super::super::inspection::overlay(session, &tools.debug, matrix, 16.0 / 9.0);
    assert!(hidden.geometry.vertices.is_empty() && hidden.labels.is_empty());
    tools.reset(Item::PhotoMode).unwrap();
    assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
    tools.change(Item::FreeCamera, 1).unwrap();
    assert!(tools.debug.camera.is_none());
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    session.step(0).unwrap();
    tools.sync(&content, 0);
    tools.change(Item::FreeCamera, 1).unwrap();
    tools.sync(&Content::Failed, 1);
    assert!(tools.debug.camera.is_none() && tools.debug.bookmark.is_none());
    assert!(tools.change(Item::FreeCamera, 1).is_err());
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; follows live guards without changing simulation and returns/reset across scenes"]
fn shared_follow_camera_tracks_native_guards_and_survives_player_death() {
    use crate::visuals::surfaces::Surfaces;
    use glam::Vec3;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let session = crate::tenchu2::native::Session::load(&disc, 9, 1).unwrap();
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    let mut controls = Controls::default();
    let navigation = crate::settings::Navigation::default();
    tools.sync(&content, 0);
    tools.debug.selected_actor = 1;
    tools.change(Item::FollowActor, 1).unwrap();
    let surfaces = Surfaces::default();
    for tick in 0..180 {
        let Content::Gameplay(session) = &mut content else {
            unreachable!()
        };
        session.step(0).unwrap();
        if tick == 90 {
            session.world.actors[session.world.player].actor.health = 0;
        }
        let before = postcard::to_allocvec(&session.world).unwrap();
        tools.sync(&content, 0);
        tools.advance(&mut controls, 1.0 / 60.0, true, &navigation);
        let actor = tools.debug.followed_inspection(&tools.scene).unwrap();
        assert_eq!(actor.id, 1);
        let camera = tools.orbit.camera(1.0, 1.0 / 60.0, &navigation, &surfaces);
        let target = camera
            .matrix(16.0 / 9.0)
            .project_point3(actor.position - Vec3::Y * 1100.0);
        assert!(target.x.abs() < 0.0001 && target.y.abs() < 0.0001);
        assert!(camera.position.is_finite());
        let Content::Gameplay(session) = &content else {
            unreachable!()
        };
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
    }
    tools.change(Item::Actor, 1).unwrap();
    assert_eq!(tools.debug.camera_actor.unwrap().id, 2);
    tools.reset(Item::Actor).unwrap();
    assert_eq!(tools.debug.camera_actor.unwrap().id, tools.scene.player);
    tools.reset(Item::FollowActor).unwrap();
    assert!(tools.debug.camera_actor.is_none());
    tools.change(Item::FollowActor, 1).unwrap();
    tools.change(Item::FreeCamera, 1).unwrap();
    assert!(tools.debug.camera_actor.is_none() && tools.debug.camera.is_none());
    tools.change(Item::FollowActor, 1).unwrap();
    tools.sync(&Content::Failed, 1);
    assert!(tools.debug.camera_actor.is_none());
    assert!(tools.change(Item::FollowActor, 1).is_err());
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; shared trail records native walking without redraw duplication, teleport bridges or scene leaks"]
fn shared_trail_tracks_native_movement_and_resets_between_scenes() {
    use crate::tenchu2::native::Session;
    use tenchu_core::tenchu2::input;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut content = Content::Gameplay(Box::new(Session::load(&disc, 119, 17).unwrap()));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    tools.change(Item::Trail, 1).unwrap();
    for _ in 0..60 {
        let Content::Gameplay(session) = &mut content else {
            unreachable!()
        };
        session.step_world(input::UP, true).unwrap();
        tools.sync(&content, 0);
    }
    tools.change(Item::Overhead, 1).unwrap();
    let matrix = tools.debug.camera.unwrap().matrix(16.0 / 9.0);
    let trail = |content: &Content, tools: &Tools| {
        let Content::Gameplay(session) = content else {
            unreachable!()
        };
        super::super::inspection::overlay(session, &tools.debug, matrix, 16.0 / 9.0)
            .geometry
            .vertices
    };
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    let state = postcard::to_allocvec(&session.world).unwrap();
    let path = trail(&content, &tools);
    assert!(!path.is_empty());
    for _ in 0..10 {
        tools.sync(&content, 0);
    }
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&path),
        bytemuck::cast_slice::<_, u8>(&trail(&content, &tools))
    );
    tools.change(Item::PhotoMode, 1).unwrap();
    assert!(trail(&content, &tools).is_empty());
    tools.reset(Item::PhotoMode).unwrap();
    tools.reset(Item::Trail).unwrap();
    assert!(trail(&content, &tools).is_empty());
    tools.change(Item::Trail, 1).unwrap();
    assert_eq!(path.len(), trail(&content, &tools).len());
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    assert_eq!(state, postcard::to_allocvec(&session.world).unwrap());
    tools.sync(&content, 1);
    assert!(
        trail(&content, &tools).is_empty(),
        "new scene retained the previous path"
    );
    let Content::Gameplay(session) = &mut content else {
        unreachable!()
    };
    session.world.tick += 1;
    session.world.actors[session.world.player].actor.position.x += 5000;
    tools.sync(&content, 1);
    assert!(
        trail(&content, &tools).is_empty(),
        "teleport drew a line across the map"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; movable lights, follow/freeze, focus and scene transitions are presentation-only"]
fn lighting_tools_preserve_native_world_and_clear_between_scenes() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let session = crate::tenchu2::native::Session::load(&disc, 119, 1).unwrap();
    let before = postcard::to_allocvec(&session.world).unwrap();
    let mut content = Content::Gameplay(Box::new(session));
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    tools.change(Item::PlayerLight, 1).unwrap();
    let fixed = tools.debug.light.unwrap();
    tools.change(Item::FreeCamera, 1).unwrap();
    tools.change(Item::FollowLight, 1).unwrap();
    tools.debug.camera.as_mut().unwrap().position.x += 3000.0;
    let eye = tools.debug.camera.unwrap().position;
    assert_eq!(tools.debug.light_position(&tools.scene), Some(eye));
    assert_ne!(fixed, eye);
    tools.reset(Item::FollowLight).unwrap();
    tools.debug.camera.as_mut().unwrap().position.z += 3000.0;
    assert_eq!(tools.debug.light_position(&tools.scene), Some(eye));
    tools.change(Item::LightAxis(0), 1).unwrap();
    assert_eq!(tools.debug.light.unwrap().x, eye.x + 500.0);
    tools.reset(Item::LightAxis(0)).unwrap();
    assert_eq!(tools.debug.light.unwrap().x, 0.0);
    tools.change(Item::FocusLight, 1).unwrap();
    tools.apply(&mut content, [1280, 720]).unwrap();
    let camera = tools.debug.camera.unwrap();
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    let lamp = super::super::render::lighting::nearest(
        &session.assets.level,
        &session.assets.resources.math,
        camera.position,
    )
    .unwrap();
    let ndc = camera
        .matrix(16.0 / 9.0)
        .project_point3(lamp.light.position);
    assert!(ndc.x.abs() < 0.001 && ndc.y.abs() < 0.001);
    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
    tools.reset(Item::DebugLight).unwrap();
    assert!(tools.debug.light.is_none());
    tools.change(Item::DebugLight, 1).unwrap();
    tools.sync(&content, 1);
    assert!(tools.debug.light.is_none() && !tools.debug.light_follow);
    let mut menu = Content::Failed;
    tools.sync(&menu, 2);
    assert!(tools.change(Item::PlayerLight, 1).is_err());
    assert!(tools.change(Item::FocusLight, 1).is_err());
    tools.apply(&mut menu, [1280, 720]).unwrap();
}
