use crate::{
    render_mesh::{self, Vertex},
    visuals::{
        catalog::{self, Region},
        pixels::{Image, Mip},
        relief::{self, Surface},
    },
};
use bytemuck::{Pod, Zeroable};
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{Result, model::Texture, texture::Vram};
mod data;
pub(super) use data::shader;

pub const MAX_MIPS: usize = 13;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Material {
    source: [f32; 4],
    info: [u32; 4],
    emission: [f32; 4],
    relief: [f32; 4],
    mips: [[f32; 4]; MAX_MIPS],
}

struct Entry {
    region: Option<Region>,
    id: String,
    original: Image,
    image: Image,
    replaced: bool,
    scroll: Option<[u16; 2]>,
    scrolling: bool,
    dirty: bool,
    mips: Vec<Mip>,
    detail: Vec<Vec<u8>>,
}

impl Entry {
    fn source(&self) -> [f32; 4] {
        let mut source = self
            .region
            .map_or([0.0, 0.0, 1.0, 1.0], |r| r.rect.map(f32::from));
        if self.scrolling {
            let offset = self.scroll.unwrap_or_default();
            source[0] -= f32::from(offset[0]);
            source[1] -= f32::from(offset[1]);
        }
        source
    }

    fn relief(&self, surface: Option<&Surface>) -> [f32; 4] {
        let eligible = self.image.size.into_iter().all(|n| n >= 4)
            && self.image.rgba.chunks_exact(4).all(|p| p[3] != 0);
        [
            if eligible {
                surface.map_or(1.0, |s| s.strength)
            } else {
                0.0
            },
            f32::from(surface.is_some_and(|s| s.height.is_some())),
            0.0,
            0.0,
        ]
    }

    fn flags(&self, layers: usize) -> u32 {
        self.image.rgba.chunks_exact(4).fold(
            u32::from(self.replaced)
                | if layers == 3 { 8 } else { 0 }
                | if self.scrolling { 16 } else { 0 },
            |flags, pixel| {
                flags
                    | match pixel[3] {
                        0 => 0,
                        1..192 => 4,
                        _ => 2,
                    }
            },
        )
    }

    fn pixels(&self, mip: usize, class: usize) -> &[u8] {
        if class < 2 {
            &self.mips[mip].classes[class]
        } else {
            &self.detail[mip]
        }
    }

    fn prepare(&mut self, textures: &BTreeMap<String, Arc<Image>>, enabled: bool, upscale: u16) {
        let replacement = enabled.then(|| textures.get(&self.id)).flatten();
        let image = replacement.map_or_else(
            || {
                self.original
                    .upscale(if self.region.is_some() { upscale } else { 1 })
            },
            |image| (**image).clone(),
        );
        let replaced = self.region.is_none() || replacement.is_some() || upscale > 1;
        let scrolling = replacement.is_some() && self.scroll.is_some();
        self.dirty |=
            self.image != image || self.replaced != replaced || self.scrolling != scrolling;
        self.image = image;
        self.replaced = replaced;
        self.scrolling = scrolling;
    }
}

struct Atlas {
    texture: wgpu::Texture,
    data: data::Data,
    materials: Vec<Material>,
    side: u32,
}

