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
            assert!(scroll.water);
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
                role: crate::visuals::catalog::Role::Terrain,
                hero_part: None,
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

#[test]
#[ignore = "requires JP or EU TENCHU2_DISC; identifies source water without reclassifying terrain or altering texture animation"]
fn imported_water_materials_drive_shared_rendering_in_every_water_map() {
    use crate::{
        render_mesh,
        settings::Graphics,
        visuals::{catalog, models::Meshes},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for map in [0, 9, 16, 17, 18, 19] {
        let level = Level::load(&disc, map).unwrap();
        let source_vram = postcard::to_allocvec(&level.vram).unwrap();
        let mut water_vertices = 0;
        for enabled in [false, true] {
            let mut graphics = Graphics {
                asset_pack: false,
                ..Default::default()
            };
            graphics.water.enabled = enabled;
            let meshes = Meshes::build(
                &level.vram,
                &Default::default(),
                &graphics,
                level
                    .map
                    .iter()
                    .enumerate()
                    .map(|(index, part)| catalog::ModelSource {
                        id: format!("tenchu2/map{map}/terrain/{index}"),
                        role: catalog::Role::Terrain,
                        hero_part: None,
                        label: String::new(),
                        geometry: &part.geometry,
                    })
                    .collect(),
                true,
            )
            .unwrap();
            let mut dry_vertices = 0;
            for part in &level.map {
                meshes.visit(
                    &part.geometry,
                    Default::default(),
                    false,
                    0,
                    None,
                    |triangle| {
                        for vertex in triangle {
                            let source_water = vertex
                                .material
                                .checked_sub(1)
                                .and_then(|i| meshes.materials()[i as usize].animation)
                                .is_some_and(|animation| animation.water);
                            assert_eq!(
                                vertex.flags & render_mesh::WATER != 0,
                                source_water && enabled
                            );
                            if source_water {
                                water_vertices += 1;
                            } else {
                                dry_vertices += 1;
                            }
                        }
                    },
                );
            }
            assert!(dry_vertices > 0);
        }
        assert!(water_vertices > 0, "map {map}: source water was lost");
        assert_eq!(postcard::to_allocvec(&level.vram).unwrap(), source_vram);
        let decoded: tenchu_assets::texture::Vram = postcard::from_bytes(&source_vram).unwrap();
        assert_eq!(decoded.animations, level.vram.animations);
        eprintln!("map {map}: {water_vertices} source water vertices");
    }
}
