use super::*;
use crate::tenchu2::native::Session;

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; imported-stage guards use neutral shade indices rather than the resource donor's blue palette"]
fn imported_guards_do_not_inherit_the_resource_donor_palette() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let source = tenchu_assets::stage::MissionAssets::load(&gaisen, 0, 0, 1).unwrap();
    let donor = Level::load(&disc, 8).unwrap().shading;
    assert!((0..=255).any(|index| {
        let [r, g, b] = donor.color(index);
        b > r && b > g
    }));
    let mut session = Session::gaisen(
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
    .unwrap();
    let neutral = session.assets.level.shading.clone();
    for index in 0..=255 {
        assert_eq!(neutral.color(index), [index; 3]);
    }
    session.world.keep_all_active = true;
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let guard = session
        .world
        .actors
        .iter()
        .find(|a| (0x10..0x60).contains(&a.actor.kind))
        .unwrap();
    let center =
        Vec3::from_array(guard.actor.position.to_array().map(|v| v as f32)) - Vec3::Y * 800.0;
    let eye = center + Vec3::new(1400.0, -500.0, -2800.0);
    let forward = (center - eye).normalize();
    let camera = crate::debug::camera::Camera {
        position: eye,
        yaw: forward.x.atan2(forward.z),
        pitch: (-forward.y).asin(),
        fov: 45.0,
    };
    let graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let state = postcard::to_allocvec(&session.world).unwrap();
    let mut images = Vec::new();
    for (label, shading) in [("before", donor), ("after", neutral)] {
        session.assets.level.shading = shading;
        let path = output.join(format!("imported-guards-{label}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
            let geometry = models.geometry(
                &session.world,
                &session.assets,
                &session.frame,
                &graphics,
                Presentation {
                    camera: Some(camera),
                    ..Default::default()
                },
            )?;
            assert!(geometry.actors.len() > 100);
            if label == "after" {
                assert!(
                    geometry
                        .actors
                        .iter()
                        .all(|v| { v.color[0] == v.color[1] && v.color[1] == v.color[2] })
                );
            }
            Ok((
                SceneDrawing::new(geometry, renderer, &session.assets.level),
                camera.source_view(),
                Vec::new(),
            ))
        })
        .unwrap();
        assert!(state == postcard::to_allocvec(&session.world).unwrap());
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
    }
    let corrected = images[0]
        .rgba
        .chunks_exact(4)
        .zip(images[1].rgba.chunks_exact(4))
        .filter(|(before, after)| {
            let blue = |p: &[u8]| i16::from(p[2]) - i16::from(p[0].max(p[1]));
            blue(before) > 8 && blue(after) < blue(before)
        })
        .count();
    assert!(
        corrected > 100,
        "no visible blue guard tint was removed: {corrected}"
    );
}
