use super::*;
use tenchu_assets::game::GameAssets;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_RETAIL_TAPE; recorded skips must not blank presentation or mutate the validated core"]
fn recorded_skips_keep_character_poses_without_changing_core_state() {
    use crate::retail::tape::Tape;
    use std::path::Path;
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let tape = std::env::var_os("TENCHU_RETAIL_TAPE").expect("TENCHU_RETAIL_TAPE");
    let assets = GameAssets::open(crate::source::open_disc(Path::new(&disc)).unwrap()).unwrap();
    let tape = Tape::open(Path::new(&tape)).unwrap();
    let mut skipped = 0;
    for (index, mission) in tape.missions.iter().enumerate() {
        let mut game = mission.initial.load(&assets).unwrap();
        for update in &mission.updates {
            update.apply(&mut game).unwrap();
            for effect in game.effects.slots.iter().flatten() {
                if let tenchu_core::effects::Effect::FlyWire(wire) = effect
                    && wire.mode == 1
                    && wire.count == 0
                {
                    eprintln!(
                        "Grapple impact: mission {} field {}",
                        index + 1,
                        update.field
                    );
                }
            }
            if update.skip == 0 {
                continue;
            }
            skipped += 1;
            assert!(game.visible.is_empty());
            let before = crate::session::tests::state(&game);
            let mut parts = 0;
            visit_character_parts(&game, false, |key, _, matrix| {
                let Part::Actor {
                    root, inspection, ..
                } = key
                else {
                    unreachable!()
                };
                assert!(!inspection);
                assert!(matrix.rotation.iter().flatten().any(|&v| v != 0));
                let actor = game
                    .roster
                    .order
                    .iter()
                    .map(|&id| game.roster.get(id))
                    .find(|actor| actor.root_model == root)
                    .unwrap();
                assert_eq!(actor.attribute & tenchu_core::ids::ATTR_SUSPEND, 0);
                parts += 1;
            });
            assert!(
                parts > 0,
                "mission {} field {} lost every actor",
                index + 1,
                update.field
            );
            assert_eq!(before, crate::session::tests::state(&game));
        }
    }
    assert_eq!(skipped, 349);
}

#[test]
#[ignore = "requires TENCHU_DISC; hinged shadow casters match live poses and remain present outside the gameplay draw list"]
fn hinged_casters_keep_live_angles_without_camera_culling() {
    use tenchu_assets::misc::Kind;
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut matched = 0;
    let mut kinds = [false; 2];
    for mission in 1..=10 {
        let mut game = crate::session::Options {
            mission,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        for _ in 0..12 {
            game.step(0).unwrap();
        }
        let before = crate::session::tests::state(&game);
        let mut poses = Vec::new();
        visit_hinged_parts(&game, |_, geometry, matrix| {
            poses.push((geometry.as_ptr(), matrix))
        });
        assert_eq!(crate::session::tests::state(&game), before);
        for &(model, matrix) in &game.misc.models {
            assert!(poses.contains(&(game.mission.common_models[&model].as_ptr(), matrix)));
            matched += 1;
        }
        game.misc.models.clear();
        let positions = |game: &tenchu_core::game::Game| {
            let mut vertices = Vec::new();
            visit_hinged_parts(game, |_, geometry, matrix| {
                append_caster(&mut vertices, geometry, matrix, None)
            });
            vertices.into_iter().map(|v| v.position).collect::<Vec<_>>()
        };
        for index in 0..game.misc.objects.len() {
            let kind = match game.misc.objects[index].kind {
                Kind::Door { .. } => 0,
                Kind::Pitfall { .. } => 1,
                _ => continue,
            };
            if kinds[kind] {
                continue;
            }
            let before = positions(&game);
            game.misc.objects[index].angle = 256;
            assert_ne!(positions(&game), before);
            assert!(game.misc.models.is_empty());
            kinds[kind] = true;
        }
    }
    assert!(matched > 0);
    assert_eq!(kinds, [true; 2]);
}

#[test]
#[ignore = "requires TENCHU_DISC; preserves the original lit hand-face omission in actor ANI"]
fn ani_hand_stops_at_the_original_lit_primitive() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mission = MissionAssets::load(&assets, 9, 0, 1).unwrap();
    let mut mesh = mission.models[&0x8b][13].geometry[0].clone();
    assert_eq!(mesh.faces.len(), 11);
    assert_eq!((mesh.faces[10].mode, mesh.faces[10].flags), (0x30, 4));
    assert_eq!(draw_tmd_faces(&mesh).count(), 10);
    let mut vertices = Vec::new();
    append_geometry(&mut vertices, &[mesh.clone()], Matrix::default(), None);
    assert_eq!(vertices.len(), 39);
    mesh.faces.push(mesh.faces[0].clone());
    vertices.clear();
    append_geometry(&mut vertices, &[mesh], Matrix::default(), None);
    assert_eq!(
        vertices.len(),
        39,
        "DrawTMD stops at the lit face, including any later unlit faces"
    );
}
