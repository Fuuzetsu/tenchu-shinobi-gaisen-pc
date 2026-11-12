use super::*;
use crate::visuals::{appearance, interpolation::Part};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; imported M1/M10 cast and equipment follow native replacements without changing source visibility, player or script"]
fn imported_cinematics_use_native_cast_and_equipment() {
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    let mut onikage = 0;
    for (stage, hero) in [(0, 0), (0, 1), (7, 0), (7, 1)] {
        let session = tests::session_stage(stage, hero, 1);
        let director = &session.gaisen.as_ref().unwrap().director;
        let source = &director.source;
        eprintln!(
            "{} hero {hero} scene {}",
            source.mission.stage.name, source.cinematic.sequence
        );
        let before = postcard::to_allocvec(source).unwrap();
        let world = postcard::to_allocvec(&session.world).unwrap();
        let cast = cast::prepare(&session, &graphics).unwrap();
        let actors: BTreeMap<_, _> = source
            .roster
            .order
            .iter()
            .map(|&id| (source.roster.get(id).root_model, id))
            .collect();
        let onikage_roots: BTreeSet<_> = source
            .roster
            .order
            .iter()
            .filter_map(|&id| {
                let actor = source.roster.get(id);
                matches!(actor.kind, 0x87 | 0x8a).then_some(actor.root_model)
            })
            .collect();
        if stage == 7 {
            assert_eq!(source.mission.stage.order, 10);
            assert!(!onikage_roots.is_empty(), "missing Onikage encounter");
        }
        for &root in &onikage_roots {
            assert!(cast.actors.contains_key(&root));
            let native = director.native_actor(actors[&root]).unwrap();
            let definition = session
                .assets
                .resources
                .registry
                .actor(session.world.actors[native].actor.definition_kind())
                .unwrap();
            eprintln!(
                "M{} hero {hero}: Onikage maps to native {} ({:#x})",
                source.mission.stage.order, definition.model, definition.kind
            );
        }
        for replaced in [false, true] {
            let path = output.join(format!(
                "m{}-hero-{hero}-native-cast-{replaced}.png",
                source.mission.stage.order
            ));
            capture_frame(&source.mission.vram, &graphics, &path, |renderer| {
                let mut drawing = super::super::Drawing::new(&session, &graphics, renderer)?;
                let models = &mut drawing.gaisen.as_mut().unwrap().models;
                if !replaced {
                    *models = std::mem::take(models).with_cast(Default::default(), &graphics)?;
                }
                if replaced {
                    for inspection in [false, true] {
                        let mut original = Vec::new();
                        appearance::visit_parts(
                            source,
                            inspection,
                            None,
                            None,
                            |key, geometry, matrix| original.push((key, geometry.to_vec(), matrix)),
                        );
                        let mut actual = Vec::new();
                        appearance::visit_parts(
                            source,
                            inspection,
                            Some(models),
                            None,
                            |key, geometry, matrix| actual.push((key, geometry.to_vec(), matrix)),
                        );
                        let mut changed = BTreeSet::new();
                        for (key, geometry, matrix) in &original {
                            let Part::Actor { root, model, .. } = key else {
                                continue;
                            };
                            let source_actor = source.roster.get(actors[root]);
                            if cast.actors.contains_key(root) {
                                if source_actor.parts.contains(model) {
                                    let replacement = actual
                                        .iter()
                                        .find(|(part, _, _)| part == key)
                                        .expect("native replacement lost a visible body part");
                                    if replacement.1 != *geometry {
                                        changed.insert(*root);
                                    }
                                } else {
                                    assert!(
                                        !actual.iter().any(|(part, _, _)| part == key),
                                        "original NPC equipment leaked into the converted cast"
                                    );
                                }
                            } else {
                                assert!(
                                    actual.iter().any(|entry| entry.0 == *key
                                        && entry.1 == *geometry
                                        && entry.2 == *matrix),
                                    "player or source-only actor changed"
                                );
                            }
                        }
                        assert!(!changed.is_empty(), "no native cast was drawn");
                        if stage == 7 {
                            assert!(
                                changed.intersection(&onikage_roots).next().is_some(),
                                "Onikage was not drawn"
                            );
                        }
                        onikage += changed.intersection(&onikage_roots).count();
                        for &root in &changed {
                            let id = actors[&root];
                            super::player_tests::verify_actor_equipment(
                                models,
                                &session,
                                &before,
                                id,
                                director.native_actor(id).unwrap(),
                            );
                        }
                        for (key, _, _) in &actual {
                            if let Part::Actor { .. } = key {
                                assert!(
                                    original.iter().any(|(part, _, _)| part == key),
                                    "replacement changed source visibility"
                                );
                            }
                        }
                    }
                }
                drawing.show_hud = false;
                drawing.update(&session, renderer, None)?;
                Ok((drawing.scene, source.rendered_view, Vec::new()))
            })
            .unwrap();
            assert_eq!(before, postcard::to_allocvec(source).unwrap());
            assert_eq!(world, postcard::to_allocvec(&session.world).unwrap());
        }
    }
    assert!(onikage > 0, "fixture did not draw converted Onikage");
}

