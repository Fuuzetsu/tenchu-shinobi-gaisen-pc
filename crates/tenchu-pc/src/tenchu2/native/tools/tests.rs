use super::*;
use winit::keyboard::KeyCode;

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
