use super::*;

#[test]
#[ignore = "requires a GPU; opaque replacement metadata must not hide cutouts or semi-transparent texels"]
fn visibility_pass_preserves_replacement_blend_classes() {
    use crate::visuals::{catalog, pack::Pack, pixels::Image};
    use std::{collections::BTreeMap, sync::Arc};

    let mut vram = Vram::default();
    let source = tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [4, 2],
    };
    vram.images.push(source);
    for row in 0..2 {
        vram.words[row * 1024..row * 1024 + 4].copy_from_slice(&[0, 31, 0x801f, 0x801f]);
    }
    let id = catalog::image_id(
        source.size,
        &catalog::Region::sprite(&vram, source).pixels(&vram),
    );
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 100;
    graphics.surface_detail = 100;
    for solid in [false, true] {
        renderer.set_asset_pack(&Pack {
            textures: BTreeMap::from([(
                id.clone(),
                Arc::new(Image {
                    size: [32, 16],
                    rgba: (0..16)
                        .flat_map(|_| {
                            (0..32).flat_map(|x| {
                                [
                                    0,
                                    255,
                                    0,
                                    if solid {
                                        255
                                    } else {
                                        [0, 255, 128, 128][x / 8]
                                    },
                                ]
                            })
                        })
                        .collect(),
                }),
            )]),
            ..Default::default()
        });
        for extra_flags in [0, render_mesh::EMITTER, render_mesh::STAIN] {
            let mut vertices = rectangle(
                1000,
                128,
                Some(Texture {
                    page: 256,
                    palette: 0,
                }),
                true,
                false,
                false,
            );
            for v in &mut vertices {
                v.flags |= render_mesh::SCENERY | extra_flags;
                v.uv = [
                    (v.position[0] + 600.0) / 400.0,
                    (v.position[1] + 600.0) / 1200.0,
                ];
            }
            vertices.extend(rectangle(1000, 40, None, false, false, false));
            let mesh = renderer.upload(&vertices);
            let effects = renderer.upload_effects(render_effects::Geometry {
                vertices: vertices[..6].to_vec(),
                batches: vec![render_effects::Batch {
                    blend: 1,
                    range: 0..6,
                }],
            });
            for filtering in [0, 1, 2, 4] {
                graphics.set_filtering(filtering);
                renderer.configure(&graphics);
                renderer.skip_prepass = true;
                renderer
                    .draw(&view, camera, &[&mesh], Some(&effects))
                    .unwrap();
                let expected = read_pixels(&renderer, &target);
                assert!(expected.chunks_exact(4).any(|p| p[..3] != [0; 3]));
                renderer.skip_prepass = false;
                renderer
                    .draw(&view, camera, &[&mesh], Some(&effects))
                    .unwrap();
                assert_eq!(
                    read_pixels(&renderer, &target),
                    expected,
                    "solid {solid}, extra flags {extra_flags}, filtering {filtering}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a GPU; depth rejection must preserve cutouts, coplanar order, backfaces and blended effects"]
fn visibility_pass_preserves_ordered_surfaces_and_effects() {
    let mut vram = Vram::default();
    for y in 0..16 {
        for x in 0..16 {
            vram.words[y * 1024 + x] = [0, 0x03e0, 0x801f][(x / 3 + y / 4) % 3];
        }
    }
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut cutout = rectangle(
        1000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        true,
        false,
        false,
    );
    for v in &mut cutout {
        v.uv = [
            (v.position[0] + 600.0) / 80.0,
            (v.position[1] + 600.0) / 80.0,
        ];
    }
    let mut vertices = cutout.clone();
    vertices.extend(rectangle(1000, 200, None, false, false, false));
    vertices.extend(rectangle(1000, 40, None, false, false, false));
    vertices.extend(rectangle(800, 255, None, false, true, false));
    let mut alpha = rectangle(900, 80, None, false, false, false);
    for v in &mut alpha {
        v.flags |= render_mesh::ALPHA;
        v.opacity = 0.4;
        v.position[0] -= 400.0;
    }
    vertices.extend_from_slice(&alpha);
    let terrain = renderer.upload(&vertices);
    let mut actor = renderer.upload(&[]);
    let mut effects = render_effects::Geometry {
        vertices: cutout,
        batches: vec![render_effects::Batch {
            blend: 1,
            range: 0..6,
        }],
    };
    effects.append(render_effects::Geometry {
        vertices: alpha,
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..6,
        }],
    });
    let effects = renderer.upload_effects(effects);
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 100;
    for samples in renderer.sample_counts.clone() {
        graphics.samples = samples;
        for post in [false, true] {
            graphics.post.enabled = post;
            renderer.configure(&graphics);
            for x in [-500.0, 0.0, 500.0] {
                let mut moving = rectangle(750, 100, None, false, false, false);
                for v in &mut moving {
                    v.position[0] = v.position[0] * 0.3 + x;
                    v.position[2] += v.position[1] * 0.2;
                }
                renderer.update_indexed_mesh(&mut actor, &mut indexed_geometry(&moving));
                renderer.skip_prepass = true;
                renderer
                    .draw(&view, camera, &[&terrain, &actor], Some(&effects))
                    .unwrap();
                let expected = read_pixels(&renderer, &target);
                renderer.skip_prepass = false;
                renderer
                    .draw(&view, camera, &[&terrain, &actor], Some(&effects))
                    .unwrap();
                assert_eq!(
                    read_pixels(&renderer, &target),
                    expected,
                    "samples {samples}, HDR {post}, actor {x}"
                );
            }
        }
    }
}
