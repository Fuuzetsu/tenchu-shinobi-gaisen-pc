use super::*;
use glam::Vec3;

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MOTION_REVIEW and GPU; verifies authored motion on all three heroes, original restoration and source-state independence"]
fn native_secondary_motion_preserves_faces_weapons_and_source_state() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MOTION_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for sequence in [9, 119, 191] {
        let mut session = super::super::super::Session::load(&disc, sequence, 17).unwrap();
        let graphics = Graphics {
            fog: false,
            asset_pack: false,
            motion: crate::settings::Motion {
                hair: true,
                cloth: true,
                equipment: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let vram = session.assets.sprites.vram.clone();
        let mut camera = None;
        capture_frame(&vram, &graphics, &out.join(format!("{sequence}-motion.png")), |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            let mut peak = 0.0_f32;
            for tick in 0..160 {
                use tenchu_core::tenchu2::input::{UP, ATTACK};
                let buttons = if tick % 120 < 30 { UP } else if tick % 120 == 45 { ATTACK } else { 0 };
                session.step(buttons)?;
                let before = postcard::to_allocvec(&session.world).unwrap();
                drawing.tick(&session, &graphics)?;
                let springs = postcard::to_allocvec(&drawing.motion).unwrap();
                for alpha in [0.0, 0.5, 1.0] {
                    drawing.alpha = alpha;
                    drawing.update(&session, renderer, None)?;
                }
                assert_eq!(postcard::to_allocvec(&drawing.motion).unwrap(), springs, "drawing advanced springs");
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before, "presentation changed source state");
                let frame = drawing.motion.sample(1.0);
                for pose in frame.values() {
                    for offset in pose.offsets {
                        assert!(offset.is_finite() && offset.length() <= 25.001);
                        peak = peak.max(offset.length());
                    }
                }
                for (root, name) in &drawing.models.weapon_names {
                    for index in 0..session.assets.weapons[name].len() {
                        assert!(!frame.keys().any(|key| matches!(key, PartId::Actor { model, .. } if *model == root + 1 + index)));
                    }
                }
            }
            assert!(peak > 2.0, "no inertia for sequence {sequence}: {peak}");
            let frame = drawing.motion.sample(1.0);
            let actor = &session.world.actors[session.world.player].actor;
            let center = Vec3::from(actor.position.to_array().map(|v| v as f32)) - Vec3::Y * 1400.0;
            let mut view = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
            view.position = center + Vec3::new(1800.0, -400.0, 2200.0);
            let forward = (center - view.position).normalize();
            view.yaw = forward.x.atan2(forward.z);
            view.pitch = (-forward.y).asin();
            camera = Some(view);
            let geometry = |models: &Models, motion| models.geometry(&session.world, &session.assets, &session.frame, &graphics, Presentation { camera: Some(view), motion, ..Default::default() });
            let original = geometry(&drawing.models, None)?;
            let moving = geometry(&drawing.models, Some(&frame))?;
            assert_eq!(bytemuck::cast_slice::<_, u8>(&original.terrain), bytemuck::cast_slice::<_, u8>(&moving.terrain));
            assert_eq!(bytemuck::cast_slice::<_, u8>(&original.props), bytemuck::cast_slice::<_, u8>(&moving.props));
            assert_eq!(original.actors.expanded().len(), moving.actors.expanded().len());
            let changes = original.actors.expanded().iter().zip(&moving.actors.expanded()).filter(|(a,b)| a.position != b.position).count();
            assert!(changes > 0, "authored regions did not move: {sequence}");
            let face_ids = ["596ae678f33b71606cffa3a01cda6fde", "78ac732e16725627b500b119e69cfe91", "ea4b3e024f12217ba863c1049556a490"];
            let protected: Vec<_> = drawing.models.bodies.values().flat_map(|m| m.materials()).filter(|m| face_ids.contains(&m.id.as_str())).map(|m| renderer.bind_material(m)).collect();
            assert!(!protected.is_empty());
            for (a,b) in original.actors.expanded().iter().zip(&moving.actors.expanded()) {
                assert_eq!((a.uv, a.material, a.page, a.palette), (b.uv, b.material, b.page, b.palette));
                if protected.contains(&a.material) { assert_eq!(a.position,b.position,"face moved"); }
            }
            drawing.tick_motion(&session, &crate::settings::Motion::default())?;
            assert!(drawing.motion.sample(1.0).is_empty(), "paused disable retained deformation");
            let restored = geometry(&drawing.models, Some(&drawing.motion.sample(1.0)))?;
            assert_eq!(bytemuck::cast_slice::<_,u8>(&original.actors.expanded()), bytemuck::cast_slice::<_,u8>(&restored.actors.expanded()));
            eprintln!("sequence {sequence}: peak {peak:.2}, {changes} deformed corners");
            Ok((SceneDrawing::new(moving, renderer, &session.assets.level), view.source_view(), Vec::new()))
        }).unwrap();
        capture_frame(
            &vram,
            &graphics,
            &out.join(format!("{sequence}-rigid.png")),
            |renderer| {
                let models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
                let geometry = models.geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation {
                        camera,
                        ..Default::default()
                    },
                )?;
                Ok((
                    SceneDrawing::new(geometry, renderer, &session.assets.level),
                    camera.unwrap().source_view(),
                    Vec::new(),
                ))
            },
        )
        .unwrap();
    }
}
