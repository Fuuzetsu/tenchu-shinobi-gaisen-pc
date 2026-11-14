use crate::{
    render_mesh::{self, Vertex},
    visuals::{
        catalog::{self, Region},
        pixels::{Image, Mip},
        relief::{self, Surface},
    },
};
use bytemuck::{Pod, Zeroable};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tenchu_assets::{Result, model::Texture, texture::Vram};
mod atlas;
mod blood;
mod data;
use atlas::{Atlas, Packing};
pub(super) use data::shader;

pub const MAX_MIPS: usize = 13;
const SOLID: u32 = 128;

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
    blood_hd: bool,
    shadow_caster: bool,
    id: String,
    original: Arc<Image>,
    image: Image,
    replaced: bool,
    authored: bool,
    scroll: Option<[u16; 2]>,
    animation: Option<tenchu_assets::texture::TextureScroll>,
    phase: [f32; 2],
    manual_phase: Option<[f32; 2]>,
    scrolling: bool,
    uv_tile: bool,
    world_tile: Option<[f32; 2]>,
    dirty: bool,
    mips: Vec<Mip>,
    detail: Vec<Vec<u8>>,
}

impl Entry {
    fn source(&self) -> [f32; 4] {
        if self.animation.is_some() {
            return [-self.phase[0], -self.phase[1], 1.0, 1.0];
        }
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
        self.scrolling || self.animation.is_some() || self.uv_tile || self.world_tile.is_some()
    }

