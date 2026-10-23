use super::*;
use crate::visuals::{catalog, pack::Pack, pixels::Image};
use std::{collections::BTreeMap, sync::Arc};

#[test]
#[ignore = "requires a graphics adapter; animated replacements scroll, wrap, reload and retain identity when first seen mid-animation"]
fn replacement_scroll_tracks_original_copies_and_restores_live_water() {
    let mut vram = Vram::default();
    let image = tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0; 2],
        size: [8, 4],
    };
    vram.images.push(image);
    for y in 0..4 {
        for x in 0..8 {
            vram.words[y * 1024 + x] = 1 + (y * 8 + x) as u16 * 31;
        }
    }
    for y in 0..2 {
        for x in 0..2 {
            vram.copy_rect(image.word_rect(), [832 + x * 8, 256 + y * 4])
                .unwrap();
        }
    }
    vram.scrolling.push(tenchu_assets::texture::ScrollingImage {
        image,
        source: [832, 256],
        offset: [0; 2],
    });
    let region = catalog::Region::find(
        &vram,
        Texture {
            page: 256,
            palette: 0,
        },
        [0, 0, 7, 3],
    );
    let id = catalog::image_id(image.size, &region.pixels(&vram));
    let pack = Pack {
        textures: BTreeMap::from([(
            id,
            Arc::new(Image {
                size: [32, 16],
                rgba: (0..16)
                    .flat_map(|y| (0..32).flat_map(move |x| [x * 8, y * 16, 128, 255]))
                    .collect(),
            }),
        )]),
        ..Default::default()
    };
    let mut vertices = rectangle(
        1000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        false,
        false,
        false,
    );
    for vertex in &mut vertices {
        vertex.uv = [
            (vertex.position[0] + 600.0) / 1200.0 * 7.0,
            (vertex.position[1] + 600.0) / 1200.0 * 3.0,
        ];
    }
    let (mut renderer, target) = fixture(&vram);
    let mesh = renderer.upload(&vertices);
    let tiled: Vec<_> = [-1.0, 1.0]
        .into_iter()
        .flat_map(|side| {
            vertices.iter().copied().map(move |mut vertex| {
                vertex.position[0] = vertex.position[0] * 0.5 + side * 300.0;
                vertex
            })
        })
        .collect();
    let tiled = renderer.upload(&tiled);
    let camera = game_camera(Matrix::default(), 1.0);
    let draw = |renderer: &mut Renderer| {
        renderer
            .draw(
                &target.create_view(&Default::default()),
                camera,
                &[&mesh],
                None,
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    renderer.set_asset_pack(&pack);
    let mut graphics = crate::settings::Graphics::default();
    for filtering in [0, 2, 4] {
        graphics.set_filtering(filtering);
        graphics.asset_pack = true;
        renderer.configure(&graphics);
        let initial = draw(&mut renderer);
        vram.copy_rect([835, 257, 8, 4], [0, 0]).unwrap();
        renderer.update_vram(&vram);
        let moved = draw(&mut renderer);
        assert_ne!(moved, initial, "scrolling at filtering {filtering}");
        renderer
            .draw(
                &target.create_view(&Default::default()),
                camera,
                &[&tiled],
                None,
            )
            .unwrap();
        let joined = read_pixels(&renderer, &target);
        let a = joined[32 * 256 + 31 * 4];
        let b = joined[32 * 256 + 32 * 4];
        assert!(
            a.abs_diff(b) < 20,
            "adjacent scrolling tiles skip pixels at filtering {filtering}: {a} -> {b}"
        );
        renderer.set_asset_pack(&pack);
        assert_eq!(draw(&mut renderer), moved, "reload preserves phase");

        let (mut late, late_target) = fixture(&vram);
        let late_mesh = late.upload(&vertices);
        late.set_asset_pack(&pack);
        late.configure(&graphics);
        let late_draw = |renderer: &mut Renderer| {
            renderer
                .draw(
                    &late_target.create_view(&Default::default()),
                    camera,
                    &[&late_mesh],
                    None,
                )
                .unwrap();
            read_pixels(renderer, &late_target)
        };
        assert_eq!(
            late_draw(&mut late),
            moved,
            "identity survives late registration"
        );
        graphics.asset_pack = false;
        renderer.configure(&graphics);
        late.set_asset_pack(&Pack::default());
        assert_eq!(
            draw(&mut renderer),
            late_draw(&mut late),
            "disabled pack restores live original"
        );
        assert_ne!(draw(&mut renderer), moved);
        graphics.asset_pack = true;
        renderer.configure(&graphics);
        assert_eq!(draw(&mut renderer), moved);
        vram.copy_rect([832, 256, 8, 4], [0, 0]).unwrap();
        renderer.update_vram(&vram);
        assert_eq!(draw(&mut renderer), initial, "whole-period wrap");
    }

    // Pond meshes rotate and crop the original UVs independently on adjacent faces.
    let mut reversed = vertices.clone();
    for vertex in &mut reversed {
        vertex.uv = [7.0 - vertex.uv[0], 3.0 - vertex.uv[1]];
    }
    let reversed = renderer.upload(&reversed);
    let draw_mesh = |renderer: &mut Renderer, mesh: &Mesh| {
        renderer
            .draw(
                &target.create_view(&Default::default()),
                camera,
                &[mesh],
                None,
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    assert_ne!(draw(&mut renderer), draw_mesh(&mut renderer, &reversed));
    graphics.asset_pack = false;
    renderer.configure(&graphics);
    let original = draw(&mut renderer);
    let mut projected = pack;
    projected.surfaces.insert(
        projected.textures.keys().next().unwrap().clone(),
        crate::visuals::relief::Surface {
            height: None,
            strength: 0.0,
            world_tile: Some([1200.0, 1200.0]),
            uv_tile: false,
        },
    );
    renderer.set_asset_pack(&projected);
    for filtering in [0, 2, 4] {
        graphics.asset_pack = true;
        graphics.set_filtering(filtering);
        renderer.configure(&graphics);
        let continuous = draw(&mut renderer);
        assert_eq!(
            continuous,
            draw_mesh(&mut renderer, &reversed),
            "world mapping ignores UV orientation"
        );
        assert_eq!(
            continuous,
            draw_mesh(&mut renderer, &tiled),
            "world mapping ignores UV patch boundaries"
        );
        vram.copy_rect([835, 257, 8, 4], [0, 0]).unwrap();
        renderer.update_vram(&vram);
        let moved = draw(&mut renderer);
        assert_ne!(
            moved, continuous,
            "world mapping retains the original scroll phase"
        );
        assert_eq!(moved, draw_mesh(&mut renderer, &reversed));
        renderer.set_asset_pack(&projected);
        assert_eq!(
            moved,
            draw(&mut renderer),
            "projected reload preserves phase"
        );
        vram.copy_rect([832, 256, 8, 4], [0, 0]).unwrap();
        renderer.update_vram(&vram);
        assert_eq!(continuous, draw(&mut renderer));
    }
    graphics.asset_pack = false;
    renderer.configure(&graphics);
    assert_eq!(
        original,
        draw(&mut renderer),
        "world mapping disables with the artwork"
    );
}

#[test]
#[ignore = "requires a graphics adapter; static replacement tiles must join and restore original UVs"]
fn static_replacement_tiles_repeat_without_skipping_border_texels() {
    let mut vram = Vram::default();
    vram.images.push(tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0; 2],
        size: [8, 4],
    });
    for y in 0..4 {
        for x in 0..8 {
            vram.words[y * 1024 + x] = 1 + (y * 8 + x) as u16 * 31;
        }
    }
    let region = catalog::Region::find(
        &vram,
        Texture {
            page: 256,
            palette: 0,
        },
        [0, 0, 7, 3],
    );
    let id = catalog::image_id([8, 4], &region.pixels(&vram));
    // This continuous periodic ramp changes quickly near the join, exposing an omitted source texel.
    let image = Arc::new(Image {
        size: [64, 32],
        rgba: (0..32)
            .flat_map(|_| {
                (0..64).flat_map(|x| {
                    let value =
                        (128.0 + 100.0 * (x as f32 * std::f32::consts::TAU / 64.0).sin()) as u8;
                    [value, value, value, 255]
                })
            })
            .collect(),
    });
    let mut pack = Pack {
        textures: BTreeMap::from([(id.clone(), image)]),
        surfaces: BTreeMap::from([(
            id.clone(),
            crate::visuals::relief::Surface {
                height: None,
                strength: 0.0,
                world_tile: None,
                uv_tile: true,
            },
        )]),
        ..Default::default()
    };
    let vertices = rectangle(
        1000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        false,
        false,
        false,
    );
    let tiled: Vec<_> = [-1.0, 1.0]
        .into_iter()
        .flat_map(|side| {
            vertices.iter().copied().map(move |mut v| {
                v.uv = [
                    (v.position[0] + 600.0) / 1200.0 * 7.0,
                    (v.position[1] + 600.0) / 1200.0 * 3.0,
                ];
                v.position[0] = v.position[0] * 0.5 + side * 300.0;
                v
            })
        })
        .collect();
    let (mut renderer, target) = fixture(&vram);
    let mesh = renderer.upload(&tiled);
    let draw = |renderer: &mut Renderer| {
        renderer
            .draw(
                &target.create_view(&Default::default()),
                game_camera(Matrix::default(), 1.0),
                &[&mesh],
                None,
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut graphics = crate::settings::Graphics::default();
    for filtering in [0, 2, 4] {
        graphics.asset_pack = false;
        graphics.set_filtering(filtering);
        renderer.configure(&graphics);
        let original = draw(&mut renderer);
        renderer.set_asset_pack(&pack);
        graphics.asset_pack = true;
        renderer.configure(&graphics);
        let joined = draw(&mut renderer);
        let a = joined[32 * 256 + 31 * 4];
        let b = joined[32 * 256 + 32 * 4];
        assert!(
            a.abs_diff(b) < 30,
            "static tiles skip border texels at filtering {filtering}: {a} -> {b}"
        );
        pack.surfaces.get_mut(&id).unwrap().uv_tile = false;
        renderer.set_asset_pack(&pack);
        assert_ne!(
            joined,
            draw(&mut renderer),
            "reload must update tiling without changing the PNG"
        );
        pack.surfaces.get_mut(&id).unwrap().uv_tile = true;
        renderer.set_asset_pack(&pack);
        assert_eq!(joined, draw(&mut renderer));
        graphics.asset_pack = false;
        renderer.configure(&graphics);
        assert_eq!(
            original,
            draw(&mut renderer),
            "disabled pack preserves original UVs"
        );
    }
}
