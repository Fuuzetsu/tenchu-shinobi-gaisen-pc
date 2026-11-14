use super::*;

#[test]
#[ignore = "requires a GPU; shared external images must avoid rebinding work while edits, artwork switches and animations remain current"]
fn shared_external_images_keep_updates_and_artwork_switches() {
    let (mut renderer, _) = crate::renderer::tests::fixture(&Vram::default());
    let mut source = crate::visuals::models::Material {
        id: "custom-terrain".into(),
        region: None,
        image: Arc::new(Image {
            size: [8, 8],
            rgba: [200, 10, 20, 255].repeat(64),
        }),
        animation: None,
    };
    let flush = |renderer: &mut crate::renderer::Renderer| {
        renderer
            .materials
            .flush(&renderer.device, &renderer.queue, 0.0)
            .unwrap();
        assert_atlas_pixels(renderer);
    };
    let handle = renderer.materials.bind(&source);
    flush(&mut renderer);
    let image = renderer.materials.entries[handle as usize - 1]
        .image
        .rgba
        .as_ptr();
    for _ in 0..10 {
        assert_eq!(renderer.materials.bind(&source), handle);
        assert_eq!(
            renderer.materials.entries[handle as usize - 1]
                .image
                .rgba
                .as_ptr(),
            image
        );
        assert!(
            !renderer
                .materials
                .flush(&renderer.device, &renderer.queue, 0.0)
                .unwrap()
        );
    }
    Arc::make_mut(&mut source.image).rgba = [0, 200, 0, 255].repeat(64);
    assert_eq!(renderer.materials.bind(&source), handle);
    assert_eq!(
        renderer.materials.entries[handle as usize - 1].image.rgba,
        source.image.rgba
    );
    flush(&mut renderer);

    renderer.materials.set_pack(&crate::visuals::pack::Pack {
        textures: [(
            source.id.clone(),
            Arc::new(Image {
                size: [8, 8],
                rgba: [0, 0, 200, 255].repeat(64),
            }),
        )]
        .into(),
        ..Default::default()
    });
    for artwork in [true, false, true] {
        renderer.materials.configure(&crate::settings::Graphics {
            asset_pack: artwork,
            ..Default::default()
        });
        assert_eq!(renderer.materials.bind(&source), handle);
        assert_eq!(
            renderer.materials.entries[handle as usize - 1].image.rgba,
            if artwork {
                [0, 0, 200, 255].repeat(64)
            } else {
                source.image.rgba.clone()
            }
        );
        flush(&mut renderer);
    }
    source.animation = Some(tenchu_assets::texture::TextureScroll {
        image: tenchu_assets::texture::Image {
            page: 0,
            palette: 0,
            uv: [0, 0],
            size: [8, 8],
        },
        velocity: [1, -1],
        delay: 0,
        water: true,
    });
    let animated = renderer.materials.bind(&source);
    assert_ne!(animated, handle);
    flush(&mut renderer);
    renderer
        .materials
        .flush(&renderer.device, &renderer.queue, 2.0 / 30.0)
        .unwrap();
    assert_eq!(renderer.materials.bind(&source), animated);
    assert_eq!(
        renderer.materials.entries[animated as usize - 1].phase,
        [0.25, 0.75]
    );
    assert_atlas_pixels(&renderer);
}

