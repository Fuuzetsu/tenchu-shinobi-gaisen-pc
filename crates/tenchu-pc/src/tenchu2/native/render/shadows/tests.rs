use super::*;
use glam::Vec3;
use tenchu_core::tenchu2::world::Foot;

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SHADOW_ORACLE from JP 8003fddc; projection is an isolated boundary"]
fn ground_shadow_geometry_matches_original_instructions() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = tenchu_core::math::Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let records =
        std::fs::read_to_string(std::env::var_os("TENCHU2_SHADOW_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    let mut drawn = 0;
    for line in records.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let feet: [Foot; 2] = serde_json::from_value(row["feet"].clone()).unwrap();
        let shadow = Shadow::new(&feet, row["width"].as_i64().unwrap() as i16, &math);
        let expected = row["draws"].as_array().unwrap();
        assert_eq!(
            usize::from(shadow.is_some()),
            expected.len(),
            "case {checked}"
        );
        if let Some(shadow) = shadow {
            let actual = serde_json::json!({
                "position": shadow.position.to_array(),
                "corners": shadow.corners.map(|p| p.to_array()),
                "color": [128, 128, 128], "semi": 1,
            });
            assert_eq!(actual, expected[0], "case {checked}");
            drawn += 1;
        }
        checked += 1;
    }
    assert_eq!(checked, 4000);
    assert!(drawn > 1000 && drawn < checked);
    eprintln!("Matched {checked} shadow cases, {drawn} drawable");
}

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; source ground shadows darken native/imported floors, respect eligibility and reset without state changes"]
fn native_ground_shadows_reach_the_renderer_and_restore_exactly() {
    use crate::tenchu2::native::Session;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    for sequence in [9, 119, 191, 0] {
        let mut session = if sequence == 0 {
            let source = tenchu_assets::stage::MissionAssets::load(&gaisen, 0, 0, 1).unwrap();
            Session::gaisen(
                &disc,
                &source,
                tenchu_core::tenchu2::world::Options {
                    hero: 0,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
            )
            .unwrap()
        } else {
            Session::load(&disc, sequence, 17).unwrap()
        };
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let player = &session.world.actors[session.world.player].actor;
        let center =
            Vec3::from_array(player.position.to_array().map(|v| v as f32)) - Vec3::Y * 500.0;
        let eye = center + Vec3::new(1800.0, -3000.0, -3500.0);
        let forward = (center - eye).normalize();
        let camera = crate::debug::camera::Camera {
            position: eye,
            yaw: forward.x.atan2(forward.z),
            pitch: (-forward.y).asin(),
            fov: 45.0,
        };
        let snapshot = postcard::to_allocvec(&session.world).unwrap();
        let mut images = Vec::new();
        for (label, enabled) in [("before", false), ("after", true), ("reset", false)] {
            let mut settings = crate::settings::Settings::default();
            settings.graphics.asset_pack = false;
            settings.graphics.fog = false;
            settings.graphics.blob_shadows = enabled;
            let graphics = crate::tenchu2::native::preferences::graphics(&settings);
            let path = output.join(format!("ground-shadow-{sequence}-{label}.png"));
            capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                drawing.camera = Some(camera);
                drawing.show_hud = false;
                drawing.update(&session, renderer, None)?;
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            })
            .unwrap();
            assert_eq!(snapshot, postcard::to_allocvec(&session.world).unwrap());
            images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
        }
        assert_eq!(
            images[0].rgba, images[2].rgba,
            "reset changed pixels in sequence {sequence}"
        );
        let darker = images[0]
            .rgba
            .chunks_exact(4)
            .zip(images[1].rgba.chunks_exact(4))
            .filter(|(a, b)| {
                (0..3).any(|i| u16::from(b[i]) + 5 < u16::from(a[i]))
                    && (0..3).all(|i| b[i] <= a[i])
            })
            .count();
        assert!(
            darker > 50,
            "no visible ground shadow in sequence {sequence}: {darker}"
        );
        eprintln!("sequence {sequence}: {darker} darker pixels");
        let graphics = Graphics::default();
        let mut builder = Builder::new(camera.source_view(), &session.assets.level, &graphics);
        builder.shadows(&session.world, &session.assets, false);
        assert!(!builder.surfaces.is_empty());
        for kind in 0..3 {
            let mut world = session.world.clone();
            for agent in &mut world.actors {
                match kind {
                    0 => agent.actor.health = 0,
                    1 => agent.actor.flags |= 8,
                    _ => agent.actor.kind = 0x70,
                }
            }
            builder.surfaces.clear();
            builder.shadows(&world, &session.assets, false);
            assert!(
                builder.surfaces.is_empty(),
                "ineligible actors cast shadows"
            );
        }
    }
}
