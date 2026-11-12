use super::*;
use crate::{
    settings::Settings,
    visuals::{appearance, appearance::library::Library, interpolation::Part},
};

#[test]
#[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; M10 player bodies and independent equipment survive the imported cinematic without changing its cast or script"]
fn imported_cinematic_uses_selected_body_and_equipment() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = crate::tenchu2::native::models::donor(&assets).unwrap();
    let folder = output.join(format!("imported-custom-body-{}", std::process::id()));
    crate::visuals::export(&donor.mission, &folder).unwrap();
    let bodies = crate::visuals::pack::Pack::bodies(folder.to_str().unwrap()).unwrap();
    let graphics = Graphics {
        fog: false,
        asset_pack: false,
        ..Default::default()
    };
    let unarmed = library
        .bodies
        .iter()
        .find(|body| {
            body.equipment
                .as_ref()
                .is_some_and(|gear| gear.attachments.is_empty())
        })
        .expect("missing unarmed equipment fixture")
        .id
        .as_str();
    for (selected, weapon, moveset) in [
        ("tenchu2-jp:rikimaru", "", true),
        ("disc:0", "tenchu2-jp:yorojyu", false),
        ("tenchu2-jp:ayame", "tenchu2-jp:kagami", false),
        ("pack:0000", "tenchu2-jp:tatu2", false),
        ("", unarmed, false),
    ] {
        let mut session = tests::session_stage(9, 1, 1);
        let source_state =
            postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director.source).unwrap();
        for overridden in [false, true] {
            if overridden {
                assert!(
                    session
                        .models
                        .sync(
                            &mut session.world,
                            &mut session.assets,
                            &mut donor,
                            &library,
                            &bodies,
                            &Settings {
                                player_model: selected.into(),
                                weapon_model: weapon.into(),
                                model_moveset: moveset,
                                ..Default::default()
                            },
                        )
                        .unwrap()
                );
                if let Some(body) = library.get(selected) {
                    let appearance = player::appearance(&session, &graphics).unwrap().unwrap();
                    assert!(
                        appearance.parts == body.parts,
                        "{selected}: rig conversion changed the selected body"
                    );
                }
            }
            let world_state = postcard::to_allocvec(&session.world).unwrap();
            let source = &session.gaisen.as_ref().unwrap().director.source;
            let root = source.player().root_model;
            let mut original_cast = Vec::new();
            render_mesh::visit_character_parts(source, true, |key, geometry, matrix| {
                if matches!(key, Part::Actor { root: r, .. } if r != root) {
                    original_cast.push((key, geometry.to_vec(), matrix));
                }
            });
            let path = output.join(format!(
                "m10-{}-{}-{overridden}.png",
                selected.replace(':', "-"),
                weapon.replace(':', "-")
            ));
            capture_frame(&source.mission.vram, &graphics, &path, |renderer| {
                let mut drawing = super::super::Drawing::new(&session, &graphics, renderer)?;
                let models = &drawing.gaisen.as_ref().unwrap().models;
                let mut cast = Vec::new();
                let mut body_parts = 0;
                let mut attachments = Vec::new();
                appearance::visit_parts(
                    source,
                    true,
                    Some(models),
                    None,
                    |key, geometry, matrix| match key {
                        Part::Actor { root: r, model, .. } if r == root => {
                            if source.player().parts.contains(&model) {
                                body_parts += 1;
                            } else {
                                assert!(
                                    !overridden,
                                    "original player equipment leaked into the override"
                                );
                            }
                        }
                        Part::Actor { .. } => cast.push((key, geometry.to_vec(), matrix)),
                        Part::Equipment { attachment, .. } => attachments.push(attachment),
                        _ => {}
                    },
                );
                assert_eq!(body_parts, 15);
                assert!(cast == original_cast, "non-player cast changed");
                if overridden {
                    verify_equipment(models, &session, &source_state);
                    if weapon == unarmed {
                        assert!(attachments.is_empty(), "unarmed equipment retained swords");
                    }
                }
                let center =
                    glam::Vec3::from(source.player().position.to_array().map(|v| v as f32))
                        - glam::Vec3::Y * 850.0;
                let mut camera = crate::debug::camera::Camera {
                    position: center,
                    yaw: 0.5,
                    pitch: -0.15,
                    fov: 45.0,
                };
                camera.position -= camera.forward() * 3200.0;
                drawing.camera = Some(camera);
                drawing.show_hud = false;
                drawing.update(&session, renderer, None)?;
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            })
            .unwrap();
            assert_eq!(world_state, postcard::to_allocvec(&session.world).unwrap());
            assert_eq!(source_state, postcard::to_allocvec(source).unwrap());
        }
        while session.cinematic() {
            session.step(0).unwrap();
        }
        let state = postcard::to_allocvec(&session.world).unwrap();
        let path = output.join(format!(
            "m10-{}-{}-gameplay.png",
            selected.replace(':', "-"),
            weapon.replace(':', "-")
        ));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = super::super::Drawing::new(&session, &graphics, renderer)?;
            drawing.update(&session, renderer, None)?;
            Ok((drawing.scene, session.world.camera.view, Vec::new()))
        })
        .unwrap();
        assert_eq!(state, postcard::to_allocvec(&session.world).unwrap());
    }
}

fn verify_equipment(
    models: &crate::visuals::models::Models,
    session: &crate::tenchu2::native::Session,
    source: &[u8],
) {
    use tenchu_core::tenchu2::rig::WeaponMode;
    let mut game: tenchu_core::game::Game = postcard::from_bytes(source).unwrap();
    let mut world = session.world.clone();
    let player = world.player;
    let names: std::collections::BTreeMap<_, _> = crate::tenchu2::native::interpolation::weapons(
        &world.actors[player].actor,
        &world.instances[player],
        &session.assets,
    )
    .unwrap()
    .map(|(root, name)| (root, name.to_owned()))
    .collect();
    for drawn in [false, true] {
        if drawn {
            game.roster.get_mut(game.player_id).attribute |= tenchu_core::ids::ATTR_WEAPON_DRAWN;
        } else {
            game.roster.get_mut(game.player_id).attribute &= !tenchu_core::ids::ATTR_WEAPON_DRAWN;
        }
        world.actors[player]
            .actor
            .set_weapon(
                &mut world.instances[player].rig,
                player,
                &mut world.collision,
                &session.assets.resources,
                if drawn {
                    WeaponMode::Draw
                } else {
                    WeaponMode::Sheathe
                },
            )
            .unwrap();
        let mut expected: Vec<_> = world.instances[player]
            .rig
            .weapons
            .iter()
            .take(3)
            .flatten()
            .map(|root| {
                postcard::to_allocvec(&session.assets.weapons[&names[root]][0].geometry).unwrap()
            })
            .collect();
        let mut actual = Vec::new();
        appearance::visit_parts(&game, true, Some(models), None, |key, geometry, _| {
            if matches!(key, Part::Equipment { .. }) {
                actual.push(postcard::to_allocvec(geometry).unwrap());
            }
        });
        actual.sort();
        expected.sort();
        assert_eq!(
            actual, expected,
            "cinematic equipment disagrees with native gameplay, drawn={drawn}"
        );
    }
}
