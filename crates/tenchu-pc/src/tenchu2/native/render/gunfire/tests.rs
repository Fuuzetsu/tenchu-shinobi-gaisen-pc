use super::*;
use crate::tenchu2::native::{Session, render};
use tenchu_assets::tenchu2::{audio::Audio, layout::Layout, registry::Registry};
use tenchu_core::{
    effects::GunPart,
    tenchu2::world::{Assets, Options, World},
};

fn fixture(body: bool) -> Session {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut layout = Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| {
            l.actors
                .iter()
                .any(|a| registry.clip(a.kind, 0x900) == Some(0x418) && a.behavior & 0xf00 == 0x100)
        })
        .unwrap();
    let mut gunner = layout
        .actors
        .iter()
        .find(|a| registry.clip(a.kind, 0x900) == Some(0x418) && a.behavior & 0xf00 == 0x100)
        .unwrap()
        .clone();
    gunner.position = [0, 0, 6000];
    gunner.facing = 0;
    gunner.route.clear();
    layout.start = [0; 3];
    layout.facing = 2048;
    layout.actors = vec![gunner];
    let hero = if layout.sequence >= 185 {
        2
    } else {
        usize::from(layout.sequence >= 101)
    };
    let (_, flat, _) = crate::tenchu2::tests::native_world::fixture();
    let mut assets = Assets::load(&disc, hero, &layout).unwrap();
    assets.level.collision = flat.level.collision;
    let mut world = World::new(
        &layout,
        Options {
            hero,
            difficulty: 2,
            seed: 123,
            inventory: Default::default(),
        },
        &assets,
    )
    .unwrap();
    world.finish_tick(&assets, 6).unwrap();
    if !body {
        for &joint in &world.instances[world.player].rig.joints {
            world.collision.remove(joint);
        }
    }
    let bank = std::sync::Arc::new(Audio::load(&disc, hero, layout.map).unwrap().bank);
    Session::from_parts(world, assets, bank)
}

fn camera(session: &Session) -> crate::debug::camera::Camera {
    let shot = &session.contact.gunshots.front().unwrap().1;
    let center = Vec3::from_array(shot.position.to_array().map(|v| v as f32));
    let position = center + Vec3::new(1600.0, -800.0, -1500.0);
    let forward = (center - position).normalize();
    crate::debug::camera::Camera {
        position,
        yaw: forward.x.atan2(forward.z),
        pitch: (-forward.y).asin(),
        fov: 45.0,
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; real source gunshots retain finite cosmetic effects across saves and arbitrary redraws"]
fn native_gunfire_follows_source_shots_and_survives_restore() {
    for body in [false, true] {
        let mut session = fixture(body);
        for _ in 0..2400 {
            session.step(0).unwrap();
            if session
                .contact
                .gunshots
                .iter()
                .any(|(_, s)| matches!(s.part, GunPart::Muzzle))
            {
                break;
            }
        }
        assert!(
            session
                .contact
                .gunshots
                .iter()
                .any(|(_, s)| matches!(s.part, GunPart::Muzzle))
        );
        let camera = camera(&session);
        let tick = u64::from(session.world.tick);
        let saved = session.capture_state().unwrap();
        for alpha in [0.0, 0.5, 1.0] {
            let effects = geometry(&session.contact, tick, camera.matrix(16.0 / 9.0), alpha);
            assert!(!effects.vertices.is_empty());
            assert!(
                effects
                    .vertices
                    .iter()
                    .all(|v| v.position.iter().all(|x| x.is_finite()))
            );
            assert_eq!(lights(&session.contact, tick, alpha).len(), 1);
        }
        assert!(session.capture_state().unwrap() == saved);
        let mut hit = false;
        for _ in 0..20 {
            session.step(0).unwrap();
            hit |= session.contact.gunshots.iter().any(|(_, s)| match s.part {
                GunPart::Body => body,
                GunPart::Surface => !body,
                GunPart::Muzzle => false,
            });
        }
        assert!(hit, "body {body}: missing source impact");
        let expected = session.capture_state().unwrap();
        assert!(session.contact.gunshots.is_empty());
        assert!(lights(&session.contact, u64::from(session.world.tick), 1.0).is_empty());
        assert!(
            geometry(
                &session.contact,
                u64::from(session.world.tick),
                camera.matrix(16.0 / 9.0),
                1.0
            )
            .vertices
            .is_empty()
        );
        session.restore_state(&saved).unwrap();
        for _ in 0..20 {
            for _ in 0..3 {
                let _ = geometry(
                    &session.contact,
                    u64::from(session.world.tick),
                    camera.matrix(16.0 / 9.0),
                    0.5,
                );
            }
            session.step(0).unwrap();
        }
        assert!(session.capture_state().unwrap() == expected);
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; F1 gunfire flashes/light/smoke can be disabled while paused without changing the world"]
fn native_gunfire_renders_and_toggles_without_changing_source_state() {
    let mut session = fixture(true);
    for _ in 0..2400 {
        session.step(0).unwrap();
        if !session.contact.gunshots.is_empty() {
            break;
        }
    }
    let camera = camera(&session);
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let state = session.capture_state().unwrap();
    let mut images = Vec::new();
    for enabled in [false, true] {
        let mut settings = crate::settings::Settings::default();
        settings.set_visual_preset(crate::settings::presets::MODERN_HIGH);
        settings.graphics.asset_pack = false;
        settings.graphics.fog = false;
        settings.graphics.effects.gunfire = enabled;
        let graphics = crate::tenchu2::native::preferences::graphics(&settings);
        let path = out.join(format!("native-gunfire-{enabled}.png"));
        render::capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = render::Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            assert_eq!(
                drawing
                    .scene
                    .lighting
                    .transients
                    .iter()
                    .filter(|s| s.name == "Gun muzzle")
                    .count(),
                usize::from(enabled)
            );
            let mut off = graphics.clone();
            off.effects.gunfire = false;
            renderer.configure(&off);
            drawing.update(&session, renderer, None)?;
            assert!(
                !drawing
                    .scene
                    .lighting
                    .transients
                    .iter()
                    .any(|s| s.name == "Gun muzzle")
            );
            renderer.configure(&graphics);
            drawing.update(&session, renderer, None)?;
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
        assert!(session.capture_state().unwrap() == state);
    }
    let changed = images[0]
        .rgba
        .chunks_exact(4)
        .zip(images[1].rgba.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(changed > 20, "gunfire has no visible effect: {changed}");
}
