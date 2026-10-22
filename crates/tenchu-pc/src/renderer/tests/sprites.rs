use super::*;
use crate::visuals::{catalog, pack::Pack, pixels::Image};
use std::{collections::BTreeMap, sync::Arc};

#[test]
#[ignore = "requires a graphics adapter; world sprite overrides preserve holes/blending and never replace a shared HUD image"]
fn world_sprite_replacements_keep_hud_and_blend_classes() {
    let mut vram = Vram::default();
    let source = tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0; 2],
        size: [4, 2],
    };
    vram.images.push(source);
    for row in 0..2 {
        vram.words[row * 1024..row * 1024 + 4].copy_from_slice(&[0, 31, 0x801f, 0x801f]);
    }
    let region = catalog::Region::sprite(&vram, source);
    let id = catalog::image_id(source.size, &region.pixels(&vram));
    let replacement = |rgb: [u8; 3]| Pack {
        textures: BTreeMap::from([(
            id.clone(),
            Arc::new(Image {
                size: [32, 16],
                rgba: (0..16)
                    .flat_map(|_| {
                        (0..32).flat_map(|x| {
                            [
                                rgb[0],
                                rgb[1],
                                rgb[2],
                                match x {
                                    0..8 => 0,
                                    8..16 => 255,
                                    _ => 128,
                                },
                            ]
                        })
                    })
                    .collect(),
            }),
        )]),
        ..Default::default()
    };
    let mut sprites = Vec::new();
    for (offset, ui) in [(-60.0, false), (60.0, true)] {
        let mut vertices = rectangle(
            1000,
            128,
            Some(Texture {
                page: 256,
                palette: 0,
            }),
            true,
            false,
            true,
        );
        for vertex in &mut vertices {
            vertex.uv = [
                (vertex.position[0] + 600.0) / 1200.0 * 3.0,
                (vertex.position[1] + 600.0) / 1200.0,
            ];
            vertex.position = [
                vertex.position[0] / 10.0 + offset,
                vertex.position[1] / 5.0,
                0.1,
            ];
            vertex.flags = render_mesh::TEXTURED
                | render_mesh::SCREEN
                | render_mesh::TRANSLUCENT
                | if ui { render_mesh::UI } else { 0 };
        }
        sprites.extend(vertices);
    }
    let (mut renderer, target) = fixture(&vram);
    let background = renderer.upload(&rectangle(1000, 96, None, false, false, true));
    let effects = renderer.upload_effects(render_effects::Geometry {
        batches: vec![render_effects::Batch {
            blend: 1,
            range: 0..sprites.len() as u32,
        }],
        vertices: sprites,
    });
    let draw = |renderer: &mut Renderer| {
        renderer
            .draw(
                &target.create_view(&Default::default()),
                game_camera(Matrix::default(), 1.0),
                &[&background],
                Some(&effects),
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let at = |pixels: &[u8], x: usize| -> [u8; 4] {
        pixels[32 * 256 + x * 4..32 * 256 + x * 4 + 4]
            .try_into()
            .unwrap()
    };
    let mut graphics = crate::settings::Graphics::default();
    for filtering in [0, 2, 4] {
        graphics.set_filtering(filtering);
        graphics.asset_pack = false;
        renderer.configure(&graphics);
        let original = draw(&mut renderer);
        renderer.set_asset_pack(&replacement([0, 255, 0]));
        assert_eq!(
            draw(&mut renderer),
            original,
            "disabled pack at filtering {filtering}"
        );
        graphics.asset_pack = true;
        renderer.configure(&graphics);
        let green = draw(&mut renderer);
        assert_eq!(at(&green, 4), at(&original, 4), "cutout remains empty");
        assert_eq!(at(&green, 14), [0, 255, 0, 255], "opaque sprite texel");
        let blended = at(&green, 27);
        assert!(
            blended[1] > blended[0] && blended[1] < 255 && blended[0] > 0,
            "semi-transparent sprite keeps its blend: {blended:?}"
        );
        for y in 0..64 {
            assert_eq!(
                &green[y * 256 + 128..(y + 1) * 256],
                &original[y * 256 + 128..(y + 1) * 256],
                "shared HUD remains original"
            );
        }
        renderer.set_asset_pack(&replacement([0, 0, 255]));
        assert_eq!(
            at(&draw(&mut renderer), 14),
            [0, 0, 255, 255],
            "live reload"
        );
        graphics.asset_pack = false;
        renderer.configure(&graphics);
        assert_eq!(draw(&mut renderer), original, "exact original restoration");
    }
}
