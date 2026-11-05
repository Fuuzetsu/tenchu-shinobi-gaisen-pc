use crate::lighting::{Light, MAX_LIGHTS};
use glam::{Vec3, Vec4};

const WIDTH: u32 = 256;
const CELLS: usize = 32_768;
const TEXELS_PER_CELL: usize = MAX_LIGHTS / 4;
const CELLS_PER_ROW: usize = WIDTH as usize / TEXELS_PER_CELL;
const HEIGHT: u32 = (CELLS * TEXELS_PER_CELL / WIDTH as usize) as u32;

#[derive(Default)]
struct Cells {
    lights: Vec<Vec4>,
    origin: Vec3,
    spacing: f32,
    size: [u32; 3],
    budget: usize,
    indices: Vec<[u32; MAX_LIGHTS]>,
    scores: Vec<[f32; MAX_LIGHTS]>,
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
        self.scores.resize(count, [f32::INFINITY; MAX_LIGHTS]);
        self.scores.fill([f32::INFINITY; MAX_LIGHTS]);
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
                        if score >= self.scores[cell][budget - 1] {
                            continue;
                        }
                        let Some(slot) = self.scores[cell][..budget]
                            .iter()
                            .position(|&old| score < old)
                        else {
                            continue;
                        };
                        self.scores[cell].copy_within(slot..budget - 1, slot + 1);
                        self.indices[cell].copy_within(slot..budget - 1, slot + 1);
                        self.scores[cell][slot] = score;
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
    uploaded: Vec<[u32; MAX_LIGHTS]>,
    pub order: Vec<usize>,
}

impl Grid {
    pub fn new(device: &wgpu::Device) -> Self {
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        #[cfg(test)]
        let usage = usage | wgpu::TextureUsages::COPY_SRC;
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
            usage,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&Default::default()),
            texture,
            cells: Cells::default(),
            uploaded: Vec::new(),
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
        self.cells.indices.resize(
            self.cells.indices.len().next_multiple_of(CELLS_PER_ROW),
            [0; MAX_LIGHTS],
        );
        let rows = changed_rows(&self.cells.indices, &self.uploaded);
        if rows.is_empty() {
            return false;
        }
        if self.uploaded.len() < self.cells.indices.len() {
            self.uploaded
                .resize(self.cells.indices.len(), [0; MAX_LIGHTS]);
        }
        let cells = rows.start * CELLS_PER_ROW..rows.end * CELLS_PER_ROW;
        let data = &self.cells.indices[cells.clone()];
        self.uploaded[cells].copy_from_slice(data);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: rows.start as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 16),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: rows.len() as u32,
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

fn changed_rows(
    current: &[[u32; MAX_LIGHTS]],
    previous: &[[u32; MAX_LIGHTS]],
) -> std::ops::Range<usize> {
    let changed = |(index, row): (usize, &[[u32; MAX_LIGHTS]])| {
        previous.get(index * CELLS_PER_ROW..(index + 1) * CELLS_PER_ROW) != Some(row)
    };
    let mut rows = current.chunks_exact(CELLS_PER_ROW).enumerate();
    let Some((first, _)) = rows.find(|&row| changed(row)) else {
        return 0..0;
    };
    let last = rows
        .rfind(|&row| changed(row))
        .map_or(first, |(index, _)| index);
    first..last + 1
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

    #[test]
    fn local_light_selection_matches_exhaustive_stable_ranking() {
        let lights: Vec<_> = (0..48)
            .map(|i| {
                if i % 3 == 0 {
                    Vec4::new(0.0, 0.0, 0.0, 8192.0)
                } else {
                    Vec4::new(
                        ((i * 13) % 9 - 4) as f32 * 4096.0,
                        ((i * 7) % 5 - 2) as f32 * 4096.0,
                        ((i * 11) % 7 - 3) as f32 * 4096.0,
                        (i % 4 + 1) as f32 * 4096.0,
                    )
                }
            })
            .collect();
        let mut cells = Cells::default();
        for budget in [1, 8, MAX_LIGHTS] {
            cells.update(lights.clone(), budget);
            for (index, actual) in cells.indices.iter().enumerate() {
                let [nx, ny, _] = cells.size.map(|n| n as usize);
                let position = Vec3::new(
                    (index % nx) as f32,
                    (index / nx % ny) as f32,
                    (index / (nx * ny)) as f32,
                );
                let low = cells.origin + position * cells.spacing;
                let high = low + Vec3::splat(cells.spacing);
                let center = low + Vec3::splat(cells.spacing * 0.5);
                let mut candidates: Vec<_> = lights
                    .iter()
                    .enumerate()
                    .filter_map(|(index, light)| {
                        let p = light.truncate();
                        let radius_squared = light.w * light.w;
                        (p.clamp(low, high).distance_squared(p) < radius_squared).then(|| {
                            (
                                center.distance_squared(p) / radius_squared,
                                index as u32 + 1,
                            )
                        })
                    })
                    .collect();
                candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut expected = [0; MAX_LIGHTS];
                for (slot, &(_, light)) in candidates.iter().take(budget).enumerate() {
                    expected[slot] = light;
                }
                assert_eq!(*actual, expected, "budget {budget}, cell {index}");
            }
        }
    }

    #[test]
    #[ignore = "requires a GPU; partial light-table uploads must match full tables through motion, resizing, removal and budget changes"]
    fn partial_uploads_preserve_light_tables() {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default(), None)).unwrap();
        let mut grid = Grid::new(&device);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light table verification"),
            size: u64::from(WIDTH * HEIGHT * 16),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let lamp = |x, radius| Light {
            position: Vec3::new(x, 0.0, 0.0),
            radius,
            color: [1.0; 3],
            flame: false,
            emission_radius: 0.0,
        };
        let mut partial = false;
        let mut reused = false;
        for step in 0..24 {
            let mut lights = vec![lamp(-40_000.0, 12_000.0), lamp(40_000.0, 12_000.0)];
            if step < 16 {
                lights.push(lamp(step as f32 * 400.0, 3000.0));
            }
            if (8..12).contains(&step) {
                lights.push(lamp(-120_000.0, 8000.0));
            }
            let budget = [8, 1, 0, 16][step / 6];
            let previous = grid.uploaded.clone();
            grid.update(&queue, &lights, budget);
            if grid.cells.indices.is_empty() {
                continue;
            }
            let rows = changed_rows(&grid.cells.indices, &previous);
            partial |= !rows.is_empty() && rows.len() < grid.cells.indices.len() / CELLS_PER_ROW;
            reused |= rows.is_empty();
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_buffer(
                grid.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(WIDTH * 16),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: WIDTH,
                    height: (grid.cells.indices.len() / CELLS_PER_ROW) as u32,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit([encoder.finish()]);
            let (send, receive) = std::sync::mpsc::channel();
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    send.send(result).unwrap();
                });
            device.poll(wgpu::Maintain::Wait);
            receive.recv().unwrap().unwrap();
            {
                let actual = buffer.slice(..).get_mapped_range();
                let expected: &[u8] = bytemuck::cast_slice(&grid.cells.indices);
                assert_eq!(&actual[..expected.len()], expected, "step {step}");
            }
            buffer.unmap();
        }
        assert!(partial && reused);
    }
}
