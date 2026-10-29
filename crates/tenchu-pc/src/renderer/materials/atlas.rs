use super::{Material, data};
use tenchu_assets::Result;

#[derive(Clone, Copy, Default)]
pub(super) struct Packing {
    x: u32,
    y: u32,
    row_height: u32,
    page: u32,
}

impl Packing {
    pub fn place(&mut self, side: u32, [width, height]: [u32; 2]) -> [f32; 4] {
        if self.x + width + 2 > side {
            self.x = 0;
            self.y += self.row_height;
            self.row_height = 0;
        }
        if self.y + height + 2 > side {
            self.page += 1;
            self.x = 0;
            self.y = 0;
            self.row_height = 0;
        }
        let rect = [
            (self.x + 1 + self.page * side) as f32,
            (self.y + 1) as f32,
            width as f32,
            height as f32,
        ];
        self.x += width + 2;
        self.row_height = self.row_height.max(height + 2);
        rect
    }

    pub fn pages(&self) -> u32 {
        self.page + 1
    }
}

pub(super) struct Atlas {
    pub texture: wgpu::Texture,
    pub data: data::Data,
    pub materials: Vec<Material>,
    pub side: u32,
    pub packing: Packing,
}

impl Atlas {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        side: u32,
        layers: usize,
        materials: Vec<Material>,
        packing: Packing,
    ) -> Self {
        let capacity = (packing.pages() + u32::from(side > 1)).min(page_limit(device, layers));
        Self {
            texture: texture(device, side, capacity * layers as u32),
            data: data::Data::new(device, queue, &materials),
            materials,
            side,
            packing,
        }
    }

    pub fn reserve(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layers: usize,
    ) -> Result<bool> {
        let needed = self.packing.pages();
        let limit = page_limit(device, layers);
        if needed > limit {
            return Err("Decoded materials exceed the GPU atlas budget".into());
        }
        let capacity = self.texture.depth_or_array_layers() / layers as u32;
        let mut changed = false;
        if needed > capacity {
            let capacity = needed.max(capacity * 2).min(limit);
            let texture = texture(device, self.side, capacity * layers as u32);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Grow material atlas"),
            });
            encoder.copy_texture_to_texture(
                self.texture.as_image_copy(),
                texture.as_image_copy(),
                self.texture.size(),
            );
            // Submit the copy before any new tile writes into the retained pages.
            queue.submit([encoder.finish()]);
            self.texture = texture;
            changed = true;
        }
        if self.materials.len() > self.data.capacity() {
            self.data = data::Data::new(device, queue, &self.materials);
            changed = true;
        }
        Ok(changed)
    }

    pub fn bindings(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Material mip interpolation"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Decoded material bindings"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self.texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.data.binding(),
                },
            ],
        })
    }
}

pub(super) fn page_limit(device: &wgpu::Device, layers: usize) -> u32 {
    let limit = device.limits().max_texture_array_layers;
    let pages = limit / layers as u32;
    pages - u32::from(super::super::array_layers(device, pages * layers as u32) > limit)
}

fn texture(device: &wgpu::Device, side: u32, layers: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Isolated opaque and semi-transparent mip chains"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: super::super::array_layers(device, layers),
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
