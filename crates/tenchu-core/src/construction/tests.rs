use super::*;

#[test]
fn exact_negative_cell_boundaries_keep_the_original_preceding_cell() {
    assert_eq!(
        [-16001, -16000, -15999, -1, 0, 15999, 16000].map(cell),
        [-2, -2, -1, -1, 0, 0, 1]
    );
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_SCENE_TRACE; compares constructed scenery with retail RAM"]
fn construction_coordinates_geometry_and_cell_order_match_retail() {
    use std::{fs::File, io::Read};
    let mut capture = vec![0; 96 + 0x200000];
    File::open(std::env::var_os("TENCHU_SCENE_TRACE").expect("Set TENCHU_SCENE_TRACE"))
        .unwrap()
        .read_exact(&mut capture)
        .unwrap();
    assert!(matches!(&capture[..8], b"ONKTR001" | b"ONKTR002"));
    assert_eq!(&capture[16..24], b"ONKRAM01");
    let ram = &capture[96..];
    let bytes = |p: u32, n: usize| &ram[(p & 0x1fffff) as usize..][..n];
    let word = |p| u32::from_le_bytes(bytes(p, 4).try_into().unwrap());
    let short = |p| i16::from_le_bytes(bytes(p, 2).try_into().unwrap());
    let matrix = |p| Matrix {
        rotation: std::array::from_fn(|i| {
            std::array::from_fn(|j| short(p + (i * 3 + j) as u32 * 2))
        }),
        translation: std::array::from_fn(|i| word(p + 20 + i as u32 * 4) as i32),
    };
    let mut game = crate::oracle::game(word(0x80097ac4) as usize, short(word(0x80097c60)));
    assert_eq!(game.construction.objects.len(), word(0x8008dbe4) as usize);
    let slots = word(0x8008dbe0);
    for (i, object) in game.construction.objects.iter().enumerate() {
        let slot = slots + i as u32 * 12;
        let model = word(slot + 4);
        assert_eq!(
            game.scene.models[object.model].local,
            matrix(model + 4),
            "object {i} transform"
        );
        assert_eq!(
            (object.size, object.shift_y),
            (short(slot + 8), short(slot + 10)),
            "object {i} bounds"
        );
        assert_eq!(
            u32::from(object.subdivisions),
            word(model + 0x50) >> 9 & 7,
            "object {i} subdivisions"
        );
        let tmd = word(model + 0x58);
        let mesh = game.construction.geometry(object, &game.mission).unwrap();
        assert_eq!(
            mesh.vertices.len(),
            word(tmd + 4) as usize,
            "object {i} vertices"
        );
        assert_eq!(
            mesh.faces.len(),
            word(tmd + 20) as usize,
            "object {i} faces"
        );
        let vertices = word(tmd);
        for (j, vertex) in mesh.vertices.iter().enumerate() {
            assert_eq!(
                *vertex,
                std::array::from_fn(|k| short(vertices + j as u32 * 8 + k as u32 * 2)),
                "object {i} vertex {j}"
            );
        }
    }
    for (i, cell) in game.construction.cells.iter().enumerate() {
        let mut pointer = word(0x8008dbec + i as u32 * 4);
        let mut ids = Vec::new();
        while pointer != 0 {
            assert!(ids.len() < game.construction.objects.len());
            ids.push(((pointer - slots) / 12) as usize);
            pointer = word(pointer);
        }
        assert_eq!(
            cell.iter().copied().rev().collect::<Vec<_>>(),
            ids,
            "world cell {i}"
        );
    }
    game.camera.position = std::array::from_fn(|i| word(0x800b8080 + i as u32 * 4) as i32);
    game.camera.view = matrix(0x800c6520);
    let bytes = game
        .construction
        .packets(
            &game.mission,
            &mut game.scene,
            &game.camera,
            &game.drawing.subdivision,
        )
        .unwrap();
    assert!(bytes > 0);
    println!(
        "{} retail construction objects; {bytes} scenery packet bytes at captured view",
        game.construction.objects.len()
    );
}
