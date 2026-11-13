use super::*;
use glam::Vec3;
mod gaisen;
mod ground;
mod heroes;
mod indexed;
mod items;

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; sound marker geometry, labels and photo-mode suppression"]
fn training_sound_overlay_is_visible_without_changing_the_scene() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let session = super::super::Session::load(&disc, 1, 1).unwrap();
    let marker = super::super::inspection::sounds::markers(&session).remove(0);
    let camera = crate::demo::framing::Path {
        eye: (marker.position + Vec3::new(1000.0, -2300.0, -6000.0)).to_array(),
        target: marker.position.to_array(),
        end_eye: None,
        end_target: None,
        follow_player: false,
        fov: 55.0,
    }
    .sample(0.0);
    let graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    let before = postcard::to_allocvec(&session.world).unwrap();
    let mut captures = Vec::new();
    for (name, sound_triggers, photo_mode) in [
        ("off", false, false),
        ("on", true, false),
        ("photo", true, true),
    ] {
        let path = output.join(format!("training-sounds-{name}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            let mut debug = crate::debug::Debug::default();
            debug.sound_triggers = sound_triggers;
            debug.photo_mode = photo_mode;
            debug.xray = true;
            drawing.update(&session, renderer, Some(&debug))?;
            assert_eq!(
                drawing
                    .labels
                    .iter()
                    .any(|label| label.1.starts_with("Sign ")),
                sound_triggers && !photo_mode
            );
            let texts = drawing
                .labels
                .into_iter()
                .filter_map(|crate::debug::Label(anchor, text, color)| {
                    let crate::debug::Anchor::Projected([x, y]) = anchor else {
                        return None;
                    };
                    Some(crate::fonts::Text {
                        position: egui::pos2((x - 0.5) * (240.0 * 16.0 / 9.0), (y - 0.5) * 240.0),
                        text,
                        color: crate::fonts::color(color),
                        size: 5.0,
                        width: 200.0,
                        centered: false,
                    })
                })
                .collect();
            Ok((drawing.scene, camera.source_view(), texts))
        })
        .unwrap();
        captures.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    assert_ne!(captures[0], captures[1]);
    assert_eq!(captures[0], captures[2]);
    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; checks opaque night scenery independently of particles"]
fn native_night_scenery_does_not_depend_on_the_effect_pass() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..90 {
        session.step(0).unwrap();
    }
    let mut images = Vec::new();
    for effects in [true, false] {
        let path = output.join(format!("jp-night-effects-{effects}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
            let mut geometry = models.geometry(
                &session.world,
                &session.assets,
                &session.frame,
                &graphics,
                Presentation {
                    show_hud: true,
                    ..Default::default()
                },
            )?;
            if !effects {
                geometry.effects.vertices.clear();
                geometry.effects.batches.clear();
            }
            Ok((
                SceneDrawing::new(geometry, renderer, &session.assets.level),
                session.world.camera.view,
                Vec::new(),
            ))
        })
        .unwrap();
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
    }
    let unchanged = images[0]
        .rgba
        .chunks_exact(4)
        .zip(images[1].rgba.chunks_exact(4))
        .filter(|(a, b)| a == b)
        .count();
    assert!(
        unchanged * 100 > images[0].rgba.len() / 4 * 95,
        "removing particles also removed opaque scenery"
    );
}

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
                let mut geometry = models.geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation {
                        show_hud: true,
                        ..Default::default()
                    },
                )?;
                assert_eq!(geometry.backgrounds.len(), 3);
                if mode == "scene" {
                    geometry.backgrounds.clear();
                }
                if mode == "background" {
                    geometry.terrain.clear();
                    geometry.props.clear();
                    geometry.actors.clear(None);
                    geometry.effects.vertices.clear();
                    geometry.effects.batches.clear();
                }
                Ok((
                    SceneDrawing::new(geometry, renderer, &session.assets.level),
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
        sources("map0/terrain", &session.assets.level.map, Role::Terrain),
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
            capture(
                &session,
                &crate::settings::Settings {
                    graphics: graphics.clone(),
                    ..Default::default()
                },
                &path,
            )
            .unwrap();
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
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; shared water must render, animate and restore Original without simulation changes"]
fn native_modern_water_uses_shared_materials_without_changing_logic() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 1, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let before = postcard::to_allocvec(&session.world).unwrap();
    let mut settings = crate::settings::Settings::default();
    settings.graphics.asset_pack = false;
    settings.graphics.fog = false;
    settings.graphics.water.enabled = true;
    let graphics = super::super::preferences::graphics(&settings);
    let center = nearest_water(&session, &graphics);
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    camera.pitch = -1.55;
    camera.position = center - camera.forward() * 6000.0;
    let mut images = Vec::new();
    for mode in ["original", "modern", "reflective", "restored"] {
        settings.graphics.water.enabled = matches!(mode, "modern" | "reflective");
        settings.graphics.water.reflective = mode == "reflective";
        let graphics = super::super::preferences::graphics(&settings);
        let path = output.join(format!("jp-water-shared-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            assert_eq!(drawing.scene.environment.is_some(), graphics.water.enabled);
            if let Some(environment) = &mut drawing.scene.environment {
                let frame = environment::Frame {
                    level: &session.assets.level,
                    meshes: &drawing.models.scenery,
                    math: &session.assets.resources.math,
                    tick: u64::from(session.world.tick),
                    camera: camera.matrix(16.0 / 9.0),
                    alpha: 1.0,
                    contact: None,
                    world: None,
                    scenery: None,
                    sprites: &session.assets.sprites,
                };
                let spray = environment.update(&frame, &graphics, renderer);
                assert!(
                    !spray.vertices.is_empty(),
                    "source waterfall should feed shared spray"
                );
                let mut disabled = graphics.clone();
                disabled.water.spray = 0;
                assert!(
                    environment
                        .update(&frame, &disabled, renderer)
                        .vertices
                        .is_empty()
                );
            }
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    for pair in images[..3].windows(2) {
        let changed = pair[0]
            .chunks_exact(4)
            .zip(pair[1].chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed > 100,
            "water effect did not reach renderer: {changed}"
        );
    }
    assert_eq!(images[0], images[3]);
    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
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
            sources(&format!("actors/{}", entry.name), &model.parts, Role::Actor),
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

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; native sources use common lighting/shadows without touching simulation"]
fn native_sources_share_lighting_and_inspection_without_changing_logic() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..40 {
        session.step(0).unwrap();
    }
    let before = serde_json::to_vec(&session.world).unwrap();
    let mut graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    graphics.lighting.enabled = true;
    graphics.lighting.count = 8;
    graphics.lighting.shadow_count = 4;
    let mut sources = lighting::Lighting::new(&session.assets.level);
    let player = glam::Vec3::from(
        session.world.actors[session.world.player]
            .actor
            .position
            .to_array()
            .map(|v| v as f32),
    );
    sources.focus = Some(player);
    let lights = sources.sources(&graphics.lighting, player);
    assert!(!lights.is_empty(), "night stage has no mapped flames");
    assert_eq!(
        lights.iter().map(|l| l.position).collect::<Vec<_>>(),
        sources
            .sources(&graphics.lighting, player + glam::Vec3::X * 100000.0)
            .iter()
            .map(|l| l.position)
            .collect::<Vec<_>>(),
        "camera orbit must not change light/shadow priority"
    );
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    camera.focus(lights[0].position, false);
    let mut images = Vec::new();
    for mode in [
        "original",
        "lit",
        "shadowed",
        "inspection",
        "flicker",
        "hdr",
        "heat",
        "markers",
        "movable",
    ] {
        graphics.lighting.enabled = mode != "original";
        graphics.lighting.shadows = mode != "lit";
        graphics.lighting.flame_motion = if mode == "flicker" { 100 } else { 0 };
        graphics.lighting.heat_shimmer = if mode == "heat" { 100 } else { 0 };
        graphics.lighting.markers = mode == "markers";
        graphics.lighting.fires = mode != "movable";
        graphics.post.enabled = matches!(mode, "hdr" | "heat");
        let path = output.join(format!("native-shared-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            let mut debug = crate::debug::Debug::default();
            debug.hitboxes = mode == "inspection";
            debug.xray = true;
            if mode == "movable" {
                debug.light = Some(player + glam::Vec3::new(2000.0, -4000.0, 2000.0));
            }
            drawing.update(&session, renderer, Some(&debug))?;
            if mode == "markers" {
                assert!(!drawing.labels.is_empty());
            }
            if mode == "movable" {
                assert!(
                    drawing.scene.environment.is_some(),
                    "movable light needs shadow casters without flames"
                );
                let sources = drawing
                    .scene
                    .lighting
                    .named_sources(&graphics.lighting, player);
                assert_eq!(sources.len(), 1);
                assert_eq!(Some(sources[0].light.position), debug.light);
            }
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
    }
    for pair in images.windows(2) {
        let changed = pair[0]
            .rgba
            .chunks_exact(4)
            .zip(pair[1].rgba.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed > 100,
            "lighting, shadows or inspection failed to reach the shared renderer: {changed}"
        );
    }
    assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; native sky uses the shared shading and shadow pipeline"]
fn native_sky_lighting_and_shadows_do_not_change_simulation() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..40 {
        session.step(0).unwrap();
    }
    let before = serde_json::to_vec(&session.world).unwrap();
    let mut graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    graphics.lighting.count = 0;
    graphics.lighting.ambient = 55;
    graphics.lighting.directional.intensity = 100;
    let mut images = Vec::new();
    for mode in ["original", "moon", "moon-shadows", "noon", "night"] {
        graphics.lighting.enabled = matches!(mode, "moon" | "moon-shadows");
        graphics.lighting.shadows = mode != "moon";
        graphics.sky.enabled = matches!(mode, "noon" | "night");
        graphics.sky.time = if mode == "noon" { 720 } else { 0 };
        let path = output.join(format!("native-sky-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            assert_eq!(renderer.daylight.is_some(), graphics.sky.enabled);
            if mode == "moon-shadows" || graphics.sky.enabled {
                assert!(
                    drawing.scene.environment.is_some(),
                    "missing native shadow casters"
                );
            }
            Ok((drawing.scene, session.world.camera.view, Vec::new()))
        })
        .unwrap();
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
    }
    for pair in images.windows(2) {
        let changed = pair[0]
            .rgba
            .chunks_exact(4)
            .zip(pair[1].rgba.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed > 100,
            "sky option did not change the rendered scene: {changed}"
        );
    }
    assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);

    let mut story = super::super::story::Story::load(&disc, 0, 201, 1).unwrap();
    for _ in 0..60 {
        story.step(true).unwrap();
    }
    graphics.sky.time = 720;
    graphics.sky.speed = 120;
    capture_frame(
        &story.level.vram.clone(),
        &graphics,
        &output.join("native-sky-cutscene.png"),
        |renderer| {
            let mut drawing = StoryDrawing::new(&story, &graphics, renderer)?;
            drawing.update(&story, renderer, None)?;
            for _ in 0..30 {
                story.step(true)?;
            }
            let before = serde_json::to_vec(&story.state).unwrap();
            drawing.update(&story, renderer, None)?;
            let minutes = drawing.scene.sky.minutes(&graphics.sky, 1.0);
            assert_eq!(minutes, 840.0);
            let mut rebuilt = StoryDrawing::new(&story, &graphics, renderer)?;
            rebuilt.preserve_environment(&mut drawing);
            rebuilt.update(&story, renderer, None)?;
            assert_eq!(rebuilt.scene.sky.minutes(&graphics.sky, 1.0), minutes);
            assert!(rebuilt.scene.environment.is_some());
            assert!(renderer.daylight.is_some());
            assert_eq!(serde_json::to_vec(&story.state).unwrap(), before);
            Ok((rebuilt.scene, story.state.view, rebuilt.texts))
        },
    )
    .unwrap();
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; renders shared orbit following a native guard"]
fn native_follow_camera_uses_the_shared_scene_surfaces() {
    use crate::settings::menu::Item;
    use crate::tenchu2::native::{campaign::Content, presentation::ContentDrawing, tools::Tools};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let content = Content::Gameplay(Box::new(session));
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    let before = postcard::to_allocvec(&session.world).unwrap();
    let guard = session
        .world
        .actors
        .iter()
        .enumerate()
        .find(|(id, agent)| *id != session.world.player && agent.actor.flags & 8 == 0)
        .map(|(id, _)| id)
        .expect("visible guard");
    let mut tools = Tools::default();
    tools.sync(&content, 0);
    tools.debug.selected_actor = guard;
    tools.change(Item::FollowActor, 1).unwrap();
    let graphics = Graphics {
        fog: false,
        ..Default::default()
    };
    for relative in [false, true] {
        if relative {
            tools.debug.return_camera();
        }
        capture_frame(
            &session.assets.sprites.vram,
            &graphics,
            &output.join(format!("jp-orbit-relative-{relative}.png")),
            |renderer| {
                let mut drawing = ContentDrawing::new(&content, &graphics, renderer)?;
                tools.update_camera(
                    &content,
                    &mut drawing,
                    &crate::settings::Navigation {
                        camera_relative: relative,
                        ..Default::default()
                    },
                    1.0 / 60.0,
                    renderer,
                );
                let camera = tools.debug.follow_camera.expect("follow camera");
                drawing.update(
                    &content,
                    renderer,
                    &tools.debug,
                    &crate::settings::Settings {
                        show_hud: false,
                        ..Default::default()
                    },
                )?;
                let ContentDrawing::Gameplay(drawing) = drawing else {
                    unreachable!()
                };
                let target = Vec3::from(
                    session.world.actors[if relative {
                        session.world.player
                    } else {
                        guard
                    }]
                    .actor
                    .position
                    .to_array()
                    .map(|v| v as f32),
                ) - Vec3::Y * 1100.0;
                let environment = drawing.scene.environment.as_ref().expect("shared surfaces");
                assert!(
                    !environment.surfaces.blocked(camera.position, target),
                    "camera crossed a wall"
                );
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            },
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; renders distinct in-between body/weapon poses without changing source state"]
fn native_interpolated_geometry_matches_current_pose_at_endpoint() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    session.interpolate(true).unwrap();
    for _ in 0..50 {
        session.step(tenchu_core::tenchu2::input::UP).unwrap();
    }
    let before = postcard::to_allocvec(&session.world).unwrap();
    let graphics = Graphics {
        interpolation: true,
        fog: false,
        ..Default::default()
    };
    let camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    let mut vertices = Vec::new();
    for (index, alpha) in [0.0, 0.5, 1.0].into_iter().enumerate() {
        capture_frame(
            &session.assets.sprites.vram,
            &graphics,
            &output.join(format!("jp-interpolation-{index}.png")),
            |renderer| {
                let pose = session
                    .interpolation
                    .as_ref()
                    .unwrap()
                    .sample(u64::from(session.world.tick), alpha);
                let models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
                let geometry = models.geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation {
                        pose: pose.as_ref(),
                        camera: Some(camera),
                        ..Default::default()
                    },
                )?;
                vertices.push(bytemuck::cast_slice::<_, u8>(&geometry.actors.expanded()).to_vec());
                if alpha == 1.0 {
                    let exact = models.geometry(
                        &session.world,
                        &session.assets,
                        &session.frame,
                        &graphics,
                        Presentation {
                            camera: Some(camera),
                            ..Default::default()
                        },
                    )?;
                    assert_eq!(
                        vertices.last().unwrap(),
                        bytemuck::cast_slice::<_, u8>(&exact.actors.expanded())
                    );
                }
                Ok((
                    SceneDrawing::new(geometry, renderer, &session.assets.level),
                    camera.source_view(),
                    Vec::new(),
                ))
            },
        )
        .unwrap();
    }
    assert_ne!(vertices[0], vertices[1]);
    assert_ne!(vertices[1], vertices[2]);
    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
}

#[test]
#[ignore = "requires TENCHU_DISC, Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; custom rigs and reloads must not change source logic"]
fn native_custom_bodies_keep_source_state_and_weapon_attachments() {
    use crate::settings::Settings;
    use crate::tenchu2::native::models;
    use crate::visuals::appearance::library::Library;
    use std::sync::Arc;
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    let folder = output.join("custom-models");
    let mut donor = models::donor(&gaisen).unwrap();
    crate::visuals::export(&donor.mission, &folder).unwrap();
    let mut manifest: crate::visuals::pack::Manifest =
        serde_json::from_slice(&std::fs::read(folder.join("pack.json")).unwrap()).unwrap();
    let body = manifest.player_models.get_mut("0000").unwrap();
    body.name = "Attachment check".into();
    body.parts[13].position[0] += 300;
    body.parts[14].position[0] -= 300;
    for part in 0..3 {
        let rules = manifest.motion[&format!("actors/0000/{part}")].clone();
        manifest.motion.insert(format!("player/0000/{part}"), rules);
    }
    std::fs::write(
        folder.join("pack.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let pack = Pack::bodies(folder.to_str().unwrap()).unwrap();
    let settings = Settings {
        player_model: "pack:0000".into(),
        ..Default::default()
    };
    let graphics = Graphics {
        fog: false,
        asset_pack: false,
        models: crate::settings::Models {
            heroes: true,
            ..Default::default()
        },
        motion: crate::settings::Motion {
            hair: true,
            cloth: true,
            equipment: true,
            wind: 80,
            ..Default::default()
        },
        ..Default::default()
    };
    let layout = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    for player in 0..3 {
        let sequence = layout
            .iter()
            .find(|l| match player {
                0 => l.sequence < 100,
                1 => (100..185).contains(&l.sequence),
                _ => l.sequence >= 185,
            })
            .unwrap()
            .sequence;
        let mut session = super::super::Session::load(&disc, sequence, 1).unwrap();
        let mut reference = super::super::Session::load(&disc, sequence, 1).unwrap();
        let library = Library::default();
        let before = serde_json::to_vec(&session.world).unwrap();
        assert!(
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &pack,
                    &settings
                )
                .unwrap()
        );
        assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
        assert!(
            !session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &pack,
                    &settings
                )
                .unwrap()
        );
        for tick in 0..75 {
            let input = match tick {
                30..=45 => tenchu_core::tenchu2::input::UP,
                55 => tenchu_core::tenchu2::input::ATTACK,
                _ => 0,
            };
            session.step(input).unwrap();
            reference.step(input).unwrap();
        }
        let before = serde_json::to_vec(&reference.world).unwrap();
        assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
        capture_frame(
            &session.assets.sprites.vram,
            &graphics,
            &output.join(format!("jp-custom-{player}.png")),
            |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                let custom = drawing.models.player.as_ref().unwrap();
                let actor = &session.world.actors[session.world.player].actor;
                let rig = &session.world.instances[session.world.player].rig;
                let mut scene = session.world.scene.clone();
                let matrices = rig
                    .joints
                    .iter()
                    .map(|&j| scene.world(j))
                    .collect::<tenchu_assets::Result<Vec<_>>>()?;
                let target = custom.pose(&matrices, scene.models[rig.joints[0]].local);
                let definition = session
                    .assets
                    .resources
                    .registry
                    .actor(actor.definition_kind())
                    .unwrap();
                let body = &session.assets.models[&definition.model];
                let mut plain_graphics = graphics.clone();
                plain_graphics.models.heroes = false;
                let plain = custom::Player::new(
                    "0000",
                    &pack,
                    &body.parts,
                    &body.vram,
                    &plain_graphics,
                    renderer,
                )?;
                assert_eq!(
                    plain.pose(&matrices, scene.models[rig.joints[0]].local),
                    target
                );
                let posed = |player: &custom::Player| {
                    player.appearance.meshes.pose_skin(&target, None);
                    let mut vertices: Vec<Vertex> = Vec::new();
                    for (part, &matrix) in player.appearance.parts.iter().zip(&target) {
                        player.appearance.meshes.visit(
                            &part.geometry,
                            matrix,
                            false,
                            0,
                            None,
                            |t| vertices.extend(t),
                        );
                    }
                    vertices
                };
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&posed(custom)),
                    bytemuck::cast_slice::<_, u8>(&posed(&plain)),
                    "hero refinement changed custom geometry"
                );
                let point = |p: Position| Vec3::from(p.to_array().map(|v| v as f32));
                for root in rig.weapons.iter().take(3).flat_map(|p| p.iter()) {
                    let parent = scene.models[*root].parent.unwrap();
                    let bone = rig.joints.iter().position(|&j| j == parent).unwrap();
                    let index = tenchu_assets::tenchu2::BONES
                        .iter()
                        .position(|&j| j == bone)
                        .unwrap();
                    let hand = custom::Player::attach(matrices[bone], bone, &matrices, &target);
                    assert!(
                        point(hand.translation).distance_squared(point(target[index].translation))
                            < 9.0
                    );
                    let weapon = scene.world(*root)?;
                    let moved = custom::Player::attach(weapon, bone, &matrices, &target);
                    let source_distance =
                        point(weapon.translation).distance(point(matrices[bone].translation));
                    let target_distance =
                        point(moved.translation).distance(point(target[index].translation));
                    assert!((source_distance - target_distance).abs() < 5.0);
                }
                let center =
                    Vec3::from(actor.position.to_array().map(|v| v as f32)) - Vec3::Y * 1100.0;
                let mut camera =
                    crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
                camera.focus(center, false);
                drawing.camera = Some(camera);
                drawing.show_hud = false;
                drawing.update(&session, renderer, None)?;
                assert!(drawing.models.player.is_some());
                let motion = drawing.motion.sample(1.0);
                assert!(
                    motion
                        .values()
                        .any(|p| p.offsets.iter().any(|v| v.length() > 0.0))
                );
                for part in 0..3 {
                    assert!(motion.contains_key(&interpolation::part(
                        rig.root,
                        rig.joints[tenchu_assets::tenchu2::BONES[part]],
                        actor.definition_kind(),
                    )));
                }
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            },
        )
        .unwrap();
        assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
        let reload = Pack::bodies(folder.to_str().unwrap()).unwrap();
        assert!(!Arc::ptr_eq(&reload, &pack));
        assert!(
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &reload,
                    &settings
                )
                .unwrap()
        );
        assert!(
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Arc::default(),
                    &settings
                )
                .is_err()
        );
        assert!(session.models.custom.is_some());
        assert!(
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &pack,
                    &Settings::default()
                )
                .unwrap()
        );
        assert!(session.models.custom.is_none());
        assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
    }
}

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; shared refinement preserves native/imported source state, palettes, joint motion and terrain"]
fn native_model_refinement_uses_shared_skinning_and_keeps_original_reversible() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut library = crate::visuals::appearance::library::Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = crate::tenchu2::native::models::donor(&gaisen).unwrap();
    for (sequence, body) in [
        (9, ""),
        (119, ""),
        (191, ""),
        (119, "disc:1"),
        (191, "tenchu2-jp:ayamea"),
    ] {
        let mut session = super::super::Session::load(&disc, sequence, 17).unwrap();
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &Default::default(),
                &crate::settings::Settings {
                    player_model: body.into(),
                    ..Default::default()
                },
            )
            .unwrap();
        for tick in 0..60 {
            let buttons = if tick < 10 {
                tenchu_core::tenchu2::input::UP
            } else if tick == 30 {
                tenchu_core::tenchu2::input::ATTACK
            } else {
                0
            };
            session.step(buttons).unwrap();
        }
        let before = postcard::to_allocvec(&session.world).unwrap();
        let actor = &session.world.actors[session.world.player].actor;
        let center = Vec3::from(actor.position.to_array().map(|v| v as f32)) - Vec3::Y * 1100.0;
        let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
        camera.focus(center, false);
        let mut results: Vec<Geometry> = Vec::new();
        for mode in 0..6 {
            let mut settings = crate::settings::Settings::default();
            settings.graphics.fog = false;
            settings.graphics.smooth_models = (1..4).contains(&mode);
            settings.graphics.model_detail = if mode == 2 { 2 } else { 0 };
            settings.graphics.models.characters = if mode == 3 { 2 } else { 0 };
            settings.graphics.models.heroes = mode == 4;
            settings.graphics.lighting.directional.intensity = 60;
            let graphics = super::super::preferences::graphics(&settings);
            capture_frame(
                &session.assets.sprites.vram,
                &graphics,
                &output.join(format!(
                    "jp-refinement-s{sequence}-{}-{mode}.png",
                    body.replace(':', "-")
                )),
                |renderer| {
                    let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                    let geometry = drawing.models.geometry(
                        &session.world,
                        &session.assets,
                        &session.frame,
                        &graphics,
                        Presentation {
                            camera: Some(camera),
                            ..Default::default()
                        },
                    )?;
                    assert!(
                        geometry
                            .actors
                            .expanded()
                            .iter()
                            .all(|v| Vec3::from(v.position).is_finite()
                                && Vec3::from(v.normal).is_finite())
                    );
                    results.push(geometry);
                    drawing.camera = Some(camera);
                    drawing.show_hud = false;
                    drawing.update(&session, renderer, None)?;
                    Ok((drawing.scene, camera.source_view(), Vec::new()))
                },
            )
            .unwrap();
        }
        let baseline = &results[0];
        assert_eq!(
            baseline.actors.expanded().len(),
            results[1].actors.expanded().len()
        );
        assert!(
            baseline
                .actors
                .expanded()
                .iter()
                .zip(&results[1].actors.expanded())
                .any(|(a, b)| a.normal != b.normal)
        );
        for refined in &results[2..5] {
            assert!(refined.actors.expanded().len() > baseline.actors.expanded().len());
            assert_eq!(refined.terrain.len(), baseline.terrain.len());
            assert!(
                refined
                    .terrain
                    .iter()
                    .zip(&baseline.terrain)
                    .all(|(a, b)| a.position == b.position)
            );
            let bounds = |vertices: &[Vertex]| {
                vertices.iter().fold(
                    (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                    |(lo, hi), v| {
                        (
                            lo.min(Vec3::from(v.position)),
                            hi.max(Vec3::from(v.position)),
                        )
                    },
                )
            };
            let (a, b) = bounds(&baseline.actors.expanded());
            let (c, d) = bounds(&refined.actors.expanded());
            assert!((a - c).length() < 1000.0 && (b - d).length() < 1000.0);
        }
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&baseline.actors.expanded()),
            bytemuck::cast_slice::<_, u8>(&results[5].actors.expanded())
        );
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; verifies shared native impact sparks, lights, pause and original restoration"]
fn native_modern_sparks_preserve_source_effects_and_restore_original() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    let position = session.world.actors[session.world.player]
        .actor
        .position
        .wrapping_add(Position::new(900, -1300, 0));
    let center = Vec3::from(position.to_array().map(|v| v as f32));
    camera.position = center - camera.forward() * 3500.0;
    session.world.effects.flash(position, false).unwrap();
    session
        .world
        .effects
        .impact(
            position,
            Position::new(1, 0, 0),
            &session.assets.resources.math,
            &session.assets.effect_rules,
            &mut session.world.random,
        )
        .unwrap();
    session
        .world
        .effects
        .fuse(
            position,
            &session.assets.effect_rules,
            &mut session.world.random,
        )
        .unwrap();
    session.step(0).unwrap();
    assert_eq!(session.frame.effects.contacts.len(), 2);
    assert!(
        session
            .frame
            .effects
            .points
            .iter()
            .any(|p| p.contact.is_none())
    );
    let state = postcard::to_allocvec(&session.world).unwrap();
    let mut images = Vec::new();
    for mode in ["original", "modern", "unlit", "zero", "restored"] {
        let modern = mode != "original";
        let mut graphics = Graphics {
            asset_pack: false,
            fog: false,
            ..Default::default()
        };
        graphics.effects.sparks = modern;
        graphics.effects.spark_density = if mode == "zero" { 0 } else { 100 };
        graphics.lighting.enabled = mode != "unlit";
        graphics.lighting.fires = false;
        graphics.lighting.shadows = false;
        let path = output.join(format!("jp-sparks-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            if modern {
                let effects =
                    postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().contact)
                        .unwrap();
                drawing.update(&session, renderer, None)?;
                assert_eq!(
                    postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().contact)
                        .unwrap(),
                    effects,
                    "a paused redraw must not replay impacts"
                );
                assert_eq!(drawing.scene.lighting.transients.is_empty(), mode == "zero");
                assert_eq!(
                    drawing
                        .scene
                        .lighting
                        .sources(&graphics.lighting, camera.position)
                        .is_empty(),
                    matches!(mode, "zero" | "unlit")
                );
                let before = drawing.scene.lighting.transients.len();
                assert!(before <= 4, "moving sparks need bounded light clusters");
                if mode == "restored" {
                    let mut original = graphics.clone();
                    original.effects.sparks = false;
                    renderer.configure(&original);
                    drawing.update(&session, renderer, None)?;
                    assert!(drawing.scene.lighting.transients.is_empty());
                }
            }
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    assert_ne!(
        images[0], images[1],
        "modern geometry did not change the source sprites"
    );
    assert_ne!(
        images[1], images[2],
        "sparks did not illuminate their surroundings"
    );
    assert_ne!(
        images[1], images[3],
        "zero density failed to suppress modern bursts"
    );
    assert_eq!(
        images[0], images[4],
        "returning to Original must recover the exact source drawing"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC and GPU; verifies native impact history under skipped renders, setting rebuilds and expiry"]
fn native_sparks_survive_skipped_renders_without_repeating_impacts() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        [640, 360],
        &session.assets.sprites.vram,
    ))
    .unwrap();
    let mut graphics = Graphics {
        asset_pack: false,
        ..Default::default()
    };
    graphics.effects.sparks = true;
    renderer.configure(&graphics);
    let mut frequent = Drawing::new(&session, &graphics, &mut renderer).unwrap();
    frequent.update(&session, &mut renderer, None).unwrap();
    let mut sparse = Drawing::new(&session, &graphics, &mut renderer).unwrap();
    sparse.update(&session, &mut renderer, None).unwrap();
    for frame in 0..16 {
        if matches!(frame, 0 | 7 | 13) {
            let position = session.world.actors[session.world.player]
                .actor
                .position
                .wrapping_add(Position::new(0, -1400, 0));
            session.world.effects.flash(position, frame != 7).unwrap();
        }
        session.step(0).unwrap();
        let state = postcard::to_allocvec(&session.world).unwrap();
        frequent.update(&session, &mut renderer, None).unwrap();
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
    }
    sparse.update(&session, &mut renderer, None).unwrap();
    let contact = |drawing: &Drawing| {
        postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().contact).unwrap()
    };
    assert_eq!(contact(&frequent), contact(&sparse));
    assert!(!sparse.scene.lighting.transients.is_empty());
    let before = contact(&sparse);
    let mut rebuilt = Drawing::new(&session, &graphics, &mut renderer).unwrap();
    rebuilt.preserve_environment(&mut sparse);
    rebuilt.update(&session, &mut renderer, None).unwrap();
    assert_eq!(
        contact(&rebuilt),
        before,
        "model/artwork rebuild restarted sparks"
    );
    for _ in 0..90 {
        session.step(0).unwrap();
    }
    rebuilt.update(&session, &mut renderer, None).unwrap();
    assert!(
        rebuilt.scene.lighting.transients.is_empty(),
        "expired pre-fast-forward impacts returned"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; compares shared water contacts, delayed presentation and original restoration"]
fn native_modern_splashes_replace_only_water_particles() {
    use tenchu_core::tenchu2::effects::{Contact, ContactKind};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let layout = tenchu_assets::tenchu2::layout::Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| l.map == 16)
        .unwrap();
    let mut session = super::super::Session::load(&disc, layout.sequence, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let mut settings = Graphics {
        asset_pack: false,
        fog: false,
        ..Default::default()
    };
    settings.water.enabled = true;
    settings.water.spray = 0;
    let center = nearest_water(&session, &settings);
    let position = center.as_ivec3().to_array().into();
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    camera.pitch = -1.1;
    camera.position = center - camera.forward() * 4500.0;
    session.world.camera.view = camera.source_view();
    session
        .world
        .effects
        .splash(
            position,
            1024,
            &session.assets.resources.math,
            &mut session.world.random,
        )
        .unwrap();
    session.world.effects.ripple(position, 40).unwrap();
    session
        .world
        .effects
        .fuse(
            position,
            &session.assets.effect_rules,
            &mut session.world.random,
        )
        .unwrap();
    for tick in 0..5 {
        session.world.tick += 1;
        session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        session
            .contact
            .record(u64::from(session.world.tick), &session.frame.effects);
        assert_eq!(
            session
                .frame
                .effects
                .contacts
                .iter()
                .filter(|e| matches!(e, Contact::Splash { .. }))
                .count(),
            usize::from(tick == 0)
        );
    }
    assert!(
        session
            .frame
            .effects
            .sprites
            .iter()
            .any(|s| s.contact == Some(ContactKind::Water))
    );
    assert!(
        session
            .frame
            .effects
            .points
            .iter()
            .any(|p| p.contact.is_none())
    );
    let state = postcard::to_allocvec(&session.world).unwrap();
    let mut images = Vec::new();
    for mode in ["original", "modern", "restored"] {
        let mut graphics = settings.clone();
        graphics.effects.splashes = mode != "original";
        let path = output.join(format!("jp-splash-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            if mode != "original" {
                let contact = &drawing.scene.environment.as_ref().unwrap().contact;
                assert!(
                    !contact
                        .geometry(camera.matrix(16.0 / 9.0), 1.0, &graphics.effects)
                        .vertices
                        .is_empty(),
                    "delayed first presentation lost the water contact"
                );
                let particles = postcard::to_allocvec(contact).unwrap();
                drawing.update(&session, renderer, None)?;
                assert_eq!(
                    postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().contact)
                        .unwrap(),
                    particles,
                    "pause repeated water contacts"
                );
            }
            if mode == "restored" {
                let mut original = graphics.clone();
                original.effects.splashes = false;
                renderer.configure(&original);
                drawing.update(&session, renderer, None)?;
            }
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    assert_ne!(images[0], images[1], "modern water contacts were invisible");
    assert_eq!(
        images[0], images[2],
        "disabling water contacts changed unrelated particles"
    );
}

fn nearest_water(session: &super::super::Session, graphics: &Graphics) -> Vec3 {
    let meshes = prepare(
        &session.assets.level.vram,
        sources(
            &format!("map{}/terrain", session.world.map),
            &session.assets.level.map,
            Role::Terrain,
        ),
        graphics,
    )
    .unwrap();
    let mut centers = Vec::new();
    for part in &session.assets.level.map {
        meshes.visit(
            &part.geometry,
            Matrix {
                translation: part.position.map(|v| i32::from(v) * 10).into(),
                ..Default::default()
            },
            false,
            0,
            None,
            |triangle| {
                if triangle[0].flags & (render_mesh::WATER | render_mesh::WATERFALL)
                    == render_mesh::WATER
                    && triangle
                        .iter()
                        .all(|v| (v.position[1] - triangle[0].position[1]).abs() < 10.0)
                {
                    centers.push(
                        triangle
                            .iter()
                            .map(|v| Vec3::from(v.position))
                            .sum::<Vec3>()
                            / 3.0,
                    );
                }
            },
        );
    }
    let player = Vec3::from(
        session.world.actors[0]
            .actor
            .position
            .to_array()
            .map(|v| v as f32),
    );
    centers
        .into_iter()
        .min_by(|a, b| {
            a.distance_squared(player)
                .total_cmp(&b.distance_squared(player))
        })
        .unwrap()
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; verifies detailed native hook/rope follows the source path and restores exactly"]
fn native_detailed_grapple_preserves_source_path_and_restores_original() {
    use tenchu_core::tenchu2::items::{State, aim::Aim, grapple::Grapple};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let owner = session.world.player;
    let hand = session
        .world
        .scene
        .world(session.world.instances[owner].rig.joints[11])
        .unwrap()
        .translation;
    let direction = Position::new(1200, -2500, -4000);
    let target = hand.wrapping_add(direction);
    let mut aim = Aim {
        direction,
        target,
        reach: Some(5000),
        distance: 5000,
        phase: 1,
    };
    let mut hook = Grapple::launch(
        owner,
        hand,
        direction,
        &mut aim,
        &session.assets.resources.math,
    )
    .unwrap();
    for _ in 0..10 {
        hook.advance(target, hand, &session.assets.resources.math);
    }
    assert!(hook.rope.len() > 2);
    let points: Vec<_> = std::iter::once(hand)
        .chain(hook.rope.iter().copied())
        .map(|p| Vec3::from(p.to_array().map(|v| v as f32)))
        .collect();
    let rope = crate::visuals::contact::rope::path(&points);
    assert!(!rope.vertices.is_empty() && rope.vertices.len() <= 64 * 48);
    for vertex in &rope.vertices {
        let p = Vec3::from(vertex.position);
        assert!(p.is_finite() && Vec3::from(vertex.normal).is_finite());
        assert!(
            points.iter().any(|point| p.distance(*point) <= 16.1),
            "rope moved away from source path"
        );
    }
    session
        .world
        .items
        .insert(
            State::Grapple(hook),
            &mut session.world.scene,
            &mut session.world.collision,
        )
        .unwrap();
    let state = postcard::to_allocvec(&session.world).unwrap();
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    let center = Vec3::from(
        hand.wrapping_add(direction.map(|v| v / 2))
            .to_array()
            .map(|v| v as f32),
    );
    camera.position = center - camera.forward() * 8000.0;
    let mut images = Vec::new();
    let mut hooks = Vec::new();
    for mode in ["original", "detailed", "restored"] {
        let mut graphics = Graphics {
            asset_pack: false,
            fog: false,
            ..Default::default()
        };
        graphics.models.grapple = mode != "original";
        let path = output.join(format!("jp-grapple-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            let mut vertices = 0;
            for part in session.assets.items.model(0).unwrap() {
                drawing.models.items.visit(
                    &part.geometry,
                    Matrix::default(),
                    false,
                    0,
                    None,
                    |_| vertices += 3,
                );
            }
            hooks.push(vertices);
            if mode == "restored" {
                let mut original = graphics.clone();
                original.models.grapple = false;
                renderer.configure(&original);
                let mut restored = Drawing::new(&session, &original, renderer)?;
                restored.preserve_environment(&mut drawing);
                restored.camera = Some(camera);
                restored.show_hud = false;
                restored.update(&session, renderer, None)?;
                drawing = restored;
            }
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    assert!(
        hooks[1] > hooks[0],
        "detailed hook did not refine source geometry"
    );
    assert_ne!(images[0], images[1]);
    assert_eq!(images[0], images[2]);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; verifies source blood styling, pause, interpolation and original restoration"]
fn native_blood_hit_styles_preserve_source_particles_and_ground_effects() {
    use crate::settings::BloodStyle;
    use tenchu_core::tenchu2::effects::blood::Spurt;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let position = session.world.actors[session.world.player]
        .actor
        .position
        .wrapping_add(Position::new(500, -1200, 0));
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    camera.position = Vec3::from(position.to_array().map(|v| v as f32)) - camera.forward() * 3500.0;
    session
        .world
        .effects
        .blood(
            Spurt {
                position,
                yaw: 1024,
                speed: 100,
                gravity: 20,
            },
            &session.assets.resources.math,
            &session.assets.effect_rules,
            &mut session.world.random,
        )
        .unwrap();
    session
        .world
        .effects
        .death_pool(
            position,
            &session.assets.level.collision,
            &mut Default::default(),
        )
        .unwrap();
    for _ in 0..4 {
        session.step(0).unwrap();
    }
    assert!(
        session
            .frame
            .effects
            .sprites
            .iter()
            .any(|s| s.blood.is_some())
    );
    assert!(
        session
            .frame
            .effects
            .points
            .iter()
            .any(|p| p.blood.is_some())
    );
    assert!(!session.frame.effects.quads.is_empty());
    let state = postcard::to_allocvec(&session.world).unwrap();
    let mut images = Vec::new();
    for (mode, style) in [
        ("original", BloodStyle::Original),
        ("hd", BloodStyle::OriginalHd),
        ("modern", BloodStyle::Modern),
        ("off", BloodStyle::Off),
        ("restored", BloodStyle::OriginalHd),
    ] {
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.blood.hit = style;
        let path = output.join(format!("jp-blood-hit-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            let presentation = || Presentation {
                camera: Some(camera),
                ..Default::default()
            };
            let before = drawing.models.geometry(
                &session.world,
                &session.assets,
                &session.frame,
                &graphics,
                presentation(),
            )?;
            let paused = drawing.models.geometry(
                &session.world,
                &session.assets,
                &session.frame,
                &graphics,
                presentation(),
            )?;
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&before.effects.vertices),
                bytemuck::cast_slice::<_, u8>(&paused.effects.vertices)
            );
            let mut hidden = graphics.clone();
            hidden.blood.hit = BloodStyle::Off;
            let without_hit = drawing.models.geometry(
                &session.world,
                &session.assets,
                &session.frame,
                &hidden,
                presentation(),
            )?;
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&before.terrain),
                bytemuck::cast_slice::<_, u8>(&without_hit.terrain)
            );
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&before.actors.expanded()),
                bytemuck::cast_slice::<_, u8>(&without_hit.actors.expanded())
            );
            // All source surface blood survives independently of the airborne style.
            let floor = |geometry: &Geometry| {
                geometry
                    .effects
                    .vertices
                    .iter()
                    .filter(|v| v.flags & render_mesh::SCREEN == 0)
                    .copied()
                    .collect::<Vec<_>>()
            };
            if style.uses_original() {
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&floor(&before)),
                    bytemuck::cast_slice::<_, u8>(&floor(&without_hit))
                );
            }
            if style == BloodStyle::Modern {
                let interpolated = drawing.models.geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation {
                        alpha: 0.25,
                        ..presentation()
                    },
                )?;
                assert_ne!(
                    bytemuck::cast_slice::<_, u8>(&before.effects.vertices),
                    bytemuck::cast_slice::<_, u8>(&interpolated.effects.vertices)
                );
                assert!(before.effects.vertices.len() > without_hit.effects.vertices.len());
                assert!(
                    before
                        .effects
                        .vertices
                        .iter()
                        .all(|v| Vec3::from(v.position).is_finite())
                );
            }
            if mode == "restored" {
                let mut original = graphics.clone();
                original.blood.hit = BloodStyle::Original;
                renderer.configure(&original);
                drawing.update(&session, renderer, None)?;
            }
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    for i in 0..4 {
        for j in i + 1..4 {
            assert_ne!(
                images[i], images[j],
                "blood styles {i}/{j} are visually identical"
            );
        }
    }
    assert_eq!(
        images[0], images[4],
        "restoring Original changed source drawing"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; source ground blood survives skipped rendering, rebuilds and clearing through shared cached decals"]
fn native_retained_blood_uses_source_contacts_and_cached_surface_projection() {
    use crate::settings::BloodStyle;
    use tenchu_core::tenchu2::effects::blood::Spurt;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let position = session.world.actors[session.world.player]
        .actor
        .position
        .wrapping_add(Position::new(1600, -900, 0));
    session.world.effects = Default::default();
    for yaw in [0, 1024, 2048, 3072] {
        session
            .world
            .effects
            .blood(
                Spurt {
                    position,
                    yaw,
                    speed: 100,
                    gravity: 20,
                },
                &session.assets.resources.math,
                &session.assets.effect_rules,
                &mut session.world.random,
            )
            .unwrap();
    }
    for _ in 0..240 {
        session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        session.world.tick += 1;
        session
            .contact
            .record(u64::from(session.world.tick), &session.frame.effects);
    }
    assert!(session.world.effects.entries.is_empty());
    assert!(
        session.contact.blood.latest() >= 2,
        "source droplets did not hit dry ground"
    );
    let state = postcard::to_allocvec(&session.world).unwrap();
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    camera.pitch = -0.6;
    let center = Vec3::from(position.to_array().map(|v| v as f32)) + Vec3::Y * 700.0;
    camera.position = center - camera.forward() * 3500.0;
    let mut images = Vec::new();
    for (mode, style, retain, conform) in [
        ("expired", BloodStyle::Original, false, true),
        ("modern", BloodStyle::Modern, false, true),
        ("original", BloodStyle::Original, true, true),
        ("hd", BloodStyle::OriginalHd, true, true),
        ("raw", BloodStyle::Original, true, false),
        ("cleared", BloodStyle::Modern, false, true),
        ("restored", BloodStyle::Modern, false, true),
    ] {
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.blood.ground = style;
        graphics.blood.retain_original = retain;
        graphics.blood.conform_retained = conform;
        let path = output.join(format!("jp-ground-blood-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            if graphics.blood.retained_ground() {
                let environment = drawing.scene.environment.as_ref().unwrap();
                let pages: Vec<_> = environment.blood.pages().cloned().collect();
                assert!(!pages.is_empty(), "retained blood missing: {mode}");
                assert!(
                    pages
                        .iter()
                        .flat_map(|p| p.iter())
                        .all(|v| Vec3::from(v.position).is_finite())
                );
                assert!(
                    pages.iter().flat_map(|p| p.iter()).all(
                        |v| (v.flags & render_mesh::SOFT != 0) == (style == BloodStyle::Modern)
                    )
                );
                assert!(
                    pages
                        .iter()
                        .flat_map(|p| p.iter())
                        .all(|v| render_mesh::original_hd_blood(v.flags)
                            == (style == BloodStyle::OriginalHd))
                );
                if conform {
                    for v in pages.iter().flat_map(|p| p.iter()).step_by(3) {
                        let p = Vec3::from(v.position);
                        let hit = environment
                            .surfaces
                            .cast(p - Vec3::Y * 5.0, p + Vec3::Y * 5.0)
                            .expect("ground mark floats away from visible terrain");
                        assert!(p.distance(hit.position) < 1.0);
                    }
                }
                let stored = postcard::to_allocvec(&environment.blood).unwrap();
                assert_eq!(
                    renderer.set_projected_decals(environment.blood.projected_pages()),
                    0,
                    "unchanged stains were uploaded again"
                );
                drawing.update(&session, renderer, None)?;
                let mut rebuilt = Drawing::new(&session, &graphics, renderer)?;
                rebuilt.camera = Some(camera);
                rebuilt.show_hud = false;
                rebuilt.preserve_environment(&mut drawing);
                rebuilt.update(&session, renderer, None)?;
                let blood = &rebuilt.scene.environment.as_ref().unwrap().blood;
                assert_eq!(
                    postcard::to_allocvec(blood).unwrap(),
                    stored,
                    "paused redraw/rebuild advanced or cleared stains"
                );
                assert_eq!(renderer.set_projected_decals(blood.projected_pages()), 0);
                drawing = rebuilt;
                if mode == "cleared" {
                    drawing.clear_blood();
                    drawing.update(&session, renderer, None)?;
                    assert!(
                        drawing
                            .scene
                            .environment
                            .as_ref()
                            .unwrap()
                            .blood
                            .pages()
                            .next()
                            .is_none()
                    );
                } else if mode == "restored" {
                    let mut original = graphics.clone();
                    original.blood = Default::default();
                    renderer.configure(&original);
                    drawing.update(&session, renderer, None)?;
                }
            }
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    for i in 1..=4 {
        assert_ne!(
            images[0], images[i],
            "style {i} produced no visible retained blood"
        );
    }
    assert_ne!(
        images[2], images[3],
        "Original HD failed to smooth retained artwork"
    );
    assert_eq!(images[0], images[5], "clear replayed old contacts");
    assert_eq!(
        images[0], images[6],
        "restoring source style left retained blood"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; source death-pool growth, styling, retention and paused restoration"]
fn native_death_pools_share_styles_and_keep_source_timing() {
    use crate::settings::BloodStyle;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let position = session.world.actors[session.world.player]
        .actor
        .position
        .wrapping_add(Position::new(1200, -500, 0));
    session.world.effects = Default::default();
    session
        .world
        .effects
        .death_pool(
            position,
            &session.assets.level.collision,
            &mut Default::default(),
        )
        .unwrap();
    let advance = |session: &mut super::super::Session| {
        session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        session.world.tick += 1;
        session
            .contact
            .record(u64::from(session.world.tick), &session.frame.effects);
    };
    for _ in 0..50 {
        advance(&mut session);
    }
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    camera.pitch = -0.6;
    camera.position = Vec3::from(position.to_array().map(|v| v as f32)) + Vec3::Y * 500.0
        - camera.forward() * 3200.0;
    let mut images = Vec::new();
    for (mode, style, size) in [
        ("original", BloodStyle::Original, 100),
        ("hd", BloodStyle::OriginalHd, 100),
        ("modern", BloodStyle::Modern, 100),
        ("off", BloodStyle::Off, 100),
        ("half", BloodStyle::Original, 50),
        ("restored", BloodStyle::Modern, 100),
    ] {
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.blood.death = style;
        graphics.blood.death_size = size;
        let state = postcard::to_allocvec(&session.world).unwrap();
        let path = output.join(format!("jp-death-pool-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            if style == BloodStyle::Modern {
                let blood = &drawing.scene.environment.as_ref().unwrap().blood;
                let pool = blood.pool_geometry();
                assert!(!pool.vertices.is_empty(), "modern death pool missing");
                assert!(
                    pool.vertices
                        .iter()
                        .all(|v| v.flags & render_mesh::SOFT != 0)
                );
                let saved = postcard::to_allocvec(blood).unwrap();
                drawing.update(&session, renderer, None)?;
                assert_eq!(
                    postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().blood)
                        .unwrap(),
                    saved
                );
                // Restore source drawing while paused, then enable the already-grown pool again.
                let mut source = graphics.clone();
                source.blood.death = BloodStyle::Original;
                renderer.configure(&source);
                drawing.update(&session, renderer, None)?;
                assert!(
                    drawing
                        .scene
                        .environment
                        .as_ref()
                        .unwrap()
                        .blood
                        .pool_geometry()
                        .vertices
                        .is_empty()
                );
                if mode != "restored" {
                    renderer.configure(&graphics);
                    drawing.update(&session, renderer, None)?;
                    assert!(
                        !drawing
                            .scene
                            .environment
                            .as_ref()
                            .unwrap()
                            .blood
                            .pool_geometry()
                            .vertices
                            .is_empty()
                    );
                }
            }
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    for i in 0..5 {
        for j in i + 1..5 {
            assert!(
                images[i] != images[j],
                "death-pool modes {i}/{j} look identical"
            );
        }
    }
    assert!(
        images[0] == images[5],
        "restoring Original changed its source pool"
    );
    for _ in 0..300 {
        advance(&mut session);
    }
    assert!(session.world.effects.entries.is_empty());
    for style in [
        BloodStyle::Modern,
        BloodStyle::Original,
        BloodStyle::OriginalHd,
    ] {
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.blood.death = style;
        graphics.blood.retain_original = true;
        let path = output.join(format!("jp-death-pool-retained-{style:?}.png"));
        let state = postcard::to_allocvec(&session.world).unwrap();
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            let environment = drawing.scene.environment.as_ref().unwrap();
            let vertices: Vec<_> = environment.blood.pages().flat_map(|p| p.iter()).collect();
            assert!(
                !vertices.is_empty(),
                "expired pool was not retained: {style:?}"
            );
            assert!(
                vertices
                    .iter()
                    .all(|v| render_mesh::original_hd_blood(v.flags)
                        == (style == BloodStyle::OriginalHd))
            );
            for v in vertices.iter().step_by(3) {
                let p = Vec3::from(v.position);
                let hit = environment
                    .surfaces
                    .cast(p - Vec3::Y * 5.0, p + Vec3::Y * 5.0)
                    .unwrap();
                assert!(
                    p.distance(hit.position) < 1.0,
                    "pool floats above the floor"
                );
            }
            assert_eq!(
                renderer.set_projected_decals(environment.blood.projected_pages()),
                0
            );
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; optional player pools, selected artwork, live toggle, rebuild/cache and clear"]
fn native_player_pool_uses_shared_death_styles_and_completed_pose() {
    use crate::settings::BloodStyle;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = super::super::tests::player_corpse(&disc, 119, true);
    session.frame.fade = None;
    session.frame.transition_fade = None;
    let corpse = session.contact.player_corpse.unwrap();
    let center = Vec3::from(corpse.position.to_array().map(|v| v as f32));
    let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
    camera.pitch = -0.8;
    camera.position = center - camera.forward() * 3200.0;
    let state = postcard::to_allocvec(&session.world).unwrap();
    let mut images = Vec::new();
    for (mode, style, enabled) in [
        ("off", BloodStyle::Original, false),
        ("original", BloodStyle::Original, true),
        ("hd", BloodStyle::OriginalHd, true),
        ("modern", BloodStyle::Modern, true),
        ("restored", BloodStyle::Modern, true),
        ("cleared", BloodStyle::Modern, true),
    ] {
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.blood.death = style;
        graphics.blood.player_death = enabled;
        let path = output.join(format!("jp-player-pool-{mode}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            if enabled {
                let environment = drawing.scene.environment.as_ref().unwrap();
                let blood = &environment.blood;
                assert!(
                    blood.pages().next().is_some() || !blood.pool_geometry().vertices.is_empty()
                );
                let saved = postcard::to_allocvec(blood).unwrap();
                drawing.update(&session, renderer, None)?;
                let mut rebuilt = Drawing::new(&session, &graphics, renderer)?;
                rebuilt.camera = Some(camera);
                rebuilt.show_hud = false;
                rebuilt.preserve_environment(&mut drawing);
                rebuilt.update(&session, renderer, None)?;
                assert!(
                    postcard::to_allocvec(&rebuilt.scene.environment.as_ref().unwrap().blood)
                        .unwrap()
                        == saved,
                    "paused/rebuilt player pool changed"
                );
                drawing = rebuilt;
                if mode == "restored" {
                    let mut original = graphics.clone();
                    original.blood = Default::default();
                    renderer.configure(&original);
                    drawing.update(&session, renderer, None)?;
                } else if mode == "cleared" {
                    drawing.clear_blood();
                    drawing.update(&session, renderer, None)?;
                    assert!(
                        drawing
                            .scene
                            .environment
                            .as_ref()
                            .unwrap()
                            .blood
                            .pages()
                            .next()
                            .is_none()
                    );
                }
            }
            assert!(postcard::to_allocvec(&session.world).unwrap() == state);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
    }
    for i in 0..4 {
        for j in i + 1..4 {
            assert!(
                images[i] != images[j],
                "player blood styles {i}/{j} have no visible difference"
            );
        }
    }
    assert!(
        images[0] == images[4],
        "player toggle failed to restore Original"
    );
    assert!(
        images[0] == images[5],
        "clearing recreated the player's pool"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; verifies native lantern emission, shadows and exact restoration"]
fn native_lamps_use_shared_lighting_without_changing_source_state() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    for map in [1, 12, 17] {
        let sequence = layouts
            .iter()
            .find(|layout| layout.map == map)
            .unwrap()
            .sequence;
        let mut session = super::super::Session::load(&disc, sequence, 1).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let original = postcard::to_allocvec(&session.world).unwrap();
        let sources = lamps::defaults(&session.assets.level, &session.assets.resources.math);
        let lamp = &sources[0];
        let target = lamp.light.position;
        let position = target + Vec3::new(3500.0, -1800.0, 6000.0);
        let direction = (target + Vec3::Y * 800.0 - position).normalize();
        let camera = crate::debug::camera::Camera {
            position,
            yaw: direction.x.atan2(direction.z),
            pitch: (-direction.y).asin(),
            fov: 43.6,
        };
        let mut images = Vec::new();
        for mode in ["off", "lit", "shadows", "restored"] {
            let mut graphics = Graphics {
                fog: false,
                asset_pack: false,
                ..Default::default()
            };
            graphics.lighting.enabled = true;
            graphics.lighting.fires = false;
            graphics.lighting.lamps = matches!(mode, "lit" | "shadows");
            graphics.lighting.shadows = mode == "shadows";
            graphics.lighting.count = 8;
            graphics.lighting.shadow_count = 4;
            let path = output.join(format!("jp-lamps-map{map}-{mode}.png"));
            capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                drawing.camera = Some(camera);
                drawing.show_hud = false;
                drawing.update(&session, renderer, None)?;
                let lights = drawing
                    .scene
                    .lighting
                    .named_sources(&graphics.lighting, camera.position);
                if graphics.lighting.lamps {
                    assert_eq!(lights.len(), sources.len());
                    if graphics.lighting.shadows {
                        assert!(drawing.scene.environment.is_some());
                    }
                } else {
                    assert!(lights.is_empty());
                }
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            })
            .unwrap();
            images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
            assert!(postcard::to_allocvec(&session.world).unwrap() == original);
        }
        let changed = |a: usize, b: usize| {
            images[a]
                .rgba
                .chunks_exact(4)
                .zip(images[b].rgba.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count()
        };
        assert_eq!(changed(0, 3), 0, "lamp toggle did not restore map {map}");
        assert!(changed(0, 1) > 100, "lamps did not light map {map}");
        assert!(
            changed(1, 2) > 100,
            "lamps did not cast shadows in map {map}"
        );
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; dormant doors draw without activation and join moving shadow casters"]
fn native_distant_scenery_renders_without_waking_source_objects() {
    distant_scenery_fixture(true);
    distant_scenery_fixture(false);
}

fn distant_scenery_fixture(training: bool) {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap())
        .join(if training { "signs" } else { "doors" });
    std::fs::create_dir_all(&output).unwrap();
    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    let maps: &[usize] = if training { &[0] } else { &[1, 4, 12, 15] };
    let (session, door) = maps.iter().copied()
        .find_map(|map| {
            let sequence = layouts.iter().find(|l| l.map == map).unwrap().sequence;
            let mut session = super::super::Session::load(&disc, sequence, 1).unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let door = scenery::distant_objects(
                &session.world,
                &session.assets.level,
                &session.frame.scenery,
                &session.assets.resources.math,
            )
            .find(|model| {
                matches!(model.source, tenchu_core::tenchu2::environment::scenery::Source::Object(index)
                    if if training {session.assets.level.scene.dynamic[index].model==Some(18)} else {session.assets.level.scene.dynamic[index].kind <= 2})
            });
            door.map(|door| (session, door))
        })
        .expect("fixture with a dormant door");
    eprintln!("distant door: map {}, {:?}", session.assets.level.id, door);
    let matrix = Matrix {
        translation: door.position,
        rotation: session.assets.resources.math.rotate_yxz(door.rotation),
    };
    let vertices: Vec<_> = session.assets.level.objects[door.model]
        .geometry
        .iter()
        .flat_map(|g| &g.vertices)
        .collect();
    let min: [i16; 3] = std::array::from_fn(|axis| vertices.iter().map(|v| v[axis]).min().unwrap());
    let max: [i16; 3] = std::array::from_fn(|axis| vertices.iter().map(|v| v[axis]).max().unwrap());
    let center: [i16; 3] =
        std::array::from_fn(|axis| ((i32::from(min[axis]) + i32::from(max[axis])) / 2) as i16);
    let mut eye = center;
    eye[if max[0] - min[0] < max[2] - min[2] {
        0
    } else {
        2
    }] += 5000;
    eye[1] -= 500;
    let target = Vec3::from(matrix.transform(center).map(|v| v as f32).to_array());
    let position = Vec3::from(matrix.transform(eye).map(|v| v as f32).to_array());
    let direction = (target - position).normalize();
    let camera = crate::debug::camera::Camera {
        position,
        yaw: direction.x.atan2(direction.z),
        pitch: (-direction.y).asin(),
        fov: 43.6,
    };
    let before = postcard::to_allocvec(&session.world).unwrap();
    let mut images = Vec::new();
    let mut prop_counts = Vec::new();
    for enabled in [false, true, false] {
        let graphics = Graphics {
            fog: false,
            distant_doors: enabled,
            asset_pack: false,
            ..Default::default()
        };
        let path = output.join(format!("jp-distant-door-{}.png", images.len()));
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
            prop_counts.push(geometry.props.len());
            assert!(
                geometry
                    .props
                    .iter()
                    .all(|v| v.flags & render_mesh::SCENERY != 0)
            );
            Ok((
                SceneDrawing::new(geometry, renderer, &session.assets.level),
                camera.source_view(),
                Vec::new(),
            ))
        })
        .unwrap();
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
        assert!(postcard::to_allocvec(&session.world).unwrap() == before);
    }
    assert!(prop_counts[1] > prop_counts[0]);
    assert!(
        images[0] != images[1],
        "distant door is not visible in the fixture camera"
    );
    assert!(
        images[0] == images[2],
        "door toggle did not restore original draw list"
    );
    images.clear();
    for shadows in [false, true] {
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.lighting.enabled = true;
        graphics.lighting.fires = false;
        graphics.lighting.lamps = false;
        graphics.lighting.shadows = shadows;
        graphics.lighting.ambient = 10;
        let path = output.join(format!("jp-prop-shadows-{shadows}.png"));
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
            let mut scene = SceneDrawing::new(geometry, renderer, &session.assets.level);
            scene.lighting.debug_light =
                Some(target - (position - target) * 0.6 - Vec3::Y * 1500.0);
            let mut environment = environment::Environment::new(
                &session.assets.level,
                &models.scenery,
                &session.assets.resources.math,
                renderer,
            );
            // Isolate the moving-prop caster from terrain and actor shadows.
            environment.caster = renderer.upload_shadow(&[]);
            scene.actors = renderer.upload(&[]);
            scene.environment = Some(environment);
            Ok((scene, camera.source_view(), Vec::new()))
        })
        .unwrap();
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
    }
    let changed = images[0]
        .rgba
        .chunks_exact(4)
        .zip(images[1].rgba.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed > 100,
        "moving props did not cast shadows ({changed} pixels)"
    );
    assert!(postcard::to_allocvec(&session.world).unwrap() == before);
}
