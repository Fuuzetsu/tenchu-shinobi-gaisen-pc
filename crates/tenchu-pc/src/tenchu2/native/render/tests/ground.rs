use super::*;
use tenchu_core::tenchu2::effects::{blood::Spurt, surface};

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; live landed stains keep their coverage against the floor at several camera angles"]
fn live_ground_blood_keeps_coverage_against_the_floor() {
    let source = super::super::super::gaisen::Source::open(
        std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
        0,
        0,
        1,
        tenchu_assets::script::Language::English,
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = source
        .session(
            &disc,
            tenchu_core::tenchu2::world::Options {
                hero: 0,
                difficulty: 2,
                seed: 1,
                inventory: Default::default(),
            },
            false,
            4000,
        )
        .unwrap();
    session.gaisen.as_mut().unwrap().director.source.events = Default::default();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let position = session.world.actors[session.world.player]
        .actor
        .position
        .wrapping_add(Position::new(1600, -900, 0));
    session.world.effects = Default::default();
    session
        .world
        .effects
        .blood(
            Spurt {
                position,
                yaw: 0,
                speed: 100,
                gravity: 20,
            },
            &session.assets.resources.math,
            &session.assets.effect_rules,
            &mut session.world.random,
        )
        .unwrap();
    let quad = loop {
        session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        session.world.tick += 1;
        assert!(session.world.tick < 100);
        if let Some(quad) = session.frame.effects.quads.iter().find(|q| {
            q.blood == Some(surface::Blood::Ground)
                && q.parent.is_none()
                && q.position.y == 0
                && q.corners.iter().all(|p| p.y == 0)
        }) {
            break quad.clone();
        }
    };
    let world = postcard::to_allocvec(&session.world).unwrap();
    let center = Vec3::from(quad.position.to_array().map(|v| v as f32));
    for style in [
        crate::settings::BloodStyle::Original,
        crate::settings::BloodStyle::OriginalHd,
    ] {
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            blob_shadows: false,
            ..Default::default()
        };
        graphics.blood.ground = style;
        graphics.blood.retain_original = true;
        graphics.blood.conform_retained = true;
        for (angle, pitch) in [-0.35, -0.8, -1.2].into_iter().enumerate() {
            let mut camera = crate::debug::camera::Camera {
                position: center,
                yaw: 0.5,
                pitch,
                fov: 45.0,
            };
            camera.position -= camera.forward() * 1400.0;
            let mut coverage = Vec::new();
            for reference in [false, true] {
                session.frame.effects = Default::default();
                session.contact = Default::default();
                let mut contact = quad.clone();
                if reference {
                    // Use the same artwork and blending, raised clear of the floor.
                    // Retained decals have different opacity and cannot be a pixel reference.
                    contact.position.y -= 4;
                }
                session.frame.effects.quads.push(contact);
                let path = output.join(format!("live-blood-{style:?}-{angle}-{reference}.png"));
                capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                    let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                    drawing.camera = Some(camera);
                    drawing.show_hud = false;
                    drawing.update(&session, renderer, None)?;
                    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), world);
                    Ok((drawing.scene, camera.source_view(), Vec::new()))
                })
                .unwrap();
                let image =
                    crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap();
                coverage.push(
                    image
                        .rgba
                        .chunks_exact(4)
                        .filter(|p| {
                            p[0] > 40
                                && u16::from(p[0]) > u16::from(p[1]) * 2
                                && u16::from(p[0]) > u16::from(p[2]) * 2
                        })
                        .count(),
                );
            }
            eprintln!("{style:?} angle={angle} floor/reference coverage {coverage:?}");
            assert!(
                coverage[1] > 1000,
                "fixture did not draw a visible ground stain"
            );
            assert!(
                coverage[0].abs_diff(coverage[1]) * 100 < coverage[1] * 5,
                "coplanar live blood lost coverage against the floor: {coverage:?}"
            );
        }
    }
}
