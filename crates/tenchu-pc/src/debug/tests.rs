use super::*;
use tenchu_assets::{game::GameAssets, terrain::AreaMap};

#[test]
fn focus_views_look_down_from_above_the_target() {
    let target = Vec3::new(4500.0, -1200.0, 2000.0);
    let mut camera = camera::Camera {
        position: Vec3::ZERO,
        yaw: 1.2,
        pitch: 0.0,
        fov: 44.0,
    };
    for overhead in [false, true] {
        camera.focus(target, overhead);
        assert!(camera.position.y < target.y);
        assert!(camera.forward().y > 0.0);
        let projected = camera.matrix(16.0 / 9.0).project_point3(target);
        assert!(projected.x.abs() < 0.0001 && projected.y.abs() < 0.0001);
        assert!((0.0..1.0).contains(&projected.z));
    }
}

#[test]
fn picking_respects_holes_slopes_stacked_floors_and_parallel_rays() {
    let mut bytes = vec![0u8; 64];
    bytes[2..4].copy_from_slice(&2i16.to_le_bytes());
    bytes[4..8].copy_from_slice(&32u32.to_le_bytes());
    for (at, y, division) in [(32, -100i16, 0xfffeu16), (48, 300, 0xffff)] {
        for (offset, value) in [(0, y), (2, 99), (4, -200), (6, -200), (8, 199), (10, 199)] {
            bytes[at + offset..at + offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        bytes[at + 12..at + 14].copy_from_slice(&0x4000u16.to_le_bytes());
        bytes[at + 14..at + 16].copy_from_slice(&division.to_le_bytes());
    }
    let map = AreaMap::parse(bytes).unwrap();
    let surfaces = terrain::surfaces(&map).unwrap();
    let hit = |x, z| terrain::pick(&surfaces, Vec3::new(x, -10000.0, z), Vec3::Y).unwrap();
    // The first quarter of the upper slope is a hole; select the lower floor.
    assert_eq!(hit(-1500.0, -1500.0), [-1500, 3120, -1500]);
    // Quantize with ACM's integer height calculation, not the interpolated triangle.
    assert_eq!(hit(333.0, 333.0), [333, -430, 333]);
    assert!(terrain::pick(&surfaces, Vec3::new(3000.0, -10000.0, 0.0), Vec3::Y).is_none());
    assert!(terrain::pick(&surfaces, Vec3::new(0.0, -10000.0, 0.0), Vec3::X).is_none());
    let low = Vec3::splat(-1.0);
    let high = Vec3::splat(1.0);
    assert_eq!(
        ray_box(Vec3::new(1.0, 0.0, -5.0), Vec3::Z, low, high),
        Some(4.0)
    );
    assert_eq!(ray_box(Vec3::new(2.0, 0.0, -5.0), Vec3::Z, low, high), None);
    assert_eq!(ray_box(Vec3::ZERO, Vec3::Z, low, high), Some(0.0));
}

#[test]
#[ignore = "requires TENCHU_DISC; verifies inspection geometry/camera leave the live simulation unchanged"]
fn inspecting_live_state_does_not_change_the_game() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    for character in 0..2 {
        let mut game = crate::session::Options {
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        for _ in 0..40 {
            game.step(0).unwrap();
        }
        let before = crate::session::tests::state(&game);
        let mut debug = Debug::new(&game).unwrap();
        debug.toggle_camera(&game);
        let camera = debug.camera.unwrap();
        let world = camera.position + camera.forward() * 5000.0;
        let original =
            crate::renderer::game_camera(game.rendered_view, 16.0 / 9.0).project_point3(world);
        let inspected = camera.matrix(16.0 / 9.0).project_point3(world);
        assert!(
            original.distance(inspected) < 0.002,
            "{original:?} != {inspected:?}"
        );
        debug.hitboxes = true;
        debug.sight = true;
        debug.collision = true;
        debug.patrols = true;
        debug.events = true;
        debug.labels = true;
        debug.inspect_actor = true;
        debug.show_trail = true;
        let lights = crate::lighting::sources(
            &game,
            &crate::settings::Lighting {
                enabled: true,
                count: 1,
                ..Default::default()
            },
            camera.position,
            Some(camera.position),
        );
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].position, camera.position);
        for _ in 0..3 {
            debug.focus(&game, true);
            let (origin, direction) = camera::ray(debug.matrix(&game, 16.0 / 9.0), [0.0, 0.0]);
            debug.pick_actor(&game, origin, direction);
            assert!(!debug.overlay(&game, 16.0 / 9.0).vertices.is_empty());
            assert!(!crate::render_mesh::inspection_characters(&game).is_empty());
            crate::render_effects::build_for_camera(&game, true, Some(camera.source_view()));
        }
        assert_eq!(before, crate::session::tests::state(&game));
    }
}
