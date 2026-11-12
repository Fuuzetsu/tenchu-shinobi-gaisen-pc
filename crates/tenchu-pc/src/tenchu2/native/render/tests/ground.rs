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

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; overlapping floor/wall stains remain stable as the camera moves"]
fn overlapping_blood_does_not_write_competing_depth() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = super::super::super::Session::load(&disc, 1, 1).unwrap();
    session.step(0).unwrap();
    let center = Vec3::new(59774.0, 500.0, 4035.0);
    session
        .world
        .teleport_player(center.as_ivec3().to_array().into(), true, &session.assets)
        .unwrap();
    let world = postcard::to_allocvec(&session.world).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let base_camera = crate::debug::camera::Camera {
        position: Vec3::new(63244.0, -3534.0, 4325.0),
        yaw: -1.655,
        pitch: -0.588,
        fov: 43.6,
    };
    let vram = session.assets.sprites.vram.clone();
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
        for wall in [false, true] {
            for shift in [-111.0, 0.0, 111.0] {
                let mut images = Vec::new();
                for reference in [false, true] {
                    let path = output.join(format!(
                        "blood-overlap-{style:?}-wall{wall}-{shift}-{reference}.png"
                    ));
                    capture_frame(&vram, &graphics, &path, |renderer| {
                        let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                        drawing.camera = Some(base_camera);
                        drawing.show_hud = false;
                        drawing.update(&session, renderer, None)?;
                        let (focus, normal, right, down) = if wall {
                            let start = center - Vec3::Y * 700.0;
                            let hit = drawing
                                .scene
                                .environment
                                .as_ref()
                                .unwrap()
                                .surfaces
                                .cast(start, start + Vec3::X * 12000.0)
                                .unwrap();
                            assert!(hit.normal.y.abs() < 0.1);
                            (hit.position, hit.normal, Vec3::Y.cross(hit.normal), Vec3::Y)
                        } else {
                            (center, Vec3::NEG_Y, Vec3::X, Vec3::Z)
                        };
                        let mut camera = if wall {
                            let eye = focus + normal * 5000.0 - Vec3::Y * 1200.0;
                            let direction = (focus - eye).normalize();
                            crate::debug::camera::Camera {
                                position: eye,
                                yaw: direction.x.atan2(direction.z),
                                pitch: (-direction.y).asin(),
                                fov: 43.6,
                            }
                        } else {
                            base_camera
                        };
                        camera.position += right * shift;
                        session.frame.effects = Default::default();
                        session.contact = Default::default();
                        for i in 0..16 {
                            let offset = right * (i * 12) as f32 + down * (i * 9) as f32;
                            session.frame.effects.quads.push(surface::Quad {
                                blood: Some(surface::Blood::Ground),
                                image: surface::Image::Effect(if wall { 3 } else { 2 }),
                                parent: None,
                                position: (focus + offset).as_ivec3().to_array().into(),
                                corners: [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].map(
                                    |(x, y)| {
                                        ((right * x + down * y) * 350.0)
                                            .to_array()
                                            .map(|v| v as i16)
                                            .into()
                                    },
                                ),
                                color: [(60 + i * 4) as u8; 3],
                                blend: surface::Blend::Opaque,
                            });
                        }
                        let mut geometry = drawing.models.geometry(
                            &session.world,
                            &session.assets,
                            &session.frame,
                            &graphics,
                            Presentation {
                                camera: Some(camera),
                                ..Default::default()
                            },
                        )?;
                        if reference {
                            // Independently compose the fixture's marks in insertion order,
                            // with scene depth testing but no writes between coplanar stains.
                            let mut marks: Vec<_> = geometry
                                .effects
                                .vertices
                                .chunks_exact(6)
                                .map(|v| v.to_vec())
                                .collect();
                            marks.sort_by(|a, b| a[0].color[0].total_cmp(&b[0].color[0]));
                            geometry.effects.vertices = marks.into_iter().flatten().collect();
                            for vertex in &mut geometry.effects.vertices {
                                vertex.flags |= render_mesh::ALPHA;
                            }
                            geometry.effects.batches = vec![render_effects::Batch {
                                blend: 5,
                                range: 0..geometry.effects.vertices.len() as u32,
                            }];
                        }
                        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), world);
                        Ok((
                            SceneDrawing::new(geometry, renderer, &session.assets.level),
                            camera.source_view(),
                            Vec::new(),
                        ))
                    })
                    .unwrap();
                    images.push(
                        crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap(),
                    );
                }
                let red = images[1]
                    .rgba
                    .chunks_exact(4)
                    .filter(|p| {
                        p[0] > 40
                            && u16::from(p[0]) > u16::from(p[1]) * 2
                            && u16::from(p[0]) > u16::from(p[2]) * 2
                    })
                    .count();
                assert!(red > 1000, "fixture must show overlapping blood");
                let changed = images[0]
                    .rgba
                    .chunks_exact(4)
                    .zip(images[1].rgba.chunks_exact(4))
                    .filter(|(a, b)| a.iter().zip(b.iter()).any(|(a, b)| a.abs_diff(*b) > 1))
                    .count();
                assert_eq!(
                    changed, 0,
                    "{style:?} wall={wall} shift={shift}: overlapping stains fought for depth or changed order"
                );
            }
        }
    }
}