pub struct Materials {
    pub layout: wgpu::BindGroupLayout,
    pub bindings: wgpu::BindGroup,
    vram: Vram,
    entries: Vec<Entry>,
    regions: BTreeMap<Region, u32>,
    external: BTreeMap<String, u32>,
    lookups: BTreeMap<(u16, u16, [u16; 4]), u32>,
    atlas: Atlas,
    repack: bool,
    textures: BTreeMap<String, Arc<Image>>,
    surfaces: BTreeMap<String, Surface>,
    layers: usize,
    enabled: bool,
    upscale: u16,
    emissions: BTreeMap<Region, [f32; 4]>,
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
                    ty: data::binding_type(device),
                    count: None,
                },
            ],
        });
        let (bindings, atlas) = upload(
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
            external: BTreeMap::new(),
            lookups: BTreeMap::new(),
            atlas,
            repack: false,
            textures: BTreeMap::new(),
            surfaces: BTreeMap::new(),
            layers: 2,
            enabled: true,
            upscale: 1,
            emissions: BTreeMap::new(),
        }
    }

    pub fn update(&mut self, vram: &Vram) {
        let changes = Changes::between(&self.vram, vram);
        for entry in &mut self.entries {
            let Some(region) = entry.region else { continue };
            entry.scroll = region.scroll_offset(vram);
            if !changes.affects(region) {
                continue;
            }
            let rgba = region.pixels(vram);
            if rgba != entry.original.rgba {
                entry.original.rgba = rgba;
                entry.prepare(&self.textures, self.enabled, self.upscale);
            }
        }
        self.vram.words.clone_from(&vram.words);
        self.vram.images.clone_from(&vram.images);
        self.vram.scrolling.clone_from(&vram.scrolling);
        self.vram.revision = vram.revision;
    }

    pub fn reset(&mut self, vram: &Vram) {
        self.vram = vram.clone();
        self.entries.clear();
        self.regions.clear();
        self.external.clear();
        self.lookups.clear();
        self.emissions.clear();
        self.repack = true;
    }

    pub fn set_pack(&mut self, pack: &crate::visuals::pack::Pack) {
        self.textures = pack.textures.clone();
        self.surfaces = pack.surfaces.clone();
        self.repack = true;
        for entry in &mut self.entries {
            entry.dirty = true;
        }
        self.prepare();
    }

    pub fn set_emissions(&mut self, emissions: BTreeMap<Region, [f32; 4]>) {
        self.emissions = emissions;
        self.repack = true;
    }

    pub fn configure(&mut self, graphics: &crate::settings::Graphics) {
        let layers = if graphics.surface_detail != 0 { 3 } else { 2 };
        if self.layers != layers || self.enabled != graphics.asset_pack {
            self.layers = layers;
            self.repack = true;
            for entry in &mut self.entries {
                entry.dirty = true;
            }
        }
        if self.enabled != graphics.asset_pack || self.upscale != graphics.upscale {
            self.enabled = graphics.asset_pack;
            self.upscale = graphics.upscale;
            self.prepare();
        }
    }

    fn prepare(&mut self) {
        for entry in &mut self.entries {
            entry.prepare(&self.textures, self.enabled, self.upscale);
        }
    }

    pub fn bind_models(&mut self, models: &mut crate::visuals::models::Models) {
        models.bind(|material| {
            if let Some(region) = material.region {
                return self.register(region);
            }
            let id = if let Some(&id) = self.external.get(&material.id) {
                id
            } else {
                let id = self.entries.len() as u32 + 1;
                self.entries.push(Entry {
                    region: None,
                    id: material.id.clone(),
                    original: (*material.image).clone(),
                    image: (*material.image).clone(),
                    replaced: true,
                    scroll: None,
                    scrolling: false,
                    dirty: true,
                    mips: Vec::new(),
                    detail: Vec::new(),
                });
                self.external.insert(material.id.clone(), id);
                id
            };
            let entry = &mut self.entries[id as usize - 1];
            entry.original = (*material.image).clone();
            entry.prepare(&self.textures, self.enabled, self.upscale);
            id
        });
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
        let original = Image {
            size: [u32::from(region.rect[2]), u32::from(region.rect[3])],
            rgba: region.pixels(&self.vram),
        };
        let mut entry = Entry {
            region: Some(region),
            id: catalog::image_id(
                original.size.map(|n| n as u16),
                &region.original_pixels(&self.vram),
            ),
            image: original.clone(),
            original,
            replaced: false,
            scroll: region.scroll_offset(&self.vram),
            scrolling: false,
            dirty: true,
            mips: Vec::new(),
            detail: Vec::new(),
        };
        entry.prepare(&self.textures, self.enabled, self.upscale);
        self.entries.push(entry);
        self.regions.insert(region, id);
        id
    }

    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        self.repack |= self.entries.len() + 1 != self.atlas.materials.len();
        for (index, entry) in self.entries.iter_mut().enumerate() {
            if !self.repack && self.atlas.materials[index + 1].source != entry.source() {
                self.atlas.materials[index + 1].source = entry.source();
                self.atlas.data.write(
                    queue,
                    index + 1,
                    &self.atlas.materials[index + 1..index + 2],
                );
            }
            if !entry.dirty {
                continue;
            }
            if entry.mips.first().map(|mip| mip.size) != Some(entry.image.size) {
                self.repack = true;
            }
            entry.mips = entry.image.mips();
            entry.detail.clear();
            if self.layers == 3 {
                let explicit = self
                    .enabled
                    .then(|| self.surfaces.get(&entry.id))
                    .flatten()
                    .and_then(|s| s.height.as_deref());
                let sizes: Vec<_> = entry.mips.iter().map(|m| m.size).collect();
                let image = self
                    .enabled
                    .then(|| self.textures.get(&entry.id))
                    .flatten()
                    .map_or(&entry.original, |image| image.as_ref());
                entry.detail = relief::mips(image, explicit, &sizes);
                if entry.detail.is_empty() {
                    entry.detail = sizes
                        .iter()
                        .map(|s| [128, 128, 128, 255].repeat((s[0] * s[1]) as usize))
                        .collect();
                }
            }
            if !self.repack {
                let material = &mut self.atlas.materials[index + 1];
                material.info[3] = entry.flags(self.layers);
                material.relief =
                    entry.relief(self.enabled.then(|| self.surfaces.get(&entry.id)).flatten());
                self.atlas
                    .data
                    .write(queue, index + 1, std::slice::from_ref(material));
                for (mip, image) in entry.mips.iter().enumerate() {
                    let rect = material.mips[mip];
                    let x = rect[0] as u32 % self.atlas.side - 1;
                    let y = rect[1] as u32 - 1;
                    let layer = rect[0] as u32 / self.atlas.side * self.layers as u32;
                    for class in 0..self.layers {
                        let bytes = padded(image.size, entry.pixels(mip, class), entry.scrolling);
                        write_texture(
                            queue,
                            &self.atlas.texture,
                            [x, y, layer + class as u32],
                            image.size.map(|n| n + 2),
                            &bytes,
                        );
                    }
                }
            }
            entry.dirty = false;
        }
        if !self.repack {
            return Ok(());
        }
        let mut tiles: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .flat_map(|(entry, data)| {
                data.mips
                    .iter()
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
        if data::uses_texture(device)
            && self.entries.len() + 1 > limits.max_texture_dimension_2d as usize
        {
            return Err("Material metadata exceeds the GPU texture size limit".into());
        }
        if side > limits.max_texture_dimension_2d {
            return Err("Decoded material exceeds the GPU texture size limit".into());
        }
        let mut materials = vec![Material::zeroed(); self.entries.len() + 1];
        for (index, entry) in self.entries.iter().enumerate() {
            materials[index + 1].emission = entry
                .region
                .and_then(|r| self.emissions.get(&r))
                .copied()
                .unwrap_or([0.0; 4]);
            let surface = self.enabled.then(|| self.surfaces.get(&entry.id)).flatten();
            materials[index + 1].relief = entry.relief(surface);
            materials[index + 1].source = entry.source();
            materials[index + 1].info = [
                entry.mips.len() as u32,
                entry.image.size[0],
                entry.image.size[1],
                entry.flags(self.layers),
            ];
        }
        let page_bytes = (side * side * 4) as usize;
        let mut pages = vec![vec![0u8; page_bytes]; self.layers];
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
                if super::array_layers(device, (page + 1) * self.layers as u32)
                    > limits.max_texture_array_layers
                {
                    return Err("Decoded materials exceed the GPU atlas budget".into());
                }
                pages.extend((0..self.layers).map(|_| vec![0; page_bytes]));
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
            for class in 0..self.layers {
                for dy in 0..height + 2 {
                    let sy = border_pixel(dy, height, self.entries[entry].scrolling);
                    for dx in 0..width + 2 {
                        let sx = border_pixel(dx, width, self.entries[entry].scrolling);
                        let from = ((sy * width + sx) * 4) as usize;
                        let to = (((y + dy) * side + x + dx) * 4) as usize;
                        pages[page as usize * self.layers + class][to..to + 4].copy_from_slice(
                            &self.entries[entry].pixels(mip, class)[from..from + 4],
                        );
                    }
                }
            }
            x += width + 2;
            row_height = row_height.max(height + 2);
        }
        (self.bindings, self.atlas) = upload(device, queue, &self.layout, side, &pages, &materials);
        self.repack = false;
        Ok(())
    }
}