#[test]
#[ignore = "requires a GPU; refined blood is cached separately from artwork, refreshes on VRAM changes and never tags foliage"]
fn original_hd_blood_materials_are_isolated_and_cached() {
    let mut vram = Vram::default();
    vram.words[5 * 1024 + 5] = 31;
    let image = tenchu_assets::texture::Image {
        page: 256,
        palette: 0,
        uv: [0, 0],
        size: [16, 16],
    };
    vram.images.push(image);
    let (mut renderer, _) = crate::renderer::tests::fixture(&vram);
    let region = Region::sprite(&vram, image);
    let regular = renderer.materials.register(region);
    let mut vertices = crate::render_effects::quad([[0.0; 3]; 4], [[1.0; 3]; 4], Some(image), 0);
    for vertex in &mut vertices {
        vertex.flags |= render_mesh::STAIN | render_mesh::BLOOD_HD;
    }
    let mut annotated = vertices.clone();
    renderer.materials.annotate(&mut annotated);
    let refined = annotated[0].material;
    assert_ne!(regular, refined);
    renderer
        .materials
        .flush(&renderer.device, &renderer.queue, 0.0)
        .unwrap();
    let pixels = renderer.materials.entries[refined as usize - 1]
        .image
        .rgba
        .as_ptr();
    for _ in 0..30 {
        annotated.clone_from(&vertices);
        renderer.materials.annotate(&mut annotated);
        assert_eq!(annotated[0].material, refined);
        assert!(
            !renderer
                .materials
                .flush(&renderer.device, &renderer.queue, 0.0)
                .unwrap()
        );
        assert_eq!(
            renderer.materials.entries[refined as usize - 1]
                .image
                .rgba
                .as_ptr(),
            pixels
        );
    }
    let id = renderer.materials.entries[regular as usize - 1].id.clone();
    renderer.materials.set_pack(&crate::visuals::pack::Pack {
        textures: [(
            id,
            Arc::new(Image {
                size: [16, 16],
                rgba: [0, 255, 0, 255].repeat(256),
            }),
        )]
        .into(),
        ..Default::default()
    });
    assert_eq!(
        renderer.materials.entries[regular as usize - 1].image.rgba[1],
        255
    );
    assert!(
        renderer.materials.entries[refined as usize - 1]
            .image
            .rgba
            .chunks_exact(4)
            .all(|p| p[1] == 0)
    );
    vram.words[5 * 1024 + 5] = 0;
    renderer.materials.update(&vram);
    assert!(
        renderer.materials.entries[refined as usize - 1]
            .image
            .rgba
            .iter()
            .all(|&n| n == 0)
    );
    for vertex in &mut vertices {
        vertex.flags |= render_mesh::FOLIAGE | 255 << render_mesh::WIND_WEIGHT_SHIFT;
        assert!(!render_mesh::original_hd_blood(vertex.flags));
    }
}

#[test]
fn animation_changes_include_packed_pixels_and_wrapping_palettes() {
    let before = Vram::default();
    let four_bit = Region {
        page: 15,
        palette: (480 << 6) | 63,
        rect: [1, 10, 3, 2],
    };
    let eight_bit = Region {
        page: 0x8f,
        ..four_bit
    };
    let direct = Region {
        page: 0x10f,
        ..four_bit
    };
    for (x, y, affected) in [
        (960, 10, [true, true, false]),
        (962, 11, [false, false, true]),
        (1008, 480, [true, true, false]),
        (0, 480, [false, true, false]),
        (240, 480, [false, false, false]),
        (960, 12, [false, false, false]),
    ] {
        let mut after = before.clone();
        after.words[y * 1024 + x] = 0x83e0;
        let changes = Changes::between(&before, &after);
        assert_eq!(
            [four_bit, eight_bit, direct].map(|r| changes.affects(r)),
            affected,
            "word ({x}, {y})",
        );
    }
}

#[test]
#[ignore = "requires a GPU; checks incremental uploads, atlas/metadata growth, mip gutters and live updates"]
fn late_materials_preserve_existing_gpu_pixels_and_coordinates() {
    let mut vram = Vram::default();
    let regions: Vec<_> = (0..70)
        .map(|i| Region {
            page: 256 + i / 64 * 4,
            palette: 0,
            rect: [i % 8 * 32, i % 64 / 8 * 32, 32, 32],
        })
        .collect();
    for (i, region) in regions.iter().enumerate() {
        let [u, v, width, height] = region.rect.map(usize::from);
        let x = usize::from(region.page & 15) * 64 + u;
        for y in v..v + height {
            for dx in 0..width {
                vram.words[y * 1024 + x + dx] = if y == v {
                    [0, 0x83e0][dx % 2]
                } else {
                    1 + i as u16 * 407
                };
            }
        }
    }
    let (mut renderer, target) = crate::renderer::tests::fixture(&vram);
    for layers in [2, 3] {
        renderer.reset_vram(&vram);
        renderer.materials.configure(&crate::settings::Graphics {
            surface_detail: if layers == 3 { 50 } else { 0 },
            ..Default::default()
        });
        for &region in &regions {
            let materials = &mut renderer.materials;
            let previous: Vec<_> = materials.atlas.materials.iter().map(|m| m.mips).collect();
            let texture = materials.atlas.texture.clone();
            let bindings = materials.bindings.clone();
            let capacity = materials.atlas.data.capacity();
            let id = materials.register(region);
            materials
                .flush(&renderer.device, &renderer.queue, 0.0)
                .unwrap();
            if id > 1 {
                for (old, new) in previous.iter().zip(&materials.atlas.materials) {
                    assert_eq!(
                        *old, new.mips,
                        "existing coordinates moved for material {id}"
                    );
                }
                if materials.atlas.packing.pages() * layers <= texture.depth_or_array_layers() {
                    assert_eq!(texture, materials.atlas.texture);
                    if materials.atlas.materials.len() <= capacity {
                        assert_eq!(bindings, materials.bindings);
                    }
                }
            }
        }
        assert!(
            renderer.materials.atlas.data.capacity() > 64,
            "metadata grew"
        );
        assert!(renderer.materials.atlas.packing.pages() > 2, "atlas grew");
        assert_atlas_pixels(&renderer);

        for index in [0, 63, 69] {
            let region = regions[index];
            let image = tenchu_assets::texture::Image {
                page: region.page,
                palette: region.palette,
                uv: [region.rect[0] as u8, region.rect[1] as u8],
                size: [32, 32],
            };
            let mut vertices = crate::render_effects::quad(
                [
                    [-160.0, -120.0, 0.1],
                    [160.0, -120.0, 0.1],
                    [-160.0, 120.0, 0.1],
                    [160.0, 120.0, 0.1],
                ],
                [[1.0; 3]; 4],
                Some(image),
                0,
            );
            for vertex in &mut vertices {
                vertex.material = index as u32 + 1;
                vertex.flags |= render_mesh::UI;
            }
            let mesh = renderer.upload(&vertices);
            renderer
                .draw(
                    &target.create_view(&Default::default()),
                    glam::Mat4::IDENTITY,
                    &[&mesh],
                    None,
                )
                .unwrap();
            let color = crate::renderer::tests::read_pixel(&renderer, &target);
            let expected = &renderer.materials.entries[index].original.rgba[32 * 4..32 * 4 + 4];
            assert!(
                color.iter().zip(expected).all(|(a, b)| a.abs_diff(*b) <= 1),
                "GPU metadata for material {index}: {color:?}, expected {expected:?}"
            );
        }

        let bindings = renderer.materials.bindings.clone();
        let mut animated = vram.clone();
        animated.words[1024 + 1] = 0x7fe0;
        renderer.materials.update(&animated);
        renderer
            .materials
            .flush(&renderer.device, &renderer.queue, 0.0)
            .unwrap();
        assert_eq!(
            bindings, renderer.materials.bindings,
            "same-size animation only uploads its own tiles"
        );
        assert_atlas_pixels(&renderer);
    }
}

