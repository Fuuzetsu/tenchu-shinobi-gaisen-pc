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
mod atlas;
mod data;
use atlas::{Atlas, Packing};
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
    authored: bool,
    scroll: Option<[u16; 2]>,
    scrolling: bool,
    uv_tile: bool,
    world_tile: Option<[f32; 2]>,
    dirty: bool,
    mips: Vec<Mip>,
    detail: Vec<Vec<u8>>,
}

impl Entry {
    fn source(&self) -> [f32; 4] {
        let mut source = self
            .region
            .map_or([0.0, 0.0, 1.0, 1.0], |r| r.rect.map(f32::from));
        if self.world_tile.is_some() {
            let offset = self.scroll.unwrap_or_default();
            return [
                -f32::from(offset[0]) / source[2],
                -f32::from(offset[1]) / source[3],
                1.0,
                1.0,
            ];
        }
        if self.scrolling || self.uv_tile {
            let offset = self.scroll.unwrap_or_default();
            for axis in 0..2 {
                let period = source[axis + 2];
                let span = (period - 1.0).max(1.0);
                // Original mesh endpoints are texel centers; authored tiles need a full period.
                source[axis] += 0.5 - f32::from(offset[axis]) * span / period;
                source[axis + 2] = span;
            }
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
            self.world_tile.unwrap_or_default()[0],
            self.world_tile.unwrap_or_default()[1],
        ]
    }

    fn repeating(&self) -> bool {
        self.scrolling || self.uv_tile || self.world_tile.is_some()
    }

