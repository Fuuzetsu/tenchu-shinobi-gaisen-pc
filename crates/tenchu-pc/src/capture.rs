use crate::{
    render_effects, render_mesh,
    renderer::{self, Renderer},
    session::Options,
    source::FileSource,
};
use std::{fs::File, io::BufWriter, path::Path};
use tenchu_assets::{Result, game::GameAssets};

pub fn render(assets: &GameAssets<FileSource>, destination: &Path, options: Options) -> Result<()> {
    let mut game = options.load(assets)?;
    for _ in 0..30 {
        game.step(0)?;
    }
    frame(&game, destination)
}

pub fn frame(game: &tenchu_core::game::Game, destination: &Path) -> Result<()> {
    let settings = if std::env::var_os("TENCHU_SETTINGS").is_some() {
        crate::settings::Store::new()?.load()?
    } else {
        crate::settings::Settings::default()
    };
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
    let pack = crate::visuals::pack::Pack::from_env()?;
    let directory =
        std::env::var("TENCHU_MODEL_DIR").unwrap_or_else(|_| settings.model_directory.clone());
    let bodies = if directory.is_empty() {
        crate::visuals::pack::Pack::default()
    } else {
        crate::visuals::pack::Pack::load(std::path::Path::new(&directory))?
    };
    let mut models =
        crate::visuals::models::Models::build(&game.mission, &pack, &settings.graphics)?
            .with_player(
                game,
                &settings.player_model,
                &pack,
                &bodies,
                &settings.graphics,
            )?;
    let emitters =
        crate::visuals::lights::Emitters::build(game, &pack, settings.graphics.asset_pack);
    let lights: Vec<_> = crate::lighting::sources(
        game,
        &settings.graphics.lighting,
        crate::debug::camera::Camera::from_game(game).position,
        None,
        emitters.sources(game),
    )
    .into_iter()
    .map(|source| source.light)
    .collect();
    renderer.configure(&settings.graphics);
    renderer.set_asset_pack(&pack);
    renderer.set_emitters(&emitters);
    renderer.bind_models(&mut models);
    let mut vertices = render_mesh::stage(&game.mission, &game.math, Some(&models));
    vertices.extend(render_mesh::characters(game, Some(&models)));
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
    let mesh = renderer.upload(&vertices);
    let casters = if settings.graphics.lighting.enabled && settings.graphics.lighting.shadows {
        let mut vertices = render_mesh::shadow_stage(&game.mission, &game.math, Some(&models));
        vertices.extend(render_mesh::shadow_models(game, Some(&models)));
        Some(renderer.upload(&vertices))
    } else {
        None
    };
    let effects = renderer.upload_effects(render_effects::build_options(
        game,
        settings.show_hud,
        None,
        settings.graphics.blob_shadows,
        Some(&models),
    ));
    renderer.draw_lit(
        &texture.create_view(&Default::default()),
        renderer::game_camera(game.rendered_view, width as f32 / height as f32),
        &[&mesh],
        Some(&effects),
        &lights,
        &casters.iter().collect::<Vec<_>>(),
    )?;
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
    let readback = Readback::new(renderer, texture);
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    readback.read(renderer, texture, |mapped, stride| {
        for row in mapped.chunks_exact(stride) {
            pixels.extend_from_slice(&row[..width as usize * 4]);
        }
        Ok(())
    })?;
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

pub struct Readback {
    buffer: wgpu::Buffer,
    stride: u32,
}

impl Readback {
    pub fn new(renderer: &Renderer, texture: &wgpu::Texture) -> Self {
        let stride = (texture.width() * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        Self {
            buffer: renderer.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Capture readback"),
                size: u64::from(stride) * u64::from(texture.height()),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            stride,
        }
    }

    pub fn read(
        &self,
        renderer: &Renderer,
        texture: &wgpu::Texture,
        consume: impl FnOnce(&[u8], usize) -> Result<()>,
    ) -> Result<()> {
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: Some(texture.height()),
                },
            },
            texture.size(),
        );
        renderer.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        renderer.device.poll(wgpu::Maintain::Wait);
        receive
            .recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let mapped = self.buffer.slice(..).get_mapped_range();
        let result = consume(&mapped, self.stride as usize);
        drop(mapped);
        self.buffer.unmap();
        result
    }
}
