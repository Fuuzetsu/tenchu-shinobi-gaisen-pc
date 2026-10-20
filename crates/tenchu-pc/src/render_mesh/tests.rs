use super::*;
use tenchu_assets::game::GameAssets;

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
    append_geometry(&mut vertices, &[mesh.clone()], Matrix::default());
    assert_eq!(vertices.len(), 39);
    mesh.faces.push(mesh.faces[0].clone());
    vertices.clear();
    append_geometry(&mut vertices, &[mesh], Matrix::default());
    assert_eq!(
        vertices.len(),
        39,
        "DrawTMD stops at the lit face, including any later unlit faces"
    );
}
