use glam::{Vec2, Vec3};
use tenchu_assets::frontend::Image;

#[derive(Clone, Copy, Default)]
pub enum Blend {
    #[default]
    Opaque,
    Average,
    Add,
    Subtract,
    Quarter,
}

pub struct Sprite<'a> {
    image: &'a Image,
    source: [usize; 4],
    center: Vec2,
    scale: Vec2,
    angle: f32,
    color: Vec3,
    blend: Blend,
}

impl<'a> Sprite<'a> {
    pub fn new(image: &'a Image, x: f32, y: f32) -> Self {
        Self {
            image,
            source: [0, 0, image.size[0], image.size[1]],
            center: Vec2::new(x, y),
            scale: Vec2::ONE,
            angle: 0.0,
            color: Vec3::splat(128.0),
            blend: Blend::Opaque,
        }
    }
    pub fn row(mut self, row: usize, count: usize) -> Self {
        self.source[3] /= count;
        self.source[1] = row * self.source[3];
        self
    }
    pub fn region(mut self, region: [usize; 4]) -> Self {
        self.source = region;
        self
    }
    pub fn scale(mut self, x: f32, y: f32) -> Self {
        self.scale = Vec2::new(x, y);
        self
    }
    pub fn angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }
    pub fn color(mut self, color: [u8; 3]) -> Self {
        self.color = Vec3::from_array(color.map(f32::from));
        self
    }
    pub fn light(self, level: i32) -> Self {
        self.color([level.clamp(0, 255) as u8; 3])
    }
    pub fn blend(mut self, blend: Blend) -> Self {
        self.blend = blend;
        self
    }
}

pub struct Canvas {
    pub size: [usize; 2],
    pub pixels: Vec<u8>,
    pub frame: Option<super::draw::Frame>,
    pub sharp_text: bool,
    pub text: Vec<crate::fonts::Text>,
}

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vec2,
    pub uv: Vec2,
    pub color: Vec3,
}

impl Canvas {
    pub fn new(width: usize) -> Self {
        Self {
            size: [width, 240],
            pixels: vec![0; width * 240 * 4],
            frame: None,
            sharp_text: false,
            text: Vec::new(),
        }
    }
    pub fn clear(&mut self) {
        self.text.clear();
        if self.frame.is_some() {
            return;
        }
        for p in self.pixels.chunks_exact_mut(4) {
            p.copy_from_slice(&[0, 0, 0, 255]);
        }
    }

    pub fn label(&mut self, sprite: Sprite<'_>, text: impl Into<String>, color: egui::Color32) {
        if !self.sharp_text {
            self.sprite(sprite);
            return;
        }
        let width_scale = 320.0 / self.size[0] as f32;
        let height = sprite.source[3] as f32 * sprite.scale.y.abs();
        self.text.push(crate::fonts::Text {
            position: egui::pos2(
                sprite.center.x * width_scale,
                sprite.center.y - height * 0.6,
            ),
            text: text.into(),
            color,
            size: (height * 0.8).min(14.0),
            width: sprite.source[2] as f32 * sprite.scale.x.abs() * width_scale,
            centered: true,
        });
    }

    fn pixel(&mut self, x: usize, y: usize, word: u16, color: Vec3, blend: Blend) {
        if word == 0 {
            return;
        }
        let rgb = [word & 31, (word >> 5) & 31, (word >> 10) & 31];
        let target = &mut self.pixels[(y * self.size[0] + x) * 4..][..3];
        for (i, target) in target.iter_mut().enumerate() {
            let value = (f32::from(rgb[i]) * 255.0 / 31.0 * color[i] / 128.0).min(255.0) as i32;
            let old = i32::from(*target);
            *target = if word & 0x8000 == 0 {
                value
            } else {
                match blend {
                    Blend::Opaque => value,
                    Blend::Average => (old + value) / 2,
                    Blend::Add => old + value,
                    Blend::Subtract => old - value,
                    Blend::Quarter => old + value / 4,
                }
            }
            .clamp(0, 255) as u8;
        }
    }

