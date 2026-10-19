use super::*;
use crate::render_mesh;
use tenchu_assets::model::{Corner, Face, Geometry, Texture};
use tenchu_core::math::Matrix;

fn rectangle(
    z: i16,
    color: u8,
    texture: Option<Texture>,
    blend: bool,
    reversed: bool,
    double_sided: bool,
) -> Vec<Vertex> {
    let mut corners = (0..4)
        .map(|vertex| Corner {
            vertex,
            normal: None,
            color: [color; 3],
            uv: [0; 2],
        })
        .collect::<Vec<_>>();
    if reversed {
        corners.swap(1, 2);
    }
    let face = Face {
        mode: 0x28,
        flags: 1,
        corners,
        texture,
        double_sided,
        translucent: blend,
    };
    let geometry = Geometry {
        vertices: vec![
            [-600, -600, z],
            [600, -600, z],
            [-600, 600, z],
            [600, 600, z],
        ],
        normals: vec![],
        faces: vec![face],
    };
    let mut vertices = Vec::new();
    render_mesh::append_face(
        &mut vertices,
        &geometry,
        &geometry.faces[0],
        Matrix::default(),
    );
    vertices
}

fn read_pixel(renderer: &Renderer, target: &wgpu::Texture) -> [u8; 4] {
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Renderer regression readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            origin: wgpu::Origin3d { x: 24, y: 24, z: 0 },
            ..target.as_image_copy()
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap();
        });
    renderer.device.poll(wgpu::Maintain::Wait);
    receive.recv().unwrap().unwrap();
    let pixels = buffer.slice(..).get_mapped_range();
    pixels[..4].try_into().unwrap()
}

#[test]
#[ignore = "requires a graphics adapter; renders source winding, texture transparency and all four blend modes offscreen"]
fn winding_and_original_texture_blends_render_correctly() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("No graphics adapter");
    let mut vram = Vram::default();
    let renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        [64; 2],
        &vram,
    ))
    .unwrap();
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Renderer regression"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    for (reversed, double_sided, expected) in [
        (false, false, [255; 3]),
        (true, false, [2, 3, 5]),
        (true, true, [255; 3]),
    ] {
        let mesh = renderer.upload(&rectangle(1000, 255, None, false, reversed, double_sided));
        renderer.draw(&view, camera, &[&mesh], None);
        assert_eq!(
            &read_pixel(&renderer, &target)[..3],
            &expected,
            "reversed {reversed}, double sided {double_sided}"
        );
    }
    let background = renderer.upload(&rectangle(1500, 102, None, false, false, false));
    let foreground = 15 | (15 << 5) | (15 << 10);
    for (blend, pixel, expected) in [
        (0, foreground | 0x8000, 123),
        (1, foreground, 123),
        (1, foreground | 0x8000, 113),
        (2, foreground | 0x8000, 225),
        (3, foreground | 0x8000, 0),
        (4, foreground | 0x8000, 133),
        (1, 0, 102),
        (1, 0x8000, 51),
    ] {
        vram.words[0] = pixel;
        renderer.update_vram(&vram);
        let vertices = rectangle(
            1000,
            128,
            Some(Texture {
                page: 256,
                palette: 0,
            }),
            blend != 0,
            false,
            false,
        );
        let count = vertices.len() as u32;
        let effects = renderer.upload_effects(render_effects::Geometry {
            vertices,
            batches: vec![render_effects::Batch {
                blend,
                range: 0..count,
            }],
        });
        renderer.draw(&view, camera, &[&background], Some(&effects));
        let color = read_pixel(&renderer, &target);
        assert!(
            color[..3]
                .iter()
                .all(|&v| i32::from(v).abs_diff(expected) <= 1),
            "blend {blend}, pixel {pixel:04x}: {color:?}, expected {expected}"
        );
    }
}
