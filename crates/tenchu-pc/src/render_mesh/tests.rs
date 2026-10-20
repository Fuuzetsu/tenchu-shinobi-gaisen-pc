use super::*;
use tenchu_assets::game::GameAssets;

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
        visit_hinged(&game, |geometry, matrix| {
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
            visit_hinged(game, |geometry, matrix| {
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