    pub fn sprite(&mut self, sprite: Sprite<'_>) {
        let Sprite {
            image,
            source: [sx, sy, w, h],
            center,
            scale,
            angle,
            color,
            blend,
        } = sprite;
        let rotation = Vec2::from_angle(angle);
        let half = Vec2::new(w as f32, h as f32) * 0.5;
        let center = center + Vec2::new(self.size[0] as f32 / 2.0, 120.0);
        if let Some(frame) = &mut self.frame {
            let corners = [
                Vec2::ZERO,
                Vec2::new(w as f32, 0.0),
                Vec2::new(w as f32, h as f32),
                Vec2::new(0.0, h as f32),
            ];
            let vertices = [0, 1, 2, 0, 2, 3].map(|i| {
                let p = (corners[i] - half) * scale;
                Vertex {
                    position: center
                        + Vec2::new(
                            rotation.x * p.x - rotation.y * p.y,
                            rotation.y * p.x + rotation.x * p.y,
                        ),
                    uv: Vec2::new(sx as f32, sy as f32) + corners[i],
                    color,
                }
            });
            frame.triangles(image, &vertices, blend);
            return;
        }
        let radius = rotation.abs() * half.x * scale.x.abs()
            + Vec2::new(rotation.y.abs(), rotation.x.abs()) * half.y * scale.y.abs();
        let lo = (center - radius).floor().max(Vec2::ZERO);
        let hi = (center + radius)
            .ceil()
            .min(Vec2::new(self.size[0] as f32, 240.0));
        for y in lo.y as usize..hi.y as usize {
            for x in lo.x as usize..hi.x as usize {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - center;
                let uv = Vec2::new(p.dot(rotation), p.dot(Vec2::new(-rotation.y, rotation.x)))
                    / scale
                    + half;
                if uv.x >= 0.0 && uv.y >= 0.0 && uv.x < w as f32 && uv.y < h as f32 {
                    let index = (sy + uv.y as usize) * image.size[0] + sx + uv.x as usize;
                    if let Some(&word) = image.pixels.get(index) {
                        self.pixel(x, y, word, color, blend);
                    }
                }
            }
        }
    }

    pub fn triangle(&mut self, vertices: [Vertex; 3], blend: Blend, texel: impl Fn(Vec2) -> u16) {
        let [a, b, c] = vertices;
        let area = (b.position - a.position).perp_dot(c.position - a.position);
        if area.abs() < 0.01 {
            return;
        }
        let lo = a
            .position
            .min(b.position)
            .min(c.position)
            .floor()
            .max(Vec2::ZERO);
        let hi = a
            .position
            .max(b.position)
            .max(c.position)
            .ceil()
            .min(Vec2::new(self.size[0] as f32, 240.0));
        for y in lo.y as usize..hi.y as usize {
            for x in lo.x as usize..hi.x as usize {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let u = (b.position - p).perp_dot(c.position - p) / area;
                let v = (c.position - p).perp_dot(a.position - p) / area;
                let w = 1.0 - u - v;
                if u >= 0.0 && v >= 0.0 && w >= 0.0 {
                    self.pixel(
                        x,
                        y,
                        texel(a.uv * u + b.uv * v + c.uv * w),
                        a.color * u + b.color * v + c.color * w,
                        blend,
                    );
                }
            }
        }
    }

    pub fn image_triangles(&mut self, image: &Image, vertices: &[Vertex], blend: Blend) {
        if let Some(frame) = &mut self.frame {
            frame.triangles(image, vertices, blend);
        } else {
            for triangle in vertices.chunks_exact(3) {
                self.triangle(triangle.try_into().unwrap(), blend, |uv| {
                    image.pixels[uv.y.clamp(0.0, image.size[1] as f32 - 1.0) as usize
                        * image.size[0]
                        + uv.x.clamp(0.0, image.size[0] as f32 - 1.0) as usize]
                });
            }
        }
    }

    pub fn model_triangle(
        &mut self,
        vram: &tenchu_assets::texture::Vram,
        texture: Option<tenchu_assets::model::Texture>,
        vertices: [Vertex; 3],
        blend: Blend,
    ) {
        if let Some(frame) = &mut self.frame {
            frame.model(vram, texture, vertices, blend);
        } else {
            self.triangle(vertices, blend, |uv| {
                texture.map_or(0x7fff, |t| {
                    vram.texel(t.page, t.palette, [uv.x as u16, uv.y as u16])
                })
            });
        }
    }

    pub fn fade(&mut self, brightness: i32) {
        for text in &mut self.text {
            let rgb = text
                .color
                .to_array()
                .map(|v| (i32::from(v) - brightness * 2).clamp(0, 255) as u8);
            text.color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        }
        if let Some(frame) = &mut self.frame {
            frame.fade(brightness);
            return;
        }
        for p in self.pixels.chunks_exact_mut(4) {
            for v in &mut p[..3] {
                *v = (i32::from(*v) - brightness * 2).clamp(0, 255) as u8;
            }
        }
    }
}