    fn flags(&self, layers: usize) -> u32 {
        self.image.rgba.chunks_exact(4).fold(
            SOLID
                | u32::from(self.replaced)
                | if layers == 3 { 8 } else { 0 }
                | if self.repeating() { 16 } else { 0 }
                | if self.world_tile.is_some() { 32 } else { 0 }
                | if self.authored { 64 } else { 0 }
                | if self.region.is_none() { 256 } else { 0 },
            |flags, pixel| {
                (if pixel[3] < 192 {
                    flags & !SOLID
                } else {
                    flags
                }) | match pixel[3] {
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
        if self.blood_hd {
            if self.dirty {
                self.image = blood::refine(&self.original);
            }
            self.replaced = true;
            return;
        }
        let replacement = enabled.then(|| textures.get(&self.id)).flatten();
        let image = replacement.map_or_else(
            || {
                self.original
                    .upscale(if self.region.is_some() { upscale } else { 1 })
            },
            |image| (**image).clone(),
        );
        let replaced = self.region.is_none() || replacement.is_some() || upscale > 1;
        let authored = self.region.is_none() || replacement.is_some();
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
    raw_shadow_pages: BTreeSet<Region>,
    shadows_dirty: bool,
    regions: BTreeMap<(Region, bool), u32>,
    external: BTreeMap<(String, Option<tenchu_assets::texture::TextureScroll>, bool), u32>,
    lookups: BTreeMap<(u16, u16, [u16; 4], bool), u32>,
    atlas: Atlas,
    repack: bool,
    textures: BTreeMap<String, Arc<Image>>,
    surfaces: BTreeMap<String, Surface>,
    layers: usize,
    enabled: bool,
    upscale: u16,
    emissions: BTreeMap<Region, [f32; 4]>,
    external_emissions: BTreeMap<String, [f32; 4]>,
}

impl Materials {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, vram: &Vram) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Decoded materials"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
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
            raw_shadow_pages: BTreeSet::new(),
            shadows_dirty: false,
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
            external_emissions: BTreeMap::new(),
        }
    }

    pub fn update(&mut self, vram: &Vram) {
        let changes = Changes::between(&self.vram, vram);
        self.shadows_dirty |= self.raw_shadow_pages.iter().any(|&r| changes.affects(r));
        for entry in &mut self.entries {
            let Some(region) = entry.region else { continue };
            entry.scroll = region.scroll_offset(vram);
            if !changes.affects(region) {
                continue;
            }
            let rgba = region.pixels(vram);
            if rgba != entry.original.rgba {
                self.shadows_dirty |= entry.shadow_caster && !entry.replaced;
                Arc::make_mut(&mut entry.original).rgba = rgba;
                entry.dirty |= entry.blood_hd;
                entry.prepare(&self.textures, &self.surfaces, self.enabled, self.upscale);
            }
        }
        self.vram.words.clone_from(&vram.words);
        self.vram.images.clone_from(&vram.images);
        self.vram.scrolling.clone_from(&vram.scrolling);
        self.vram.animations.clone_from(&vram.animations);
        self.vram.revision = vram.revision;
    }

    pub fn reset(&mut self, vram: &Vram) {
        self.vram = vram.clone();
        self.entries.clear();
        self.raw_shadow_pages.clear();
        self.shadows_dirty = true;
        self.regions.clear();
        self.external.clear();
        self.lookups.clear();
        self.emissions.clear();
        self.external_emissions.clear();
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

    pub fn set_external_emissions(&mut self, emissions: BTreeMap<String, [f32; 4]>) {
        self.external_emissions = emissions;
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

    pub fn bind(&mut self, material: &crate::visuals::models::Material) -> u32 {
        self.bind_style(material, false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn bind_original_hd(&mut self, material: &crate::visuals::models::Material) -> u32 {
        self.bind_style(material, true)
    }

    fn bind_style(&mut self, material: &crate::visuals::models::Material, blood_hd: bool) -> u32 {
        if let Some(region) = material.region {
            return self.register_variant(region, blood_hd);
        }
        let key = (material.id.clone(), material.animation, blood_hd);
        let id = if let Some(&id) = self.external.get(&key) {
            if Arc::ptr_eq(&self.entries[id as usize - 1].original, &material.image) {
                return id;
            }
            id
        } else {
            let id = self.entries.len() as u32 + 1;
            self.entries.push(Entry {
                region: None,
                blood_hd,
                shadow_caster: false,
                id: material.id.clone(),
                original: material.image.clone(),
                image: (*material.image).clone(),
                replaced: true,
                authored: false,
                scroll: None,
                animation: material.animation,
                phase: [0.0; 2],
                manual_phase: None,
                scrolling: false,
                uv_tile: false,
                world_tile: None,
                dirty: true,
                mips: Vec::new(),
                detail: Vec::new(),
            });
            self.external.insert(key, id);
            id
        };
        let entry = &mut self.entries[id as usize - 1];
        entry.original = material.image.clone();
        entry.animation = material.animation;
        entry.prepare(&self.textures, &self.surfaces, self.enabled, self.upscale);
        id
    }

    pub fn set_phase(&mut self, material: u32, phase: [f32; 2]) {
        if let Some(entry) = material
            .checked_sub(1)
            .and_then(|id| self.entries.get_mut(id as usize))
        {
            entry.manual_phase = Some(phase);
        }
    }

    pub fn annotate(&mut self, vertices: &mut [Vertex]) {
        for triangle in vertices.chunks_exact_mut(3) {
            let unbound = triangle[0].material == 0;
            if let Some(id) = self.annotate_triangle(std::array::from_fn(|i| &triangle[i]))
                && unbound
            {
                for vertex in triangle {
                    vertex.material = id;
                }
            }
        }
    }

    pub fn annotate_indexed(&mut self, vertices: &mut [Vertex], indices: &[u32]) {
        for triangle in indices.chunks_exact(3) {
            if let Some(id) =
                self.annotate_triangle(std::array::from_fn(|i| &vertices[triangle[i] as usize]))
            {
                for &index in triangle {
                    if vertices[index as usize].material == 0 {
                        vertices[index as usize].material = id;
                    }
                }
            }
        }
    }

    fn annotate_triangle(&mut self, triangle: [&Vertex; 3]) -> Option<u32> {
        let first = *triangle[0];
        let shadow_caster = first.flags
            & (render_mesh::TEXTURED
                | render_mesh::SCREEN
                | render_mesh::DEBUG
                | render_mesh::ALPHA
                | render_mesh::WATER)
            == render_mesh::TEXTURED;
        if first.material != 0 {
            if shadow_caster && let Some(entry) = self.entries.get_mut(first.material as usize - 1)
            {
                entry.shadow_caster = true;
            }
            return Some(first.material);
        }
        let blood_hd = render_mesh::original_hd_blood(first.flags);
        if first.flags & (render_mesh::DEBUG | render_mesh::TEXTURED) != render_mesh::TEXTURED
            || !blood_hd
                && first.flags & (render_mesh::STAIN | render_mesh::SOFT) == render_mesh::STAIN
        {
            if shadow_caster {
                self.raw_shadow_pages.insert(Region {
                    page: first.page as u16,
                    palette: first.palette as u16,
                    rect: [0, 0, 256, 256],
                });
            }
            return None;
        }
        let mut bounds = [255, 255, 0, 0];
        for vertex in triangle.iter() {
            for axis in 0..2 {
                let uv = vertex.uv[axis].clamp(0.0, 255.0) as u16;
                bounds[axis] = bounds[axis].min(uv);
                bounds[axis + 2] = bounds[axis + 2].max(uv);
            }
        }
        let key = (first.page as u16, first.palette as u16, bounds, blood_hd);
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
            let id = self.register_variant(region, blood_hd);
            self.lookups.insert(key, id);
            id
        };
        self.entries[id as usize - 1].shadow_caster |= shadow_caster;
        Some(id)
    }

    #[cfg(test)]
    fn register(&mut self, region: Region) -> u32 {
        self.register_variant(region, false)
    }

    fn register_variant(&mut self, region: Region, blood_hd: bool) -> u32 {
        if let Some(&id) = self.regions.get(&(region, blood_hd)) {
            return id;
        }
        let id = self.entries.len() as u32 + 1;
        let original = Image {
            size: [u32::from(region.rect[2]), u32::from(region.rect[3])],
            rgba: region.pixels(&self.vram),
        };
        let mut entry = Entry {
            region: Some(region),
            blood_hd,
            shadow_caster: false,
            id: catalog::image_id(
                original.size.map(|n| n as u16),
                &region.original_pixels(&self.vram),
            ),
            image: original.clone(),
            original: Arc::new(original),
            replaced: false,
            authored: false,
            scroll: region.scroll_offset(&self.vram),
            animation: None,
            phase: [0.0; 2],
            manual_phase: None,
            scrolling: false,
            uv_tile: false,
            world_tile: None,
            dirty: true,
            mips: Vec::new(),
            detail: Vec::new(),
        };
        entry.prepare(&self.textures, &self.surfaces, self.enabled, self.upscale);
        self.entries.push(entry);
        self.regions.insert((region, blood_hd), id);
        id
    }

    /// Returns whether cached shadow coverage may have changed.
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, time: f32) -> Result<bool> {
        if self.entries.len() + 1 > data::Data::limit(device) {
            return Err("Material metadata exceeds the GPU size limit".into());
        }
        for entry in &mut self.entries {
            if let Some(animation) = entry.animation {
                entry.phase = entry.manual_phase.unwrap_or_else(|| {
                    let offset = animation.offset((time * 30.0).round() as u32);
                    std::array::from_fn(|axis| {
                        f32::from(offset[axis]) / f32::from(animation.image.size[axis])
                    })
                });
            }
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
                    .map_or(entry.original.as_ref(), |image| image.as_ref());
                entry.detail = relief::mips(image, explicit, &sizes);
                if entry.detail.is_empty() {
                    entry.detail = sizes
                        .iter()
                        .map(|s| [128, 128, 128, 255].repeat((s[0] * s[1]) as usize))
                        .collect();
                }
            }
        }
        let mut changed = self.repack || self.shadows_dirty;
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
                changed |= entry.shadow_caster;
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
                write_coverage(
                    queue,
                    &self.atlas.coverage,
                    self.atlas.side,
                    material.mips[0],
                    entry,
                );
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
        self.shadows_dirty = false;
        Ok(changed)
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
                .map_or_else(
                    || self.external_emissions.get(&entry.id),
                    |r| self.emissions.get(&r),
                )
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
        for (index, entry) in self.entries.iter().enumerate() {
            write_coverage(
                queue,
                &self.atlas.coverage,
                self.atlas.side,
                self.atlas.materials[index + 1].mips[0],
                entry,
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

fn write_coverage(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    side: u32,
    rect: [f32; 4],
    entry: &Entry,
) {
    let [width, height] = entry.image.size;
    let bytes: Vec<u8> = (0..height + 2)
        .flat_map(|y| {
            let sy = border_pixel(y, height, entry.repeating());
            (0..width + 2).map(move |x| {
                let sx = border_pixel(x, width, entry.repeating());
                match entry.image.rgba[((sy * width + sx) * 4 + 3) as usize] {
                    0 => 0,
                    1..192 => 128,
                    _ => 255,
                }
            })
        })
        .collect();
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            origin: wgpu::Origin3d {
                x: rect[0] as u32 % side - 1,
                y: rect[1] as u32 - 1,
                z: rect[0] as u32 / side,
            },
            ..texture.as_image_copy()
        },
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width + 2),
            rows_per_image: Some(height + 2),
        },
        wgpu::Extent3d {
            width: width + 2,
            height: height + 2,
            depth_or_array_layers: 1,
        },
    );
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
