use super::*;
use crate::tenchu2::native::render::{SceneDrawing, capture_frame};

fn rendered_bottom(actor: &Actor<'_>) -> f32 {
    actor.meshes.pose_skin(&actor.matrices, None);
    let mut bottom = f32::NEG_INFINITY;
    for (part, &matrix) in actor.parts.iter().zip(&actor.matrices) {
        actor
            .meshes
            .visit(&part.geometry, matrix, false, 0, None, |triangle| {
                for vertex in triangle {
                    bottom = bottom.max(vertex.position[1]);
                }
            });
    }
    assert!(bottom.is_finite());
    bottom
}

#[test]
#[ignore = "requires both discs, GPU and TENCHU_STORY_CONTACT_REVIEW; replacement bodies retain authored standing, kneeling and falling contact heights"]
fn native_story_replacements_retain_body_contact_height() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_STORY_CONTACT_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let donor = models::donor(&gaisen).unwrap();
    let folder = output.join(format!("custom-body-{}", std::process::id()));
    crate::visuals::export(&donor.mission, &folder).unwrap();
    let bodies = Pack::bodies(folder.to_str().unwrap()).unwrap();
    let mut worst = 0.0f32;
    let mut cases: Vec<_> = (0..3)
        .map(|hero| {
            campaign
                .sequences
                .iter()
                .enumerate()
                .find(|(i, row)| row.kind == Kind::Event && campaign.mission(hero, *i).is_some())
                .map(|(i, row)| (hero, i, row.resource))
                .unwrap()
        })
        .collect();
    cases.extend([(0, 2, 2021), (1, 103, 2031)]);
    for (hero, sequence, event) in cases {
        for refined in [false, true] {
            for selected in [
                "",
                "disc:0",
                "disc:1",
                "tenchu2-jp:rikimaru",
                "tenchu2-jp:ayame",
                "pack:0000",
            ] {
                let mut story = Story::load(&disc, sequence, event, 17).unwrap();
                story.interpolate(true).unwrap();
                let slot = story
                    .resources
                    .script
                    .actors
                    .iter()
                    .position(|a| {
                        story.resources.models.get(&a.model).is_some_and(|model| {
                            tenchu_assets::tenchu2::characters::hero(&model.name) == Some(hero)
                        })
                    })
                    .unwrap();
                let graphics = Graphics {
                    fog: false,
                    asset_pack: false,
                    models: crate::settings::Models {
                        heroes: refined,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let vram = story.sprites.vram.clone();
                let label = if selected.is_empty() {
                    "original"
                } else {
                    selected
                };
                let path = output.join(format!(
                    "event-{event}-{}-refined-{refined}.png",
                    label.replace(':', "-")
                ));
                capture_frame(&vram, &graphics, &path, |renderer| {
                    let original = Models::new(&story, &graphics, renderer)?;
                    story.models.sync(
                        &story.resources,
                        hero,
                        &library,
                        &bodies,
                        &crate::settings::Settings {
                            player_model: selected.into(),
                            ..Default::default()
                        },
                    )?;
                    let mut replacement = Models::new(&story, &graphics, renderer)?;
                    let mut error = 0.0f32;
                    let mut height = [i32::MAX, i32::MIN];
                    for _ in 0..600.min(story.resources.script.duration - 1) {
                        story.step(true)?;
                        let pose = story
                            .interpolation
                            .as_ref()
                            .unwrap()
                            .sample(story.state.frame as u64, 0.5);
                        let source = original.actor(&story, slot, None)?.unwrap();
                        height[0] = height[0].min(source.matrices[0].translation.y);
                        height[1] = height[1].max(source.matrices[0].translation.y);
                        let actor = replacement.actor(&story, slot, None)?.unwrap();
                        let difference = rendered_bottom(&actor) - rendered_bottom(&source);
                        error = error.max(difference.abs());
                        if let Some(pose) = &pose {
                            let source = original.actor(&story, slot, Some(pose))?.unwrap();
                            let actor = replacement.actor(&story, slot, Some(pose))?.unwrap();
                            let difference = rendered_bottom(&actor) - rendered_bottom(&source);
                            error = error.max(difference.abs());
                        }
                    }
                    if matches!(event, 2021 | 2031) {
                        assert!(
                            height[1] - height[0] > 1000,
                            "fixture must exercise the authored fall"
                        );
                    }
                    worst = worst.max(error);
                    eprintln!("hero={hero} event={event} body={label} refined={refined} maximum contact error={error:.3}");
                    let source = original.actor(&story, slot, None)?.unwrap();
                    let position = source.matrices[0].translation.to_array().map(|v| v as f32);
                    let center = glam::Vec3::from(position) - glam::Vec3::Y * 300.0;
                    let mut camera = crate::debug::camera::Camera::from_view(story.state.view, 300.0);
                    camera.focus(center, false);
                    let state = postcard::to_allocvec(&story.state).unwrap();
                    let (geometry, _) =
                        replacement.geometry(&story, renderer, Some(camera), None, None)?;
                    assert_eq!(state, postcard::to_allocvec(&story.state).unwrap());
                    Ok((
                        SceneDrawing::new(geometry, renderer, &story.level),
                        camera.source_view(),
                        Vec::new(),
                    ))
                })
                .unwrap();
            }
        }
    }
    assert!(
        worst <= 1.1,
        "replacement changed authored contact height by {worst}"
    );
}
