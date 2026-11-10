use super::*;

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
            let (geometry, texts) = models.geometry(&story, renderer, None, None)?;
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