fn border_pixel(coordinate: u32, size: u32, wrap: bool) -> u32 {
    if wrap {
        (coordinate + size - 1) % size
    } else {
        coordinate.saturating_sub(1).min(size - 1)
    }
}

fn padded(size: [u32; 2], pixels: &[u8], wrap: bool) -> Vec<u8> {
    let [width, height] = size;
    let mut bytes = Vec::with_capacity(((width + 2) * (height + 2) * 4) as usize);
    for y in 0..height + 2 {
        let sy = border_pixel(y, height, wrap);
        for x in 0..width + 2 {
            let sx = border_pixel(x, width, wrap);
            let at = ((sy * width + sx) * 4) as usize;
            bytes.extend_from_slice(&pixels[at..at + 4]);
        }
    }
    bytes
}

fn write_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    origin: [u32; 3],
    size: [u32; 2],
    bytes: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            origin: wgpu::Origin3d {
                x: origin[0],
                y: origin[1],
                z: origin[2],
            },
            ..texture.as_image_copy()
        },
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size[0] * 4),
            rows_per_image: Some(size[1]),
        },
        wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
    );
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    side: u32,
    pages: &[Vec<u8>],
    materials: &[Material],
) -> (wgpu::BindGroup, Atlas) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Isolated opaque and semi-transparent mip chains"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: super::array_layers(device, pages.len() as u32),
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, bytes) in pages.iter().enumerate() {
        write_texture(queue, &texture, [0, 0, layer as u32], [side, side], bytes);
    }
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Material mip interpolation"),
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let data = data::Data::new(device, queue, materials);
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                resource: data.binding(),
            },
        ],
    });
    (
        bindings,
        Atlas {
            texture,
            data,
            materials: materials.to_vec(),
            side,
        },
    )
}

