use super::*;

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; shared indexed meshes preserve native palette shading, triangle order and CPU/GPU output"]
fn indexed_actors_preserve_native_shading_and_fallbacks() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut posed = 0;
    let mut skinned = 0;
    for imported in [false, true] {
        let mut session = if imported {
            let source = tenchu_assets::stage::MissionAssets::load(&gaisen, 0, 0, 1).unwrap();
            super::super::super::Session::gaisen(
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
            super::super::super::Session::load(&disc, 9, 1).unwrap()
        };
        session.world.keep_all_active = true;
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        if imported {
            let library = crate::visuals::appearance::library::Library::load(&gaisen).unwrap();
            let mut donor = crate::session::Options::default().load(&gaisen).unwrap();
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &std::sync::Arc::default(),
                    &crate::settings::Settings {
                        player_model: "disc:1".into(),
                        ..Default::default()
                    },
                )
                .unwrap();
            session.step(0).unwrap();
        }
        let state = postcard::to_allocvec(&session.world).unwrap();
        for detail in [0, 2] {
            let mut graphics = Graphics {
                fog: false,
                asset_pack: false,
                ..Default::default()
            };
            graphics.models.characters = detail;
            let mut images = Vec::new();
            for gpu in [false, true] {
                let path = output.join(format!("indexed-{imported}-{detail}-{gpu}.png"));
                capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                    let models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
                    let mut indexed =
                        Builder::new(session.world.camera.view, &session.assets.level, &graphics);
                    let mut triangles =
                        Builder::new(session.world.camera.view, &session.assets.level, &graphics);
                    // Keep the source fog/shading branch, with its threshold beyond every actor.
                    triangles.fog_start = Some(i32::MAX / 2);
                    models.visit_actors(
                        &session.world,
                        &session.assets,
                        &mut session.world.scene.clone(),
                        None,
                        |_, meshes, part, matrix, style| {
                            indexed.model(meshes, part, matrix, style);
                            triangles.model(meshes, part, matrix, style);
                        },
                    )?;
                    assert_eq!(
                        bytemuck::cast_slice::<_, u8>(&indexed.actors.expanded()),
                        bytemuck::cast_slice::<_, u8>(&triangles.actors.expanded())
                    );
                    assert!(indexed.actors.vertices.len() < indexed.actors.indices.len());
                    let a = render_effects::surface_geometry(indexed.surfaces);
                    let b = render_effects::surface_geometry(triangles.surfaces);
                    assert_eq!(
                        bytemuck::cast_slice::<_, u8>(&a.vertices),
                        bytemuck::cast_slice::<_, u8>(&b.vertices)
                    );
                    let geometry = models.geometry(
                        &session.world,
                        &session.assets,
                        &session.frame,
                        &graphics,
                        Presentation {
                            actors: render_mesh::Indexed {
                                poses: gpu.then(|| renderer.pose_frame()).flatten(),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    )?;
                    posed += geometry.actors.poses.as_ref().map_or(0, |f| f.parts.len());
                    if let Some(frame) = &geometry.actors.poses {
                        skinned += frame
                            .parts
                            .iter()
                            .filter(|part| !part.skin.is_empty())
                            .count();
                        for group in &geometry.actors.groups {
                            if group.vertices.end > indexed.actors.vertices.len() {
                                break;
                            }
                            let bounds =
                                frame.bounds(&geometry.actors.vertices, group.vertices.clone());
                            if let Some([min, max]) = bounds.shadow {
                                for vertex in &indexed.actors.vertices[group.vertices.clone()] {
                                    let position = glam::Vec3A::from(vertex.position);
                                    assert!(
                                        position.cmpge(min).all() && position.cmple(max).all(),
                                        "weighted pose escaped its shadow bounds"
                                    );
                                }
                            }
                        }
                    }
                    assert_eq!(state, postcard::to_allocvec(&session.world).unwrap());
                    Ok((
                        SceneDrawing::new(geometry, renderer, &session.assets.level),
                        session.world.camera.view,
                        Vec::new(),
                    ))
                })
                .unwrap();
                images.push(
                    crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                        .unwrap()
                        .rgba,
                );
            }
            let changed = images[0]
                .chunks_exact(4)
                .zip(images[1].chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            eprintln!(
                "indexed imported={imported}, detail={detail}: {changed} differing GPU pixels"
            );
            // CPU/GPU floating-point transforms can move a raster edge by a subpixel.
            assert!(changed < images[0].len() / 4 / 1000);
        }
    }
    assert!(posed > 0, "fixture did not exercise shared GPU posing");
    assert!(skinned > 0, "fixture did not exercise weighted GPU posing");
}
