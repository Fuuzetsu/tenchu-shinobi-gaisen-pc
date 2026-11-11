use super::*;
use crate::tenchu2::native::models;
use crate::visuals::{appearance::library::Library, pack::Pack};
use tenchu_assets::tenchu2::campaign::{Campaign, Kind};

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; hero-only cutscene body overrides preserve scripts and attachments"]
fn native_story_model_override_preserves_cast_and_playback() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let donor = models::donor(&gaisen).unwrap();
    let folder = output.join(format!("custom-body-{}", std::process::id()));
    crate::visuals::export(&donor.mission, &folder).unwrap();
    let bodies = Pack::bodies(folder.to_str().unwrap()).unwrap();
    for (hero, selected, weapon, moveset) in [
        (0, "tenchu2-jp:ayamea", "", ""),
        (1, "tenchu2-jp:rikimaru", "", ""),
        (2, "disc:0", "", ""),
        (0, "pack:0000", "", ""),
        (0, "", "tenchu2-jp:kagami", ""),
        (1, "tenchu2-jp:ayame", "tenchu2-jp:yorojyu", ""),
        (2, "disc:1", "tenchu2-jp:tatu2", ""),
        (0, "disc:1", "", "tenchu2-jp:rikimaru"),
    ] {
        let (sequence, event) = campaign
            .sequences
            .iter()
            .enumerate()
            .find(|(i, row)| row.kind == Kind::Event && campaign.mission(hero, *i).is_some())
            .map(|(i, row)| (i, row.resource))
            .unwrap();
        let mut story = Story::load(&disc, sequence, event, 17).unwrap();
        let mut reference = Story::load(&disc, sequence, event, 17).unwrap();
        let bytes = postcard::to_allocvec(&story.state).unwrap();
        assert!(
            story
                .models
                .sync(
                    &story.resources,
                    hero,
                    &library,
                    &bodies,
                    &crate::settings::Settings {
                        player_model: selected.into(),
                        weapon_model: weapon.into(),
                        model_moveset: !moveset.is_empty(),
                        moveset_model: moveset.into(),
                        ..Default::default()
                    }
                )
                .unwrap()
        );
        assert!(
            !story.models.actors.is_empty() || !story.models.equipment.is_empty(),
            "hero {hero}, event {event}: missing hero track"
        );
        assert_eq!(bytes, postcard::to_allocvec(&story.state).unwrap());
        assert!(
            !story
                .models
                .sync(
                    &story.resources,
                    hero,
                    &library,
                    &bodies,
                    &crate::settings::Settings {
                        player_model: selected.into(),
                        weapon_model: weapon.into(),
                        model_moveset: !moveset.is_empty(),
                        moveset_model: moveset.into(),
                        ..Default::default()
                    }
                )
                .unwrap()
        );
        for _ in 0..360.min(story.resources.script.duration - 1) {
            story.step(true).unwrap();
            reference.step(true).unwrap();
            assert_eq!(
                postcard::to_allocvec(&story.state).unwrap(),
                postcard::to_allocvec(&reference.state).unwrap()
            );
        }
        let graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        let hero_slot = story
            .resources
            .script
            .actors
            .iter()
            .enumerate()
            .find(|(slot, a)| {
                story.models.actors.contains(&a.model) || story.models.equipment.contains_key(slot)
            })
            .map(|(slot, _)| slot)
            .unwrap();
        let center = glam::Vec3::from(
            reference.matrices(hero_slot).unwrap().unwrap()[0]
                .translation
                .to_array()
                .map(|v| v as f32),
        ) - glam::Vec3::Y * 600.0;
        let mut camera = crate::debug::camera::Camera::from_view(reference.state.view, 300.0);
        camera.focus(center, false);
        let label = format!(
            "hero-{hero}-{}{}{}",
            selected.replace(':', "-"),
            if weapon.is_empty() {
                String::new()
            } else {
                format!("-weapon-{}", weapon.replace(':', "-"))
            },
            if moveset.is_empty() {
                String::new()
            } else {
                format!("-moves-{}", moveset.replace(':', "-"))
            }
        );
        for (suffix, scene) in [("before", &reference), ("after", &story)] {
            super::super::super::capture_frame(
                &scene.level.vram,
                &graphics,
                &output.join(format!("{label}-{suffix}.png")),
                |renderer| {
                    let mut models = Models::new(scene, &graphics, renderer)?;
                    let original = Models::new(&reference, &graphics, renderer)?;
                    for slot in 0..scene.resources.script.actors.len() {
                        let Some(actor) = models.actor(scene, slot, None)? else {
                            continue;
                        };
                        let source = original.actor(&reference, slot, None)?.unwrap();
                        if scene.models.actors.contains(&actor.id) {
                            assert!(actor.original.is_some());
                            actor.meshes.pose_skin(&actor.matrices, None);
                            let mut triangles = 0;
                            for (part, &matrix) in actor.parts.iter().zip(&actor.matrices) {
                                actor
                                    .meshes
                                    .visit(&part.geometry, matrix, false, 0, None, |_| {
                                        triangles += 1
                                    });
                            }
                            assert!(triangles > 50, "override has no drawable body geometry");

                            for axis in 0..3 {
                                assert!(
                                    (actor.matrices[0].translation[axis]
                                        - source.matrices[0].translation[axis])
                                        .abs()
                                        < 2000,
                                    "override moved away from the authored root"
                                );
                            }
                            assert_ne!(actor.parts.as_ptr(), source.parts.as_ptr());
                            for attachment in scene
                                .resources
                                .attachments
                                .iter()
                                .filter(|a| a.actor == slot)
                            {
                                if let Some(index) = tenchu_assets::tenchu2::BONES
                                    .iter()
                                    .position(|&b| b == attachment.bone)
                                {
                                    let actual = actor.attachment(attachment.bone).translation;
                                    let target = actor.matrices[index].translation;
                                    for axis in 0..3 {
                                        assert!((actual[axis] - target[axis]).abs() < 3);
                                    }
                                }
                            }
                        } else {
                            assert!(actor.original.is_none());
                            assert_eq!(actor.matrices, source.matrices);
                        }
                    }
                    for (&slot, gear) in &models.equipment {
                        assert_eq!(gear.len(), scene.models.equipment[&slot].len());
                        for part in gear {
                            let mut triangles = 0;
                            part.meshes.visit(
                                &part.part.geometry,
                                Matrix::default(),
                                false,
                                0,
                                None,
                                |_| triangles += 1,
                            );
                            assert!(triangles > 0, "replacement equipment is not drawable");
                        }
                    }
                    let (geometry, texts) =
                        models.geometry(scene, renderer, Some(camera), None, None)?;
                    Ok((
                        super::super::super::SceneDrawing::new(geometry, renderer, &scene.level),
                        camera.source_view(),
                        texts,
                    ))
                },
            )
            .unwrap();
        }
        let before = std::fs::read(output.join(format!("{label}-before.png"))).unwrap();
        let after = std::fs::read(output.join(format!("{label}-after.png"))).unwrap();
        assert!(before != after, "override missing from rendered cutscene");
        assert!(
            story
                .models
                .sync(
                    &story.resources,
                    hero,
                    &library,
                    &bodies,
                    &crate::settings::Settings::default()
                )
                .unwrap()
        );
        assert!(story.models.actors.is_empty() && story.models.equipment.is_empty());
        let path = output.join(format!("{label}-reset.png"));
        super::super::super::capture_frame(&story.level.vram, &graphics, &path, |renderer| {
            let mut models = Models::new(&story, &graphics, renderer)?;
            let (geometry, texts) = models.geometry(&story, renderer, Some(camera), None, None)?;
            Ok((
                super::super::super::SceneDrawing::new(geometry, renderer, &story.level),
                camera.source_view(),
                texts,
            ))
        })
        .unwrap();
        assert!(
            before == std::fs::read(path).unwrap(),
            "reset did not restore original rendering"
        );
    }
}
