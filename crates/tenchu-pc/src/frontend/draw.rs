use super::raster::{Blend, Vertex};
use crate::visuals::{catalog, pixels::Image};
use glam::{Vec2, Vec3};
use std::{collections::BTreeMap, ops::Range, sync::Arc};
use tenchu_assets::{frontend, model::Texture, texture::Vram};

pub type Artwork = BTreeMap<String, Arc<Image>>;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 3],
    uv_bounds: [f32; 4],
}

pub struct Draw {
    pub image: String,
    pub vertices: Range<u32>,
    pub blend: Blend,
}

pub struct Frame {
    pub vertices: Vec<GpuVertex>,
    pub draws: Vec<Draw>,
    pub images: Artwork,
    pub filtered: std::collections::BTreeSet<String>,
    size: Vec2,
    artwork: Arc<Artwork>,
    regions: BTreeMap<catalog::Region, String>,
}

pub fn pixels(image: &frontend::Image) -> Image {
    Image {
        size: image.size.map(|n| n as u32),
        rgba: image
            .pixels
            .iter()
            .copied()
            .flat_map(tenchu_assets::texture::rgba8)
            .collect(),
    }
}

pub fn identity(image: &Image) -> String {
    catalog::image_id(image.size.map(|n| n as u16), &image.rgba)
}

impl Frame {
    pub fn new(size: [usize; 2], artwork: Arc<Artwork>) -> Self {
        Self {
            vertices: Vec::new(),
            draws: Vec::new(),
            images: BTreeMap::new(),
            filtered: Default::default(),
            size: Vec2::from_array(size.map(|n| n as f32)),
            artwork,
            regions: BTreeMap::new(),
        }
    }

    fn image(&mut self, original: Image) -> String {
        let id = identity(&original);
        let replacement = self.artwork.get(&id);
        let image = replacement.cloned().unwrap_or_else(|| Arc::new(original));
        let key = if replacement.is_some() {
            identity(&image)
        } else {
            id
        };
        if replacement.is_some() {
            self.filtered.insert(key.clone());
        }
        self.images.entry(key.clone()).or_insert(image);
        key
    }

    pub fn triangles(&mut self, image: &frontend::Image, vertices: &[Vertex], blend: Blend) {
        let key = self.image(pixels(image));
        let scale = Vec2::from_array(image.size.map(|n| n as f32));
        let lo = vertices
            .iter()
            .fold(Vec2::splat(f32::INFINITY), |a, v| a.min(v.uv))
            / scale;
        let hi = vertices
            .iter()
            .fold(Vec2::splat(f32::NEG_INFINITY), |a, v| a.max(v.uv))
            / scale;
        self.push(
            &key,
            vertices.iter().map(|v| Vertex {
                uv: v.uv / scale,
                ..*v
            }),
            blend,
            [lo.x, lo.y, hi.x, hi.y],
        );
    }

    pub fn model(
        &mut self,
        vram: &Vram,
        texture: Option<Texture>,
        vertices: [Vertex; 3],
        blend: Blend,
    ) {
        let Some(texture) = texture else {
            let image = self.image(Image {
                size: [1, 1],
                rgba: vec![255; 4],
            });
            self.push(
                &image,
                vertices.into_iter().map(|v| Vertex {
                    uv: Vec2::splat(0.5),
                    ..v
                }),
                blend,
                [0.0, 0.0, 1.0, 1.0],
            );
            return;
        };
        let mut bounds = [255, 255, 0, 0];
        for v in vertices {
            for axis in 0..2 {
                bounds[axis] = bounds[axis].min(v.uv[axis] as u16);
                bounds[axis + 2] = bounds[axis + 2].max(v.uv[axis] as u16);
            }
        }
        let region = catalog::Region::find(vram, texture, bounds);
        let key = if let Some(key) = self.regions.get(&region) {
            key.clone()
        } else {
            let key = self.image(Image {
                size: [region.rect[2].into(), region.rect[3].into()],
                rgba: region.original_pixels(vram),
            });
            self.regions.insert(region, key.clone());
            key
        };
        let [x, y, w, h] = region.rect.map(f32::from);
        self.push(
            &key,
            vertices.into_iter().map(|v| Vertex {
                uv: (v.uv - Vec2::new(x, y)) / Vec2::new(w, h),
                ..v
            }),
            blend,
            [0.0, 0.0, 1.0, 1.0],
        );
    }

    fn push(
        &mut self,
        image: &str,
        vertices: impl IntoIterator<Item = Vertex>,
        blend: Blend,
        uv_bounds: [f32; 4],
    ) {
        let start = self.vertices.len() as u32;
        self.vertices
            .extend(vertices.into_iter().map(|v| GpuVertex {
                position:
                    ((v.position / self.size * 2.0 - Vec2::ONE) * Vec2::new(1.0, -1.0)).to_array(),
                uv: v.uv.to_array(),
                color: (v.color / 128.0).to_array(),
                uv_bounds,
            }));
        self.draws.push(Draw {
            image: image.into(),
            vertices: start..self.vertices.len() as u32,
            blend,
        });
    }

    pub fn fade(&mut self, brightness: i32) {
        if brightness <= 0 {
            return;
        }
        let image = self.image(Image {
            size: [1, 1],
            rgba: vec![255, 255, 255, 128],
        });
        let color = Vec3::splat((brightness * 2).min(255) as f32 * 128.0 / 255.0);
        let corners = [
            Vec2::ZERO,
            Vec2::new(self.size.x, 0.0),
            self.size,
            Vec2::new(0.0, self.size.y),
        ];
        self.push(
            &image,
            [0, 1, 2, 0, 2, 3].map(|i| Vertex {
                position: corners[i],
                uv: Vec2::ZERO,
                color,
            }),
            Blend::Subtract,
            [0.0, 0.0, 1.0, 1.0],
        );
    }
}
