use super::*;

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; verifies sky/background resources and foreground depth"]
fn native_backgrounds_stay_behind_the_scene() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    for sequence in [1, 9] {
        let mut session = super::super::Session::load(&disc, sequence, 1).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let mut images = Vec::new();
        for mode in ["scene", "background", "combined"] {
            let path = output.join(format!("jp-background-{sequence}-{mode}.png"));
            capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                let models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
                let mut geometry =
                    models.geometry(&session.world, &session.assets, &session.frame)?;
                assert_eq!(geometry.backgrounds.len(), 3);
                if mode == "scene" {
                    geometry.backgrounds.clear();
                }
                if mode == "background" {
                    geometry.terrain.clear();
                    geometry.actors.clear();
                    geometry.effects.vertices.clear();
                    geometry.effects.batches.clear();
                }
                Ok((
                    SceneDrawing::new(geometry, renderer),
                    session.world.camera.view,
                    Vec::new(),
                ))
            })
            .unwrap();
            images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
        }
        let [r, g, b] = graphics.post.fog_color().map(|v| (v * 255.0).round() as u8);
        let clear = [r, g, b, 255];
        let mut filled = 0;
        let mut foreground = 0;
        for (before, after) in images[0]
            .rgba
            .chunks_exact(4)
            .zip(images[2].rgba.chunks_exact(4))
        {
            if before == clear && before != after {
                filled += 1;
            }
            if before != clear && before == after {
                foreground += 1;
            }
        }
        assert!(
            filled > 5000,
            "sequence {sequence}: background did not fill the horizon"
        );
        assert!(
            foreground > 50000,
            "sequence {sequence}: background covered the foreground"
        );
    }
}

#[test]
#[ignore = "requires Japan TENCHU2_DISC, TENCHU2_ASSET_PACK, TENCHU_MODEL_REVIEW and a GPU; original and replacement water scroll through a complete cycle"]
fn native_water_animation_survives_texture_replacement() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 1, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let meshes = prepare(
        &session.assets.level.vram,
        sources("map0/terrain", &session.assets.level.map),
        &Graphics {
            asset_pack: false,
            ..Default::default()
        },
    )
    .unwrap();
    let animated: Vec<_> = meshes
        .materials()
        .iter()
        .enumerate()
        .filter_map(|(i, m)| m.animation.is_some().then_some(i as u32 + 1))
        .collect();
    let mut surfaces = Vec::new();
    for part in &session.assets.level.map {
        meshes.visit(
            &part.geometry,
            Matrix {
                translation: part.position.map(|v| i32::from(v) * 10).into(),
                ..Default::default()
            },
            true,
            0,
            None,
            |triangle| {
                if animated.contains(&triangle[0].material) {
                    surfaces.push(Position::from_fn(|i| {
                        (triangle.iter().map(|v| v.position[i]).sum::<f32>() / 3.0) as i32
                    }));
                }
            },
        );
    }
    let player = session.world.actors[0].actor.position;
    let center = surfaces
        .into_iter()
        .min_by_key(|p| {
            p.iter()
                .zip(player)
                .map(|(a, b)| i64::from(a - b).pow(2))
                .sum::<i64>()
        })
        .expect("no animated water geometry");
    let eye = center.wrapping_sub(Position::new(0, 6000, 0));
    session.world.camera.view = Matrix {
        rotation: [[4096, 0, 0], [0, 0, -4096], [0, 4096, 0]],
        translation: Position::new(-eye.x, eye.z, -eye.y),
    };
    eprintln!("Viewing native water at {center:?}");
    for replaced in [false, true] {
        let graphics = Graphics {
            asset_pack: replaced,
            fog: false,
            ..Default::default()
        };
        let mut images = Vec::new();
        for tick in [30, 31, 46] {
            // Freeze the world and camera; only the two water texture phases advance.
            session.world.tick = tick;
            let path = output.join(format!("jp-water-{replaced}-{tick}.png"));
            capture(&session, &graphics, &path).unwrap();
            images.push(std::fs::read(path).unwrap());
        }
        assert!(
            images[0] != images[1],
            "water did not scroll (HD={replaced})"
        );
        assert!(
            images[0] == images[2],
            "water must repeat after both source textures wrap (HD={replaced})"
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; exercises original joint counts across every native character and costume"]
fn native_character_geometry_keeps_all_source_joints() {
    use tenchu_assets::tenchu2::characters;
    use tenchu_core::{math::Math, motion::Pose, scene::Scene};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut extended = 0;
    for entry in characters::load(&disc, None).unwrap() {
        let model = characters::rig(&disc, &entry.name).unwrap();
        let mut meshes = build(
            &model.vram,
            sources(&format!("actors/{}", entry.name), &model.parts),
            &Graphics::default(),
        )
        .unwrap();
        meshes.skin(&model.parts);
        extended += usize::from(model.parts.len() > 15);
        let mut scene = Scene::default();
        let root = scene.add(None, Matrix::default());
        let joints = Pose::new(&model.parts)
            .unwrap()
            .bind(&mut scene, root, &math);
        let mut matrices = joints
            .into_iter()
            .map(|joint| scene.world(joint).unwrap())
            .collect::<Vec<_>>();
        for pass in 0..2 {
            if pass == 1 {
                for (i, matrix) in matrices.iter_mut().enumerate() {
                    matrix.rotation = math.rotate_yxz([0, i as i16 * 128, 0]);
                }
            }
            meshes.pose_skin(&matrices, None);
            let mut count = 0;
            for (part, &matrix) in model.parts.iter().zip(&matrices) {
                assert!(
                    meshes.visit(&part.geometry, matrix, true, 0, None, |triangle| {
                        for vertex in triangle {
                            assert!(
                                vertex.position.iter().all(|v| v.is_finite()),
                                "{}",
                                entry.name
                            );
                            assert!(
                                vertex.normal.iter().all(|v| v.is_finite()),
                                "{}",
                                entry.name
                            );
                            count += 1;
                        }
                    })
                );
            }
            assert!(count > 0, "{}", entry.name);
        }
    }
    assert!(extended > 0, "did not exercise JP's extended rigs");
    eprintln!("{extended} native bodies with more than fifteen joints");
}
