use crate::{
    render_effects, render_mesh,
    renderer::{self, Renderer},
    session::Options,
    source::FileSource,
};
use std::{fs::File, io::BufWriter, path::Path};
use tenchu_assets::{Result, game::GameAssets};

pub fn render(assets: &GameAssets<FileSource>, destination: &Path) -> Result<()> {
    let mut game = Options::default().load(assets)?;
    for _ in 0..30 {
        game.step(0)?;
    }
    frame(&game, destination)
}

pub fn frame(game: &tenchu_core::game::Game, destination: &Path) -> Result<()> {
    let mut vertices = render_mesh::stage(&game.mission, &game.math);
    vertices.extend(render_mesh::characters(game));
    let instance = wgpu::Instance::new(&Default::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok_or("No graphics adapter")?;
    eprintln!(
        "Renderer: {} ({:?})",
        adapter.get_info().name,
        adapter.get_info().backend
    );
    let [width, height] = [1280, 720];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        format,
        [width, height],
        &game.mission.vram,
    ))?;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    renderer.draw(
        &texture.create_view(&Default::default()),
        renderer::game_camera(game.rendered_view, width as f32 / height as f32),
        &[&renderer.upload(&vertices)],
        Some(&renderer.upload_effects(render_effects::build(game))),
    );
    read_texture(&renderer, &texture, destination)
}

pub fn read_texture(
    renderer: &Renderer,
    texture: &wgpu::Texture,
    destination: &Path,
) -> Result<()> {
    let [width, height] = [texture.width(), texture.height()];
    let bgra = match texture.format() {
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => true,
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => false,
        format => return Err(format!("Screenshot format {format:?} is unsupported")),
    };
    let stride = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Capture readback"),
        size: u64::from(stride) * u64::from(height),
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
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = send.send(result);
        });
    renderer.device.poll(wgpu::Maintain::Wait);
    receive
        .recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mapped = buffer.slice(..).get_mapped_range();
    let mut pixels: Vec<_> = mapped
        .chunks_exact(stride as usize)
        .flat_map(|row| row[..width as usize * 4].iter().copied())
        .collect();
    if bgra {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    let file = BufWriter::new(File::create(destination).map_err(|e| e.to_string())?);
    let mut png = png::Encoder::new(file, width, height);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .map_err(|e| e.to_string())?
        .write_image_data(&pixels)
        .map_err(|e| e.to_string())?;
    eprintln!("Saved {}", destination.display());
    Ok(())
}
