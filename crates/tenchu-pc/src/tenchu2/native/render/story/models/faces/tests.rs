use super::*;
use crate::{
    settings::{Graphics, Settings},
    tenchu2::native::render::{Presentation, SceneDrawing, capture_frame},
    visuals::{appearance::library::Library, pack::Pack},
};
use tenchu_assets::tenchu2::campaign::{Campaign, Kind};

#[test]
#[ignore = "requires both discs, GPU and TENCHU_MODEL_REVIEW; source face cues animate compatible overrides without changing playback, including restore"]
fn native_story_override_faces_follow_authored_cues_and_restore() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let bodies = Arc::<Pack>::default();
    for (hero, selected) in [(0, "kagami"), (1, "matushin"), (2, "toda")] {
        let (sequence, event) = campaign
            .sequences
            .iter()
            .enumerate()
            .find(|(i, row)| row.kind == Kind::Event && campaign.mission(hero, *i).is_some())
            .map(|(i, row)| (i, row.resource))
            .unwrap();
        let mut story = Story::load(&disc, sequence, event, 17).unwrap();
        story
            .models
            .sync(
                &story.resources,
                hero,
                &library,
                &bodies,
                &Settings {
                    player_model: format!("tenchu2-jp:{selected}"),
                    ..Default::default()
                },
            )
            .unwrap();
        let owner = *story
            .models
            .actors
            .iter()
            .find(|id| story.resources.faces.contains_key(id))
            .unwrap();
        let slot = story
            .resources
            .script
            .actors
            .iter()
            .position(|a| a.model == owner)
            .unwrap();
        let mut reference = Story::load(&disc, sequence, event, 17).unwrap();
        let mut found = false;
        for _ in 0..story.resources.script.duration - 1 {
            story.step(true).unwrap();
            reference.step(true).unwrap();
            if story.state.patches.get(&owner).is_some_and(|indices| {
                if hero == 2 {
                    indices[1].is_some_and(|i| i > 16)
                } else {
                    indices[0].is_some_and(|i| i != 0)
                }
            }) {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "no authored face cue for hero {hero} in event {event}"
        );
        let state = postcard::to_allocvec(&story.state).unwrap();
        assert_eq!(state, postcard::to_allocvec(&reference.state).unwrap());
        let saved = story.capture_state().unwrap();
        let before_state = story.state.clone();
        for _ in 0..20 {
            story.step(true).unwrap();
        }
        let later_patches = story.state.patches.clone();
        story.restore_state(&disc, &saved).unwrap();
        assert_eq!(
            postcard::to_allocvec(&before_state).unwrap(),
            postcard::to_allocvec(&story.state).unwrap()
        );
        for preset in [
            0,
            crate::settings::presets::MODERN_HIGH,
            crate::settings::presets::MODERN_ULTRA,
        ] {
            let mut graphics = Graphics {
                asset_pack: false,
                ..Default::default()
            };
            graphics.set_preset(preset);
            graphics.fog = false;
            graphics.lighting.enabled = false;
            let mut images = Vec::new();
            let faces = story.models.skin.as_ref().unwrap().faces.clone();
            for (label, enabled) in [("before", false), ("after", true), ("restored", true)] {
                story.models.skin.as_mut().unwrap().faces = enabled.then(|| faces.clone().unwrap());
                let path = output.join(format!(
                    "face-{selected}-hero-{hero}-event-{event}-preset-{preset}-{label}.png"
                ));
                capture_frame(&story.sprites.vram, &graphics, &path, |renderer| {
                    let mut models = super::super::Models::new(&story, &graphics, renderer)?;
                    let actor = models.actor(&story, slot, None)?.unwrap();
                    let head = actor.matrices[2];
                    let center =
                        glam::Vec3::from_array(head.translation.to_array().map(|n| n as f32));
                    let eye = glam::Vec3::from_array(
                        head.transform([0, -30, -850]).to_array().map(|n| n as f32),
                    );
                    let forward = (center - eye).normalize();
                    let camera = crate::debug::camera::Camera {
                        position: eye,
                        yaw: forward.x.atan2(forward.z),
                        pitch: (-forward.y).asin(),
                        fov: 38.0,
                    };
                    let (geometry, texts) = models.geometry(
                        &story,
                        renderer,
                        Presentation {
                            camera: Some(camera),
                            ..Default::default()
                        },
                    )?;
                    assert_eq!(state, postcard::to_allocvec(&story.state).unwrap());
                    assert_eq!(models.faces.is_empty(), !enabled);
                    if enabled {
                        let textures = models.faces.iter().find(|f| f.owner == owner).unwrap();
                        let mut expected = (*textures.base).clone();
                        for index in story.state.patches[&owner].into_iter().flatten() {
                            textures.patches[usize::from(index)].apply(&mut expected);
                        }
                        assert!(
                            textures.materials.iter().any(|m| m.previous != m.original),
                            "face pixels did not animate for {selected}"
                        );
                        for material in &textures.materials {
                            assert_eq!(material.previous, material.region.pixels(&expected));
                        }
                        if label == "restored" {
                            for textures in &mut models.faces {
                                textures.update(&later_patches, renderer);
                            }
                            models.invalidate_textures();
                            models.geometry(
                                &story,
                                renderer,
                                Presentation {
                                    camera: Some(camera),
                                    ..Default::default()
                                },
                            )?;
                            for material in &models
                                .faces
                                .iter()
                                .find(|f| f.owner == owner)
                                .unwrap()
                                .materials
                            {
                                assert_eq!(material.previous, material.region.pixels(&expected));
                            }
                        }
                    }
                    Ok((
                        SceneDrawing::new(geometry, renderer, &story.level),
                        camera.source_view(),
                        texts,
                    ))
                })
                .unwrap();
                images
                    .push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
            }
            assert!(
                images[0].rgba != images[1].rgba,
                "invisible face change {selected}"
            );
            assert_eq!(images[1].rgba, images[2].rgba, "restore {selected}");
            story.models.skin.as_mut().unwrap().faces = faces;
            eprintln!(
                "hero {hero}, {selected}, sequence {sequence}, event {event}, frame {}, preset {preset}: face cues {:?}",
                story.state.frame, story.state.patches[&owner]
            );
        }
    }
}
