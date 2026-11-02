use crate::lighting::{Light, MAX_LIGHTS};
use glam::{Vec3, Vec4};

const WIDTH: u32 = 256;
const CELLS: usize = 32_768;
const TEXELS_PER_CELL: usize = MAX_LIGHTS / 4;
const HEIGHT: u32 = (CELLS * TEXELS_PER_CELL / WIDTH as usize) as u32;

#[derive(Default)]
struct Cells {
    lights: Vec<Vec4>,
    origin: Vec3,
    spacing: f32,
    size: [u32; 3],
    budget: usize,
    indices: Vec<[u32; MAX_LIGHTS]>,
}

impl Cells {
    fn update(&mut self, lights: Vec<Vec4>, budget: usize) -> bool {
        if self.lights == lights && self.budget == budget {
            return false;
        }
        self.lights = lights;
        self.budget = budget;
        self.indices.clear();
        self.size = [0; 3];
        if self.lights.is_empty() || budget == 0 {
            return true;
        }
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for light in &self.lights {
            low = low.min(light.truncate() - Vec3::splat(light.w));
            high = high.max(light.truncate() + Vec3::splat(light.w));
        }
        self.spacing = 4096.0;
        loop {
            self.origin = (low / self.spacing).floor() * self.spacing;
            self.size = ((high - self.origin) / self.spacing)
                .ceil()
                .to_array()
                .map(|n| (n as u32).max(1));
            if self.size.iter().map(|&n| u64::from(n)).product::<u64>() <= CELLS as u64 {
                break;
            }
            self.spacing *= 2.0;
        }
        let count = self.size.iter().product::<u32>() as usize;
        self.indices.resize(count, [0; MAX_LIGHTS]);
        let mut scores = vec![[f32::INFINITY; MAX_LIGHTS]; count];
        for (index, light) in self.lights.iter().enumerate() {
            let position = light.truncate();
            let min = self.cell(position - Vec3::splat(light.w));
            let max = self.cell(position + Vec3::splat(light.w));
            for z in min[2]..=max[2] {
                for y in min[1]..=max[1] {
                    for x in min[0]..=max[0] {
                        let corner =
                            self.origin + Vec3::new(x as f32, y as f32, z as f32) * self.spacing;
                        let nearest = position.clamp(corner, corner + Vec3::splat(self.spacing));
                        if nearest.distance_squared(position) >= light.w * light.w {
                            continue;
                        }
                        let center = corner + Vec3::splat(self.spacing * 0.5);
                        let score = center.distance_squared(position) / (light.w * light.w);
                        let cell = ((z * self.size[1] + y) * self.size[0] + x) as usize;
                        let Some(slot) = scores[cell][..budget].iter().position(|&old| score < old)
                        else {
                            continue;
                        };
                        scores[cell].copy_within(slot..budget - 1, slot + 1);
                        self.indices[cell].copy_within(slot..budget - 1, slot + 1);
                        scores[cell][slot] = score;
                        self.indices[cell][slot] = index as u32 + 1;
                    }
                }
            }
        }
        true
    }

    fn cell(&self, position: Vec3) -> [u32; 3] {
        let p = ((position - self.origin) / self.spacing).floor().to_array();
        std::array::from_fn(|axis| (p[axis] as u32).min(self.size[axis] - 1))
    }
}

pub(super) struct Grid {
    pub view: wgpu::TextureView,
    texture: wgpu::Texture,
    cells: Cells,
    pub order: Vec<usize>,
}

impl Grid {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("World light cells"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&Default::default()),
            texture,
            cells: Cells::default(),
            order: Vec::new(),
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, lights: &[Light], budget: usize) -> bool {
        self.order.clear();
        self.order.extend((0..lights.len()).filter(|&i| {
            lights[i].radius > 0.0 && lights[i].color.iter().any(|&value| value > 0.0)
        }));
        // Shadow priority follows the player. Cell membership must not follow it.
        self.order.sort_by(|&a, &b| {
            let a = lights[a].position.extend(lights[a].radius).to_array();
            let b = lights[b].position.extend(lights[b].radius).to_array();
            a.iter()
                .zip(&b)
                .map(|(a, b)| a.total_cmp(b))
                .find(|order| !order.is_eq())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let geometry = self
            .order
            .iter()
            .map(|&i| lights[i].position.extend(lights[i].radius))
            .collect();
        if !self.cells.update(geometry, budget.min(MAX_LIGHTS)) || self.cells.indices.is_empty() {
            return false;
        }
        let height = (self.cells.indices.len() * TEXELS_PER_CELL).div_ceil(WIDTH as usize) as u32;
        let mut data = self.cells.indices.clone();
        data.resize(
            height as usize * WIDTH as usize / TEXELS_PER_CELL,
            [0; MAX_LIGHTS],
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 16),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: WIDTH,
                height,
                depth_or_array_layers: 1,
            },
        );
        true
    }

    pub fn uniform(&self) -> [f32; 8] {
        let [x, y, z] = self.cells.origin.to_array();
        let [nx, ny, nz] = self.cells.size;
        [
            x,
            y,
            z,
            self.cells.spacing.max(1.0).recip(),
            nx as f32,
            ny as f32,
            nz as f32,
            0.0,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separated_lamps_share_a_local_budget_and_unchanged_cells_are_cached() {
        let lights: Vec<_> = (0..100)
            .map(|i| Vec4::new(i as f32 * 40_000.0, -2000.0, 0.0, 5000.0))
            .collect();
        let mut cells = Cells::default();
        assert!(cells.update(lights.clone(), 1));
        for (i, light) in lights.iter().enumerate() {
            let [x, y, z] = cells.cell(light.truncate());
            let cell = ((z * cells.size[1] + y) * cells.size[0] + x) as usize;
            assert_eq!(cells.indices[cell][0], i as u32 + 1);
            assert!(cells.indices[cell][1..].iter().all(|&index| index == 0));
        }
        assert!(cells.indices.len() <= CELLS);
        assert!(!cells.update(lights.clone(), 1));
        assert!(cells.update(lights, 0));
        assert!(cells.indices.is_empty());
    }
}
