use tenchu_assets::tenchu2::level::Level;

#[derive(serde::Deserialize)]
struct Case {
    map: usize,
    tick: u32,
    layer: usize,
    rectangle: [i16; 10],
    moves: Vec<([i16; 4], i16, i16)>,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_TEXTURE_ORACLE; compares scrolling coordinates to original JP GPU commands"]
fn imported_water_scroll_matches_original_texture_moves() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let cases: Vec<Case> = serde_json::from_slice(
        &std::fs::read(std::env::var_os("TENCHU2_TEXTURE_ORACLE").unwrap()).unwrap(),
    )
    .unwrap();
    let mut count = 0;
    for map in [0, 9, 16, 17, 18, 19] {
        let level = Level::load(&disc, map).unwrap();
        assert_eq!(level.vram.animations.len(), 2);
        for case in cases.iter().filter(|c| c.map == map) {
            let scroll = level.vram.animations[case.layer];
            assert_eq!(scroll.velocity, case.rectangle[8..10]);
            assert_eq!(
                scroll.image.word_rect(),
                [384, case.layer as u16 * 64, 16, 64]
            );
            if case.tick < 6 {
                continue;
            }
            let rectangle = case.moves[0].0;
            assert_eq!(
                scroll.offset(case.tick + 1),
                [
                    (rectangle[0] - case.rectangle[6]) as u16,
                    (rectangle[1] - case.rectangle[7]) as u16
                ],
                "map {map}, layer {}, tick {}",
                case.layer,
                case.tick
            );
            count += 1;
        }
        let sources: Vec<_> = level
            .map
            .iter()
            .enumerate()
            .map(|(index, p)| crate::visuals::catalog::ModelSource {
                id: format!("tenchu2/map{map}/terrain/{index}"),
                label: String::new(),
                geometry: &p.geometry,
            })
            .collect();
        let meshes = crate::visuals::models::Meshes::build(
            &level.vram,
            &Default::default(),
            &Default::default(),
            sources,
            true,
        )
        .unwrap();
        assert_eq!(
            meshes
                .materials()
                .iter()
                .filter(|m| m.animation.is_some())
                .count(),
            2
        );
    }
    assert!(count > 1000);
    eprintln!("{count} original texture move comparisons");
}
