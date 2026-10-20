use crate::{
    render_mesh::{self, Vertex},
    visuals::{catalog::Region, pixels::Image},
};
use bytemuck::{Pod, Zeroable};
use std::collections::BTreeMap;
use tenchu_assets::{Result, model::Texture, texture::Vram};
use wgpu::util::DeviceExt;

pub const MAX_MIPS: usize = 13;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Material {
    source: [f32; 4],
    info: [u32; 4],
    mips: [[f32; 4]; MAX_MIPS],
}

struct Entry {
    region: Region,
    image: Image,
}

pub struct Materials {
    pub layout: wgpu::BindGroupLayout,
    pub bindings: wgpu::BindGroup,
    vram: Vram,
    entries: Vec<Entry>,
    regions: BTreeMap<Region, u32>,
    lookups: BTreeMap<(u16, u16, [u16; 4]), u32>,
    dirty: bool,
}

impl Materials {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, vram: &Vram) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Decoded materials"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bindings = upload(
            device,
            queue,
            &layout,
            1,
            &[vec![0; 4], vec![0; 4]],
            &[Material::zeroed()],
        );
        Self {
            layout,
            bindings,
            vram: vram.clone(),
            entries: Vec::new(),
            regions: BTreeMap::new(),
            lookups: BTreeMap::new(),
            dirty: false,
        }
    }

    pub fn update(&mut self, vram: &Vram) {
        for entry in &mut self.entries {
            let rgba = entry.region.pixels(vram);
            if rgba != entry.image.rgba {
                entry.image.rgba = rgba;
                self.dirty = true;
            }
        }
        self.vram = vram.clone();
    }

    pub fn annotate(&mut self, vertices: &mut [Vertex]) {
        for triangle in vertices.chunks_exact_mut(3) {
            let first = triangle[0];
            if first.flags & (render_mesh::SCREEN | render_mesh::DEBUG | render_mesh::TEXTURED)
                != render_mesh::TEXTURED
                || first.material != 0
            {
                continue;
            }
            let mut bounds = [255, 255, 0, 0];
            for vertex in triangle.iter() {
                for axis in 0..2 {
                    let uv = vertex.uv[axis].clamp(0.0, 255.0) as u16;
                    bounds[axis] = bounds[axis].min(uv);
                    bounds[axis + 2] = bounds[axis + 2].max(uv);
                }
            }
            let key = (first.page as u16, first.palette as u16, bounds);
            let id = if let Some(&id) = self.lookups.get(&key) {
                id
            } else {
                let region = Region::find(
                    &self.vram,
                    Texture {
                        page: key.0,
                        palette: key.1,
                    },
                    bounds,
                );
                let id = self.register(region);
                self.lookups.insert(key, id);
                id
            };
            for vertex in triangle {
                vertex.material = id;
            }
        }
    }

    fn register(&mut self, region: Region) -> u32 {
        if let Some(&id) = self.regions.get(&region) {
            return id;
        }
        let id = self.entries.len() as u32 + 1;
        self.entries.push(Entry {
            region,
            image: Image {
                size: [u32::from(region.rect[2]), u32::from(region.rect[3])],
                rgba: region.pixels(&self.vram),
            },
        });
        self.regions.insert(region, id);
        self.dirty = true;
        id
    }

    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let levels: Vec<_> = self
            .entries
            .iter()
            .map(|entry| entry.image.mips())
            .collect();
        let mut tiles: Vec<_> = levels
            .iter()
            .enumerate()
            .flat_map(|(entry, mips)| {
                mips.iter()
                    .enumerate()
                    .map(move |(mip, image)| (entry, mip, image))
            })
            .collect();
        tiles.sort_by_key(|(_, _, image)| std::cmp::Reverse(image.size[1]));
        let largest = tiles
            .iter()
            .map(|(_, _, image)| image.size[0].max(image.size[1]) + 2)
            .max()
            .unwrap_or(1);
        let area: u64 = tiles
            .iter()
            .map(|(_, _, image)| u64::from(image.size[0] + 2) * u64::from(image.size[1] + 2))
            .sum();
        let side = (area as f64).sqrt().ceil() as u32;
        let side = side
            .next_power_of_two()
            .clamp(64, 2048)
            .max(largest.next_power_of_two());
        let limits = device.limits();
        if side > limits.max_texture_dimension_2d {
            return Err("Decoded material exceeds the GPU texture size limit".into());
        }
        let mut materials = vec![Material::zeroed(); self.entries.len() + 1];
        for (index, entry) in self.entries.iter().enumerate() {
            materials[index + 1].source = entry.region.rect.map(f32::from);
            materials[index + 1].info = [
                levels[index].len() as u32,
                entry.image.size[0],
                entry.image.size[1],
                0,
            ];
        }
        let page_bytes = (side * side * 4) as usize;
        let mut pages = vec![vec![0u8; page_bytes]; 2];
        let (mut x, mut y, mut row_height, mut page) = (0, 0, 0, 0);
        for (entry, mip, image) in tiles {
            if mip >= MAX_MIPS {
                return Err("Decoded material has too many mip levels".into());
            }
            let [width, height] = image.size;
            if x + width + 2 > side {
                x = 0;
                y += row_height;
                row_height = 0;
            }
            if y + height + 2 > side {
                page += 1;
                if (page + 1) * 2 > limits.max_texture_array_layers {
                    return Err("Decoded materials exceed the GPU atlas budget".into());
                }
                pages.extend([vec![0; page_bytes], vec![0; page_bytes]]);
                x = 0;
                y = 0;
                row_height = 0;
            }
            materials[entry + 1].mips[mip] = [
                (x + 1 + page * side) as f32,
                (y + 1) as f32,
                width as f32,
                height as f32,
            ];
            for class in 0..2 {
                for dy in 0..height + 2 {
                    let sy = dy.saturating_sub(1).min(height - 1);
                    for dx in 0..width + 2 {
                        let sx = dx.saturating_sub(1).min(width - 1);
                        let from = ((sy * width + sx) * 4) as usize;
                        let to = (((y + dy) * side + x + dx) * 4) as usize;
                        pages[page as usize * 2 + class][to..to + 4]
                            .copy_from_slice(&image.classes[class][from..from + 4]);
                    }
                }
            }
            x += width + 2;
            row_height = row_height.max(height + 2);
        }
        self.bindings = upload(device, queue, &self.layout, side, &pages, &materials);
        self.dirty = false;
        Ok(())
    }
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    side: u32,
    pages: &[Vec<u8>],
    materials: &[Material],
) -> wgpu::BindGroup {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Isolated opaque and semi-transparent mip chains"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: pages.len() as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, bytes) in pages.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer as u32,
                },
                ..texture.as_image_copy()
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(side * 4),
                rows_per_image: Some(side),
            },
            wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
        );
    }
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Material mip interpolation"),
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Material source regions and mip rectangles"),
        contents: bytemuck::cast_slice(materials),
        usage: wgpu::BufferUsages::STORAGE,
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Decoded material bindings"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&Default::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: data.as_entire_binding(),
            },
        ],
    })
}