struct Changes([Option<(usize, usize)>; 512]);

impl Changes {
    fn between(old: &Vram, new: &Vram) -> Self {
        Self(std::array::from_fn(|y| {
            let old = &old.words[y * 1024..(y + 1) * 1024];
            let new = &new.words[y * 1024..(y + 1) * 1024];
            if old == new {
                return None;
            }
            let start = old.iter().zip(new).position(|(a, b)| a != b).unwrap();
            let end = old.iter().zip(new).rposition(|(a, b)| a != b).unwrap() + 1;
            Some((start, end))
        }))
    }

    fn row(&self, x: usize, y: usize, width: usize) -> bool {
        let Some((start, end)) = self.0[y & 511] else {
            return false;
        };
        let x = x & 1023;
        (x < end && x + width > start) || (x + width > 1024 && x + width - 1024 > start)
    }

    fn affects(&self, region: Region) -> bool {
        let mode = (region.page >> 7) & 3;
        let scale = match mode {
            0 => 4,
            1 => 2,
            _ => 1,
        };
        let [u, v, width, height] = region.rect.map(usize::from);
        let x = usize::from(region.page & 15) * 64 + u / scale;
        let width = (u + width - 1) / scale + 1 - u / scale;
        let y = usize::from(region.page & 16) * 16 + v;
        (y..y + height).any(|y| self.row(x, y, width))
            || (mode < 2
                && self.row(
                    usize::from(region.palette & 63) * 16,
                    usize::from(region.palette >> 6),
                    if mode == 0 { 16 } else { 256 },
                ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