#[test]
#[ignore = "requires both discs; every imported mission's cast loads and preserves the separate T2 importer's geometry and joint attachments"]
fn imported_cast_matches_disc_geometry_across_missions() {
    use crate::tenchu2::native::gaisen::Source;
    use tenchu_assets::script::Language;
    use tenchu_core::tenchu2::world::Options;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let references: BTreeMap<_, _> = tenchu_assets::tenchu2::characters::load(&disc, None)
        .unwrap()
        .into_iter()
        .map(|body| (body.name, body.parts))
        .collect();
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let graphics = Graphics {
        asset_pack: false,
        ..Default::default()
    };
    let mut checked = BTreeSet::new();
    for stage in 0..11 {
        for hero in 0..2 {
            let source = Source::open(path.as_ref(), stage, hero, 1, Language::English).unwrap();
            let session = source
                .session(
                    &disc,
                    Options {
                        hero: hero as usize,
                        difficulty: 2,
                        seed: 1,
                        inventory: Default::default(),
                    },
                    false,
                    4000,
                )
                .unwrap();
            let cast = cast::prepare(&session, &graphics)
                .unwrap_or_else(|e| panic!("stage {stage} hero {hero}: {e}"));
            let director = &session.gaisen.as_ref().unwrap().director;
            for &id in &director.source.roster.order {
                let root = director.source.roster.get(id).root_model;
                let Some(actor) = cast.actors.get(&root) else {
                    continue;
                };
                let native = director.native_actor(id).unwrap();
                let definition = session
                    .assets
                    .resources
                    .registry
                    .actor(session.world.actors[native].actor.definition_kind())
                    .unwrap();
                if checked.insert(definition.model.clone()) {
                    assert!(
                        bind_faces(&cast.bodies[actor.body].parts)
                            == bind_faces(&references[&definition.model]),
                        "{} lost geometry or changed joint attachments",
                        definition.model
                    );
                }
            }
            eprintln!(
                "Prepared {} hero {hero}, {} converted actors",
                director.source.mission.stage.name,
                cast.actors.len()
            );
        }
    }
    assert!(checked.contains("suzaku"));
    eprintln!("Compared {} distinct native bodies", checked.len());
}

fn bind_faces(parts: &[tenchu_assets::model::Part]) -> Vec<Vec<u8>> {
    let roots = appearance::skin::Pose::new(parts).roots;
    let mut faces = Vec::new();
    for (part, source) in parts.iter().enumerate() {
        for geometry in &source.geometry {
            for face in &geometry.faces {
                let corners: Vec<_> = face
                    .corners
                    .iter()
                    .map(|corner| {
                        let index = usize::from(corner.vertex);
                        let position = (roots[part]
                            + glam::Vec3::from(geometry.vertices[index].map(f32::from)))
                        .to_array()
                        .map(|v| v as i32);
                        (position, geometry.joints[index], corner.uv, corner.color)
                    })
                    .collect();
                faces.push(postcard::to_allocvec(&(face.texture, corners)).unwrap());
            }
        }
    }
    faces.sort_unstable();
    faces
}
