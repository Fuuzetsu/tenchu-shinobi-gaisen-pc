use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; F1 clears paused story stains through renderer rebuilds"]
fn cutscene_retained_blood_clears_without_advancing_playback() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut story = Story::load(&disc, 23, 11, 1).unwrap();
    for _ in 0..=860 {
        story.step(true).unwrap();
    }
    assert!(story.state.effects.particles.entries.is_empty());
    assert!(story.contact.blood.latest() > 0);
    let state = postcard::to_allocvec(&story.state).unwrap();
    let graphics = Graphics {
        blood: crate::settings::Blood::modern(),
        asset_pack: false,
        ..Default::default()
    };
    for clear in [false, true] {
        let path = output.join(format!("story-retained-blood-cleared-{clear}.png"));
        capture_frame(&story.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&story, &graphics, renderer)?;
            drawing.update(&story, renderer, None)?;
            assert!(
                drawing
                    .scene
                    .environment
                    .as_ref()
                    .unwrap()
                    .blood
                    .pages()
                    .next()
                    .is_some()
            );
            if clear {
                let mut presentation =
                    crate::tenchu2::native::presentation::ContentDrawing::Story(Box::new(drawing));
                presentation.clear_blood();
                let crate::tenchu2::native::presentation::ContentDrawing::Story(cleared) =
                    presentation
                else {
                    unreachable!()
                };
                drawing = *cleared;
                for _ in 0..3 {
                    drawing.update(&story, renderer, None)?;
                }
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
                let mut rebuilt = Drawing::new(&story, &graphics, renderer)?;
                rebuilt.preserve_environment(&mut drawing);
                rebuilt.update(&story, renderer, None)?;
                assert!(
                    rebuilt
                        .scene
                        .environment
                        .as_ref()
                        .unwrap()
                        .blood
                        .pages()
                        .next()
                        .is_none(),
                    "renderer rebuild resurrected cleared stains"
                );
                drawing = rebuilt;
            }
            Ok((drawing.scene, story.state.view, drawing.texts))
        })
        .unwrap();
        assert!(postcard::to_allocvec(&story.state).unwrap() == state);
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; authored blood, modern pools, pause and preset restoration"]
fn cutscene_blood_renders_with_original_and_modern_settings() {
    use crate::settings::{BloodStyle, presets};
    use tenchu_assets::tenchu2::campaign::{Campaign, Kind};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    for (event, frame) in [(11, 620), (5, 305)] {
        let sequence = campaign
            .sequences
            .iter()
            .position(|s| s.kind == Kind::Event && s.resource == event)
            .unwrap();
        let mut story = Story::load(&disc, sequence, event, 1).unwrap();
        for _ in 0..=frame {
            story.step(true).unwrap();
        }
        if event == 11 {
            assert!(
                story
                    .state
                    .effects
                    .frame
                    .points
                    .iter()
                    .any(|p| p.blood.is_some())
            );
        } else {
            assert!(
                story
                    .state
                    .effects
                    .frame
                    .quads
                    .iter()
                    .any(|q| q.blood == Some(tenchu_core::tenchu2::effects::surface::Blood::Death))
            );
        }
        let state = postcard::to_allocvec(&story.state).unwrap();
        story.restore(postcard::from_bytes(&state).unwrap());
        let mut original = None;
        for (name, preset, style) in [
            ("original", 0, BloodStyle::Original),
            ("hd", 0, BloodStyle::OriginalHd),
            ("modern-high", presets::MODERN_HIGH, BloodStyle::Modern),
            ("modern-ultra", presets::MODERN_ULTRA, BloodStyle::Modern),
            ("off", 0, BloodStyle::Off),
            ("restored", 0, BloodStyle::Original),
        ] {
            let mut graphics = Graphics::default();
            graphics.set_preset(preset);
            graphics.asset_pack = false;
            graphics.blood.hit = style;
            graphics.blood.ground = style;
            graphics.blood.death = style;
            let path = output.join(format!("story-blood-{event}-{frame}-{name}.png"));
            capture_frame(&story.sprites.vram, &graphics, &path, |renderer| {
                let mut drawing = Drawing::new(&story, &graphics, renderer)?;
                if event == 5 {
                    let pool = &story.state.effects.frame.quads[0];
                    let center = glam::Vec3::from(pool.position.to_array().map(|n| n as f32));
                    let mut camera =
                        crate::debug::camera::Camera::from_view(story.state.view, 300.0);
                    camera.pitch = -0.8;
                    camera.position = center - camera.forward() * 3000.0;
                    drawing.camera = Some(camera);
                }
                for _ in 0..3 {
                    drawing.update(&story, renderer, None)?;
                }
                if style == BloodStyle::Modern && event == 5 {
                    assert!(
                        !drawing
                            .scene
                            .environment
                            .as_ref()
                            .unwrap()
                            .blood
                            .pool_geometry()
                            .vertices
                            .is_empty(),
                        "modern cutscene pool was filtered without replacement"
                    );
                }
                let view = drawing
                    .camera
                    .map_or(story.state.view, |camera| camera.source_view());
                Ok((drawing.scene, view, drawing.texts))
            })
            .unwrap();
            assert!(
                postcard::to_allocvec(&story.state).unwrap() == state,
                "render/preset changed playback"
            );
            let image = crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba;
            if name == "original" {
                original = Some(image);
            } else if name == "restored" {
                assert!(
                    original.as_ref().unwrap() == &image,
                    "original cutscene blood did not restore exactly"
                );
            } else if name == "off" {
                assert!(
                    original.as_ref().unwrap() != &image,
                    "blood is invisible in scene {event}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; checks the training opening's stage palette instead of black Gaisen fog"]
fn opening_scene_uses_its_own_distance_palette() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut story = Story::load(&disc, 0, 201, 1).unwrap();
    for _ in 0..60 {
        story.step(true).unwrap();
    }
    for fog in [true, false] {
        let graphics = Graphics {
            fog,
            asset_pack: false,
            ..Default::default()
        };
        let path = output.join(format!("jp-opening-palette-fog-{fog}.png"));
        capture_frame(&story.level.vram, &graphics, &path, |renderer| {
            let mut models = models::Models::new(&story, &graphics, renderer)?;
            let (geometry, texts) = models.geometry(&story, renderer, None, None, None)?;
            assert!(
                geometry
                    .terrain
                    .iter()
                    .all(|v| v.flags & crate::render_mesh::FOG == 0),
                "native palette must not receive Gaisen's fog again"
            );
            assert!(
                geometry.terrain.iter().any(|v| v.color[0] != v.color[1]),
                "stage palette was lost"
            );
            Ok((
                SceneDrawing::new(geometry, renderer, &story.level),
                story.state.view,
                texts,
            ))
        })
        .unwrap();
        let image = crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap();
        let visible = image
            .rgba
            .chunks_exact(4)
            .enumerate()
            .filter(|(i, p)| {
                let y = i / image.size[0] as usize;
                y > 220 && y < 620 && p[..3].iter().any(|&v| v > 24)
            })
            .count();
        assert!(
            visible > 200_000,
            "opening terrain is blacked out ({visible} visible pixels)"
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and a GPU; streamed cutscene meshes and facial updates match a fresh frame"]
fn animated_story_draw_matches_fresh_meshes() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    for refined in [false, true] {
        let graphics = Graphics {
            smooth_models: refined,
            models: crate::settings::Models {
                characters: if refined { 2 } else { 0 },
                ..Default::default()
            },
            asset_pack: false,
            ..Default::default()
        };
        for (sequence, event) in [(0, 201), (10, 4)] {
            let streamed = output.join(format!("story-{event}-{refined}-streamed.png"));
            let fresh = output.join(format!("story-{event}-{refined}-fresh.png"));
            let mut story = Story::load(&disc, sequence, event, 17).unwrap();
            story.step(true).unwrap();
            let vram = story.level.vram.clone();
            capture_frame(&vram, &graphics, &streamed, |renderer| {
                let mut drawing = Drawing::new(&story, &graphics, renderer)?;
                for _ in 1..=600 {
                    story.step(true)?;
                    drawing.update(&story, renderer, None)?;
                }
                Ok((drawing.scene, story.state.view, drawing.texts))
            })
            .unwrap();
            capture_frame(&vram, &graphics, &fresh, |renderer| {
                let drawing = Drawing::new(&story, &graphics, renderer)?;
                Ok((drawing.scene, story.state.view, drawing.texts))
            })
            .unwrap();
            assert_eq!(
                std::fs::read(&streamed).unwrap(),
                std::fs::read(&fresh).unwrap(),
                "event {event}: animated face/material updates differ from fresh scene"
            );
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; light tools work during cutscenes without modifying their script state"]
fn cutscene_movable_lights_follow_camera_and_hide_markers_in_photo_mode() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut story = Story::load(&disc, 10, 4, 17).unwrap();
    for _ in 0..120 {
        story.step(true).unwrap();
    }
    let before = postcard::to_allocvec(&story.state).unwrap();
    let mut settings = crate::settings::Settings::default();
    settings.shadow_preview();
    settings.graphics.lighting.markers = true;
    let graphics = super::super::super::preferences::graphics(&settings);
    capture_frame(
        &story.level.vram,
        &graphics,
        &output.join("story-light-tools.png"),
        |renderer| {
            let mut drawing = Drawing::new(&story, &graphics, renderer)?;
            let camera = crate::debug::camera::Camera::from_view(story.state.view, 300.0);
            let mut debug = crate::debug::Debug::default();
            debug.light = Some(camera.position);
            debug.light_follow = true;
            drawing.update(&story, renderer, Some(&debug))?;
            assert!(
                drawing
                    .scene
                    .lighting
                    .debug_light
                    .unwrap()
                    .distance(camera.position)
                    < 2.0
            );
            assert!(drawing.scene.environment.is_some());
            let moved = crate::debug::camera::Camera {
                position: camera.position + glam::Vec3::X * 500.0,
                ..camera
            };
            drawing.camera = Some(moved);
            drawing.update(&story, renderer, Some(&debug))?;
            assert!(
                drawing
                    .scene
                    .lighting
                    .debug_light
                    .unwrap()
                    .distance(moved.position)
                    < 2.0
            );
            debug.photo_mode = true;
            drawing.update(&story, renderer, Some(&debug))?;
            assert!(drawing.labels.is_empty());
            assert_eq!(postcard::to_allocvec(&story.state).unwrap(), before);
            Ok((drawing.scene, moved.source_view(), drawing.texts))
        },
    )
    .unwrap();
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MOTION_REVIEW and GPU; actor motion in cutscenes shares the fixed-tick solver and leaves scripts unchanged"]
fn native_story_secondary_motion_tracks_authored_poses() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MOTION_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
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
    let mut story = Story::load(&disc, 0, 201, 17).unwrap();
    let vram = story.level.vram.clone();
    capture_frame(
        &vram,
        &graphics,
        &out.join("story-motion.png"),
        |renderer| {
            let mut drawing = Drawing::new(&story, &graphics, renderer)?;
            let mut changed = false;
            for tick in 0..160 {
                story.step(true)?;
                let before = postcard::to_allocvec(&story.state).unwrap();
                drawing.tick(&story, &graphics)?;
                let frame = drawing.motion.sample(1.0);
                if tick % 10 == 0 {
                    let (moving, _) =
                        drawing
                            .models
                            .geometry(&story, renderer, None, None, Some(&frame))?;
                    let (rigid, _) = drawing
                        .models
                        .geometry(&story, renderer, None, None, None)?;
                    changed |= moving
                        .actors
                        .iter()
                        .zip(&rigid.actors)
                        .any(|(a, b)| a.position != b.position);
                }
                assert_eq!(postcard::to_allocvec(&story.state).unwrap(), before);
            }
            assert!(
                changed,
                "no cutscene motion; models: {:?}",
                story
                    .resources
                    .models
                    .values()
                    .map(|m| &m.name)
                    .collect::<Vec<_>>()
            );
            let before = postcard::to_allocvec(&drawing.motion).unwrap();
            let hero = story
                .resources
                .script
                .actors
                .iter()
                .position(|a| story.resources.models[&a.model].name == "rikimaru")
                .unwrap();
            let chest = story.matrices(hero)?.unwrap()[1].translation;
            let center = glam::Vec3::from(chest.to_array().map(|v| v as f32));
            let mut camera = crate::debug::camera::Camera::from_view(story.state.view, 300.0);
            camera.position = center + glam::Vec3::new(1800.0, -400.0, 2200.0);
            let forward = (center - camera.position).normalize();
            camera.yaw = forward.x.atan2(forward.z);
            camera.pitch = (-forward.y).asin();
            drawing.camera = Some(camera);
            drawing.update(&story, renderer, None)?;
            drawing.update(&story, renderer, None)?;
            assert_eq!(postcard::to_allocvec(&drawing.motion).unwrap(), before);
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        },
    )
    .unwrap();
}
