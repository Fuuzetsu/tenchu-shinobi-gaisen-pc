use super::*;
use crate::{
    settings::{BloodStyle, Settings},
    tenchu2::native::{dismemberment_tests as fixture, models},
};
use glam::Vec3;

#[test]
#[ignore = "requires TENCHU_DISC, Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; source lethal hits, native/imported/custom detached rendering, finite blood and presentation restore"]
fn native_dismemberment_rendering_blood_and_restore() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let mut donor = models::donor(&gaisen).unwrap();
    let mut library = crate::visuals::appearance::library::Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let folder = out.join(format!("custom-body-{}", std::process::id()));
    crate::visuals::export(&donor.mission, &folder).unwrap();
    let mut manifest: crate::visuals::pack::Manifest =
        serde_json::from_slice(&std::fs::read(folder.join("pack.json")).unwrap()).unwrap();
    manifest.player_models.get_mut("0000").unwrap().parts[2].position[1] -= 250;
    std::fs::write(
        folder.join("pack.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let custom = Pack::bodies(folder.to_str().unwrap()).unwrap();
    for (hero, model, head, side) in [
        (0, "", 100, 100),
        (1, "", 0, 100),
        (2, "", 0, 0),
        (0, "disc:1", 100, 100),
        (1, "pack:0000", 100, 100),
    ] {
        let (mut session, guard) = fixture::fixture(hero);
        let player = session.world.player;
        let mut settings = Settings {
            player_model: model.into(),
            ..Default::default()
        };
        settings.graphics.fog = false;
        settings.graphics.asset_pack = false;
        settings.graphics.smooth_models = true;
        settings.graphics.blood = crate::settings::Blood {
            hit: BloodStyle::Off,
            ground: BloodStyle::Modern,
            death: BloodStyle::Off,
            drips: BloodStyle::Off,
            severed: 200,
            severed_style: BloodStyle::Modern,
            severed_seconds: 2,
            ..Default::default()
        };
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &custom,
                &settings,
            )
            .unwrap();
        let graphics = crate::tenchu2::native::preferences::graphics(&settings);
        assert_eq!(graphics.blood, settings.graphics.blood);
        let vram = session.assets.sprites.vram.clone();
        let name = format!(
            "hero-{hero}-{}",
            if model.is_empty() {
                "native".into()
            } else {
                model.replace(':', "-")
            }
        );
        let mut bleeding = None;
        capture_frame(&vram, &graphics, &out.join(format!("{name}.png")), |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            let floor = floor();
            drawing.scene.environment.as_mut().unwrap().surfaces = crate::visuals::surfaces::Surfaces::new(&floor);
            session.world.controls.dismemberment = tenchu_core::game::dismemberment::Settings {
                player: true, player_head: head, player_arms: 100, player_right_arm: side,
                ..Default::default()
            };
            session.world.actors[player].actor.health = 1;
            wounds::tests::weapon_hit(&mut session, guard, player)?;
            assert_eq!(session.world.actors[player].actor.health, 0);
            assert!(session.world.actors[player].actor.dismemberment.is_some(), "lethal contact did not record optional death");
            let mut drops = false;
            let mut recorded = false;
            for tick in 0..160 {
                let state = session.capture_state()?;
                drawing.tick(&session, &graphics)?;
                assert!(state == session.capture_state()?);
                drops |= drawing.scene.environment.as_ref().unwrap().blood.has_drops();
                if tick == 20 {
                    assert!(session.world.actors[player].actor.severed().is_some());
                    let original = postcard::to_allocvec(&drawing.capture_state()).unwrap();
                    let mut restored = Drawing::new(&session, &graphics, renderer)?;
                    restored.restore_state(postcard::from_bytes(&original).unwrap(), &session, renderer)?;
                    assert!(original == postcard::to_allocvec(&restored.capture_state()).unwrap());
                    for alpha in [0.25, 1.0] {
                        drawing.alpha = alpha;
                        restored.alpha = alpha;
                        drawing.update(&session, renderer, None)?;
                        restored.update(&session, renderer, None)?;
                        assert!(postcard::to_allocvec(&drawing.capture_state()).unwrap() == postcard::to_allocvec(&restored.capture_state()).unwrap());
                    }
                    bleeding = Some((session.capture_state()?, original));
                    recorded = true;
                }
                session.step_world(0, true)?;
            }
            assert!(recorded && drops);
            assert!(!drawing.scene.environment.as_ref().unwrap().blood.has_drops(), "severed blood outlived duration and drop lifetime");
            let part = session.world.actors[player].actor.severed().unwrap();
            let rig = &session.world.instances[player].rig;
            let detached = session.world.scene.models[rig.joints[part.index]].local;
            let mut found = false;
            drawing.models.visit_actors(&session.world, &session.assets, &mut session.world.scene.clone(), None,
                |key, _, _, matrix, _| {
                    if matches!(key, crate::visuals::interpolation::Part::Actor { root, model, .. } if root == rig.root && model == rig.joints[part.index]) {
                        found = true;
                        assert!((Vec3::from(matrix.translation.to_array().map(|v| v as f32)) - Vec3::from(detached.translation.to_array().map(|v| v as f32))).length() < 2.0, "displayed part reattached");
                    }
                })?;
            assert!(found);
            drawing.clear_blood();
            for _ in 0..10 { session.step_world(0, true)?; drawing.tick(&session, &graphics)?; }
            assert!(!drawing.scene.environment.as_ref().unwrap().blood.has_drops());
            let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
            let center = Vec3::from(detached.translation.to_array().map(|v| v as f32));
            camera.position = center + Vec3::new(1800.0, -1800.0, 1500.0);
            let forward = (center - camera.position).normalize();
            camera.yaw = forward.x.atan2(forward.z);
            camera.pitch = (-forward.y).asin();
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.alpha = 1.0;
            drawing.update(&session, renderer, None)?;
            drawing.scene.terrain = renderer.upload(&floor);
            drawing.scene.props = renderer.upload(&[]);
            drawing.scene.backgrounds.clear();
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        }).unwrap();
        let (world, state) = bleeding.unwrap();
        session.restore_state(&world).unwrap();
        let mut without_blood = Vec::new();
        for (suffix, style) in [
            ("off", BloodStyle::Off),
            ("original", BloodStyle::Original),
            ("hd", BloodStyle::OriginalHd),
            ("modern", BloodStyle::Modern),
        ] {
            let mut current = graphics.clone();
            current.blood.severed_style = style;
            capture_frame(
                &vram,
                &current,
                &out.join(format!("{name}-bleeding-{suffix}.png")),
                |renderer| {
                    let mut drawing = Drawing::new(&session, &current, renderer)?;
                    drawing.restore_state(
                        postcard::from_bytes(&state).unwrap(),
                        &session,
                        renderer,
                    )?;
                    let actor = &session.world.actors[player].actor;
                    let joint =
                        session.world.instances[player].rig.joints[actor.severed().unwrap().index];
                    let part = session.world.scene.models[joint].local.translation;
                    let center = Vec3::from(
                        (actor.position.wrapping_add(part))
                            .to_array()
                            .map(|v| v as f32 / 2.0),
                    ) - Vec3::Y * 300.0;
                    let mut camera =
                        crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
                    camera.position = center + Vec3::new(1800.0, -1500.0, 2200.0);
                    let forward = (center - camera.position).normalize();
                    camera.yaw = forward.x.atan2(forward.z);
                    camera.pitch = (-forward.y).asin();
                    drawing.camera = Some(camera);
                    drawing.show_hud = false;
                    drawing.alpha = 1.0;
                    drawing.update(&session, renderer, None)?;
                    assert_eq!(
                        drawing
                            .scene
                            .environment
                            .as_ref()
                            .unwrap()
                            .blood
                            .has_drops(),
                        style != BloodStyle::Off
                    );
                    drawing.scene.terrain = renderer.upload(&floor());
                    drawing.scene.props = renderer.upload(&[]);
                    drawing.scene.backgrounds.clear();
                    Ok((drawing.scene, camera.source_view(), Vec::new()))
                },
            )
            .unwrap();
            let pixels = crate::visuals::pack::decode_png(
                &std::fs::read(out.join(format!("{name}-bleeding-{suffix}.png"))).unwrap(),
            )
            .unwrap()
            .rgba;
            if style == BloodStyle::Off {
                without_blood = pixels;
            } else {
                let red = pixels
                    .chunks_exact(4)
                    .zip(without_blood.chunks_exact(4))
                    .filter(|(a, b)| a != b && a[0] > 20 && a[0] / 2 > a[1] && a[0] / 2 > a[2])
                    .count();
                assert!(red > 5, "{name} {suffix}: severed droplets are not red");
            }
        }
    }
}

fn floor() -> Vec<crate::render_mesh::Vertex> {
    let mut floor = render_effects::quad(
        [
            [-6000.0, 0.0, -6000.0],
            [6000.0, 0.0, -6000.0],
            [-6000.0, 0.0, 6000.0],
            [6000.0, 0.0, 6000.0],
        ],
        [[0.3; 3]; 4],
        None,
        0,
    );
    for vertex in &mut floor {
        vertex.flags = crate::render_mesh::DOUBLE_SIDED;
    }
    floor
}
