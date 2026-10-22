use crate::{
    renderer::{EffectMesh, Mesh, Renderer},
    settings::Graphics,
    visuals::pack::Pack,
};
use glam::Mat4;
use std::{fs, path::Path};
use tenchu_assets::{Result, texture::Vram};

pub const SIZE: [u32; 2] = [1280, 720];

pub fn prepare_directory(output: &Path) -> Result<()> {
    if output.exists()
        && fs::read_dir(output)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("Preview output must be an empty directory".into());
    }
    fs::create_dir_all(output).map_err(|e| e.to_string())
}

pub struct Review {
    pub renderer: Renderer,
    texture: wgpu::Texture,
    graphics: Graphics,
}

impl Review {
    pub fn backdrop(&mut self, shade: f32) -> Mesh {
        self.renderer.upload(&crate::render_effects::quad(
            [
                [-1000.0, -120.0, 0.999999],
                [1000.0, -120.0, 0.999999],
                [-1000.0, 120.0, 0.999999],
                [1000.0, 120.0, 0.999999],
            ],
            [[shade; 3]; 4],
            None,
            0,
        ))
    }

    pub fn new(vram: &Vram) -> Result<Self> {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .ok_or("No graphics adapter")?;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer = pollster::block_on(Renderer::new(&adapter, format, SIZE, vram))?;
        renderer.set_asset_pack(&Pack::from_env()?);
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Asset review"),
            size: wgpu::Extent3d {
                width: SIZE[0],
                height: SIZE[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let graphics = if std::env::var_os("TENCHU_SETTINGS").is_some() {
            crate::settings::Store::new()?.load()?.graphics
        } else {
            let mut graphics = Graphics {
                fog: false,
                ..Default::default()
            };
            graphics.set_filtering(4);
            graphics
        };
        Ok(Self {
            renderer,
            texture,
            graphics,
        })
    }

    pub fn capture(
        &mut self,
        camera: Mat4,
        meshes: &[&Mesh],
        effects: Option<&EffectMesh>,
        enabled: bool,
        output: &Path,
    ) -> Result<()> {
        self.graphics.asset_pack = enabled;
        self.renderer.configure(&self.graphics);
        self.renderer.draw_lit(
            &self.texture.create_view(&Default::default()),
            camera,
            meshes,
            effects,
            &[],
            meshes,
        )?;
        super::read_texture(&self.renderer, &self.texture, output)
    }
}