    fn flags(&self, layers: usize) -> u32 {
        self.image.rgba.chunks_exact(4).fold(
            u32::from(self.replaced)
                | if layers == 3 { 8 } else { 0 }
                | if self.repeating() { 16 } else { 0 }
                | if self.world_tile.is_some() { 32 } else { 0 }
                | if self.authored { 64 } else { 0 },
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

    fn prepare(
        &mut self,
        textures: &BTreeMap<String, Arc<Image>>,
        surfaces: &BTreeMap<String, Surface>,
        enabled: bool,
        upscale: u16,
    ) {
        let replacement = enabled.then(|| textures.get(&self.id)).flatten();
        let image = replacement.map_or_else(
            || {
                self.original
                    .upscale(if self.region.is_some() { upscale } else { 1 })
            },
            |image| (**image).clone(),
        );
        let replaced = self.region.is_none() || replacement.is_some() || upscale > 1;
        let authored = replacement.is_some();
        let scrolling = replacement.is_some() && self.scroll.is_some();
        let surface = replacement.and_then(|_| surfaces.get(&self.id));
        let world_tile = surface.and_then(|s| s.world_tile);
        let uv_tile = self.region.is_some() && surface.is_some_and(|s| s.uv_tile);
        self.dirty |= self.image != image
            || self.replaced != replaced
            || self.authored != authored
            || self.scrolling != scrolling
            || self.uv_tile != uv_tile
            || self.world_tile != world_tile;
        self.image = image;
        self.replaced = replaced;
        self.authored = authored;
        self.scrolling = scrolling;
        self.uv_tile = uv_tile;
        self.world_tile = world_tile;
    }
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
        let atlas = Atlas::new(
            device,
            queue,
            1,
            2,
            vec![Material::zeroed()],
            Packing::default(),
        );
        Self {
            bindings: atlas.bindings(device, &layout),
            layout,
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
                entry.prepare(&self.textures, &self.surfaces, self.enabled, self.upscale);
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
            entry.prepare(&self.textures, &self.surfaces, self.enabled, self.upscale);
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
                    authored: false,
                    scroll: None,
                    scrolling: false,
                    uv_tile: false,
                    world_tile: None,
                    dirty: true,
                    mips: Vec::new(),
                    detail: Vec::new(),
                });
                self.external.insert(material.id.clone(), id);
                id
            };
            let entry = &mut self.entries[id as usize - 1];
            entry.original = (*material.image).clone();
            entry.prepare(&self.textures, &self.surfaces, self.enabled, self.upscale);
            id
        });
    }

    pub fn annotate(&mut self, vertices: &mut [Vertex]) {
        for triangle in vertices.chunks_exact_mut(3) {
            let first = triangle[0];
            if first.flags & (render_mesh::DEBUG | render_mesh::TEXTURED) != render_mesh::TEXTURED
                || first.flags & (render_mesh::STAIN | render_mesh::SOFT) == render_mesh::STAIN
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
            authored: false,
            scroll: region.scroll_offset(&self.vram),
            scrolling: false,
            uv_tile: false,
            world_tile: None,
            dirty: true,
            mips: Vec::new(),
            detail: Vec::new(),
        };
        entry.prepare(&self.textures, &self.surfaces, self.enabled, self.upscale);
        self.entries.push(entry);
        self.regions.insert(region, id);
        id
    }

    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        if self.entries.len() + 1 > data::Data::limit(device) {
            return Err("Material metadata exceeds the GPU size limit".into());
        }
        for entry in &mut self.entries {
            if !entry.dirty {
                continue;
            }
            self.repack |= entry
                .mips
                .first()
                .is_some_and(|mip| mip.size != entry.image.size)
                || entry.image.size.iter().any(|n| n + 2 > self.atlas.side);
            entry.mips = entry.image.mips();
            if entry.mips.len() > MAX_MIPS {
                return Err("Decoded material has too many mip levels".into());
            }
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
        }
        if self.repack {
            self.rebuild(device, queue)?;
        } else {
            let first_new = self.atlas.materials.len() - 1;
            self.atlas
                .materials
                .resize(self.entries.len() + 1, Material::zeroed());
            for (entry, mip, size) in tiles(&self.entries, first_new) {
                self.atlas.materials[entry + 1].mips[mip] =
                    self.atlas.packing.place(self.atlas.side, size);
            }
            if self.atlas.reserve(device, queue, self.layers)? {
                self.bindings = self.atlas.bindings(device, &self.layout);
            }
            for (index, entry) in self.entries.iter().enumerate() {
                let material = &self.atlas.materials[index + 1];
                if !entry.dirty && material.source == entry.source() {
                    continue;
                }
                if entry.dirty {
                    self.atlas.materials[index + 1] = self.material(entry, material.mips);
                } else {
                    self.atlas.materials[index + 1].source = entry.source();
                }
                let material = &self.atlas.materials[index + 1];
                self.atlas
                    .data
                    .write(queue, index + 1, std::slice::from_ref(material));
                if !entry.dirty {
                    continue;
                }
                for (mip, image) in entry.mips.iter().enumerate() {
                    let rect = material.mips[mip];
                    let x = rect[0] as u32 % self.atlas.side - 1;
                    let y = rect[1] as u32 - 1;
                    let layer = rect[0] as u32 / self.atlas.side * self.layers as u32;
                    for class in 0..self.layers {
                        let bytes = padded(image.size, entry.pixels(mip, class), entry.repeating());
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
        }
        for entry in &mut self.entries {
            entry.dirty = false;
        }
        Ok(())
    }

    fn material(&self, entry: &Entry, mips: [[f32; 4]; MAX_MIPS]) -> Material {
        Material {
            source: entry.source(),
            info: [
                entry.mips.len() as u32,
                entry.image.size[0],
                entry.image.size[1],
                entry.flags(self.layers),
            ],
            emission: entry
                .region
                .and_then(|r| self.emissions.get(&r))
                .copied()
                .unwrap_or_default(),
            relief: entry.relief(self.enabled.then(|| self.surfaces.get(&entry.id)).flatten()),
            mips,
        }
    }

    fn rebuild(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        let tiles = tiles(&self.entries, 0);
        let largest = tiles
            .iter()
            .map(|(_, _, size)| size[0].max(size[1]) + 2)
            .max()
            .unwrap_or(1);
        let area: u64 = tiles
            .iter()
            .map(|(_, _, size)| u64::from(size[0] + 2) * u64::from(size[1] + 2))
            .sum();
        let side = (area as f64).sqrt().ceil() as u32;
        let side = side
            .next_power_of_two()
            .clamp(64, 2048)
            .max(largest.next_power_of_two());
        if side > device.limits().max_texture_dimension_2d {
            return Err("Decoded material exceeds the GPU texture size limit".into());
        }
        let mut materials = vec![Material::zeroed()];
        materials.extend(
            self.entries
                .iter()
                .map(|entry| self.material(entry, [[0.0; 4]; MAX_MIPS])),
        );
        let mut packing = Packing::default();
        for (entry, mip, size) in tiles {
            materials[entry + 1].mips[mip] = packing.place(side, size);
        }
        if packing.pages() > atlas::page_limit(device, self.layers) {
            return Err("Decoded materials exceed the GPU atlas budget".into());
        }
        let page_bytes = (side * side * 4) as usize;
        let mut pages = vec![vec![0u8; page_bytes]; packing.pages() as usize * self.layers];
        for (index, entry) in self.entries.iter().enumerate() {
            for (mip, image) in entry.mips.iter().enumerate() {
                let rect = materials[index + 1].mips[mip];
                let x = rect[0] as u32 % side - 1;
                let y = rect[1] as u32 - 1;
                let layer = rect[0] as usize / side as usize * self.layers;
                let [width, height] = image.size;
                for class in 0..self.layers {
                    let pixels = entry.pixels(mip, class);
                    for dy in 0..height + 2 {
                        let sy = border_pixel(dy, height, entry.repeating());
                        for dx in 0..width + 2 {
                            let sx = border_pixel(dx, width, entry.repeating());
                            let from = ((sy * width + sx) * 4) as usize;
                            let to = (((y + dy) * side + x + dx) * 4) as usize;
                            pages[layer + class][to..to + 4]
                                .copy_from_slice(&pixels[from..from + 4]);
                        }
                    }
                }
            }
        }
        self.atlas = Atlas::new(device, queue, side, self.layers, materials, packing);
        for (layer, bytes) in pages.iter().enumerate() {
            write_texture(
                queue,
                &self.atlas.texture,
                [0, 0, layer as u32],
                [side, side],
                bytes,
            );
        }
        self.bindings = self.atlas.bindings(device, &self.layout);
        self.repack = false;
        Ok(())
    }
}

fn tiles(entries: &[Entry], first: usize) -> Vec<(usize, usize, [u32; 2])> {
    let mut tiles: Vec<_> = entries
        .iter()
        .enumerate()
        .skip(first)
        .flat_map(|(entry, data)| {
            data.mips
                .iter()
                .enumerate()
                .map(move |(mip, image)| (entry, mip, image.size))
        })
        .collect();
    tiles.sort_by_key(|(_, _, size)| std::cmp::Reverse(size[1]));
    tiles
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
mod tests;