fn read_atlas(
    renderer: &crate::renderer::Renderer,
    texture: &wgpu::Texture,
    channels: u32,
) -> (Vec<u8>, u32) {
    let stride = (texture.width() * channels).div_ceil(256) * 256;
    let size = stride * texture.height() * texture.depth_or_array_layers();
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Incremental atlas regression"),
        size: u64::from(size),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap()
        });
    renderer.device.poll(wgpu::Maintain::Wait);
    receive.recv().unwrap().unwrap();
    let pixels = buffer.slice(..).get_mapped_range().to_vec();
    (pixels, stride)
}

fn assert_atlas_pixels(renderer: &crate::renderer::Renderer) {
    let materials = &renderer.materials;
    let atlas = &materials.atlas;
    let (pixels, stride) = read_atlas(renderer, &atlas.texture, 4);
    for (index, entry) in materials.entries.iter().enumerate() {
        for (mip, image) in entry.mips.iter().enumerate() {
            let rect = atlas.materials[index + 1].mips[mip];
            let x = rect[0] as u32 % atlas.side - 1;
            let y = rect[1] as u32 - 1;
            let layer = rect[0] as u32 / atlas.side * materials.layers as u32;
            let width = (image.size[0] + 2) as usize * 4;
            for class in 0..materials.layers {
                let expected = padded(image.size, entry.pixels(mip, class), entry.repeating());
                for (dy, row) in expected.chunks_exact(width).enumerate() {
                    let offset =
                        (((layer as usize + class) * atlas.side as usize + y as usize + dy)
                            * stride as usize)
                            + x as usize * 4;
                    assert_eq!(
                        &pixels[offset..offset + width],
                        row,
                        "material {index}, mip {mip}, class {class}, row {dy}"
                    );
                }
            }
        }
    }
    let (coverage, stride) = read_atlas(renderer, &atlas.coverage, 1);
    for (index, entry) in materials.entries.iter().enumerate() {
        let rect = atlas.materials[index + 1].mips[0];
        let x = rect[0] as u32 % atlas.side - 1;
        let y = rect[1] as u32 - 1;
        let layer = rect[0] as u32 / atlas.side;
        let source = padded(entry.image.size, &entry.image.rgba, entry.repeating());
        let width = (entry.image.size[0] + 2) as usize;
        for (dy, row) in source.chunks_exact(width * 4).enumerate() {
            for (dx, pixel) in row.chunks_exact(4).enumerate() {
                let offset = ((layer * atlas.side + y + dy as u32) * stride + x) as usize + dx;
                let class = if pixel[3] == 0 {
                    0
                } else if pixel[3] >= 192 {
                    255
                } else {
                    128
                };
                assert_eq!(
                    coverage[offset], class,
                    "coverage of material {index}, ({dx}, {dy})"
                );
            }
        }
    }
}
