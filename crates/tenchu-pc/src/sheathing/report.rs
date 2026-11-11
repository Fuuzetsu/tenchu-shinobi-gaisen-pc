use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; selecting a T1 hero keeps the sheathing fallback instead of the static attention pose"]
fn t1_selected_heroes_keep_sheathed_idle_fallback() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let library = crate::visuals::appearance::library::Library::load(&assets).unwrap();
    let directory = std::env::var_os("TENCHU_SHEATHED_REPORT").map(std::path::PathBuf::from);
    if let Some(directory) = &directory {
        std::fs::create_dir_all(directory).unwrap();
    }
    for character in 0..2 {
        let body = library.get(&format!("disc:{character}")).unwrap();
        let mut fallback = None;
        for selected in [false, true] {
            let mut game = crate::session::Options {
                character,
                mission: 1,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            for id in game.roster.order.clone() {
                if id != game.player_id {
                    game.roster.get_mut(id).attribute |= ATTR_SUSPEND;
                }
            }
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            configure(
                &mut game,
                &assets,
                Settings {
                    enabled: true,
                    ..Default::default()
                },
            )
            .unwrap();
            if selected {
                body.apply_moveset(&mut game, Default::default()).unwrap();
            }
            game.roster
                .get_mut(game.player_id)
                .sheathing
                .as_mut()
                .unwrap()
                .input(true);
            for _ in 0..100 {
                game.step(0).unwrap();
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(false);
            }
            assert_eq!(game.player().motion.action, MOT_NORMAL);
            assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN, 0);
            let pose = postcard::to_allocvec(&game.player().motion.pose).unwrap();
            if let Some(expected) = &fallback {
                assert_eq!(&pose, expected);
            } else {
                fallback = Some(pose);
            }
            if let Some(directory) = &directory {
                tests::capture_pose(
                    &game,
                    &directory.join(format!("hero-{character}-selected-{selected}.png")),
                );
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; full replacement blades cannot reappear in stowed grips"]
fn sheathed_grips_do_not_use_full_weapon_replacements() {
    use crate::{
        render_mesh,
        visuals::{models::Models, obj, pack::Pack},
    };
    use tenchu_core::math::Matrix;
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let game = crate::session::Options::default().load(&assets).unwrap();
    let mut pack = Pack::default();
    pack.materials.insert("test".into(), Default::default());
    for (&kind, geometry) in game
        .mission
        .weapons
        .models
        .iter()
        .filter(|(k, _)| game.mission.weapons.grips.contains_key(k))
    {
        let mut vertices = Vec::new();
        render_mesh::append_geometry(&mut vertices, geometry, Matrix::default(), None);
        pack.models.insert(
            format!("weapons/{kind:04x}"),
            vertices
                .chunks_exact(3)
                .map(|v| obj::Triangle {
                    material: "test".into(),
                    corners: std::array::from_fn(|i| obj::Corner {
                        index: i,
                        position: v[i].position,
                        normal: v[i].normal,
                        color: v[i].color,
                        uv: v[i].uv,
                    }),
                })
                .collect(),
        );
    }
    for materials in [false, true] {
        let graphics = crate::settings::Graphics {
            asset_pack: true,
            weapon_reflections: if materials { 50 } else { 0 },
            ..Default::default()
        };
        let models = Models::build(&game.mission, &pack, &graphics).unwrap();
        for (&kind, grip) in game.mission.weapons.grips.iter() {
            let mut expected = Vec::new();
            let mut actual = Vec::new();
            render_mesh::append_geometry(&mut expected, grip, Matrix::default(), None);
            render_mesh::append_geometry(&mut actual, grip, Matrix::default(), Some(&models));
            assert_eq!(
                actual.iter().map(|v| v.position).collect::<Vec<_>>(),
                expected.iter().map(|v| v.position).collect::<Vec<_>>(),
                "stowed weapon {kind}, materials {materials}"
            );
            let mut drawn = Vec::new();
            render_mesh::append_geometry(
                &mut drawn,
                &game.mission.weapons.models[&kind],
                Matrix::default(),
                Some(&models),
            );
            assert!(drawn.len() > actual.len(), "drawn blade disappeared");
        }
    }
}
