use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

pub const FEATURES: wgpu::Features =
    wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
const QUERIES: u32 = 5;
pub const SCENE_COLUMNS: [&str; 7] = [
    "sky_clear",
    "terrain_props",
    "actors",
    "blood_decals",
    "particles_effects",
    "in_scene_overlays",
    "resolve_finish",
];
pub const SCENE_NAMES: [&str; 7] = [
    "Sky + clear",
    "Terrain + static props",
    "Characters + moving objects",
    "Blood decals",
    "Particles + effects",
    "In-scene overlays",
    "MSAA resolve + finish",
];
const SCENE_QUERIES: u32 = 6;
const FREE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

#[derive(Clone, Copy)]
pub enum SceneStage {
    Sky,
    Terrain,
    Actors,
    Decals,
    Effects,
    Overlays,
}

#[derive(Clone, Copy)]
pub struct Sample {
    pub serial: u64,
    pub milliseconds: [f32; 4],
    pub scene: Option<[f32; 7]>,
}

impl Sample {
    fn read(serial: u64, stamps: &[u64], period: f32) -> Option<Self> {
        let elapsed = |a: u64, b: u64| b.checked_sub(a).map(|ticks| ticks as f32 * period);
        let mut milliseconds = [0.0; 4];
        for (i, duration) in milliseconds.iter_mut().enumerate() {
            *duration = elapsed(stamps[i], stamps[i + 1])?;
        }
        let scene = if stamps.len() > QUERIES as usize {
            let mut boundaries = [0; 8];
            boundaries[0] = stamps[1];
            boundaries[1..7].copy_from_slice(&stamps[QUERIES as usize..]);
            boundaries[7] = stamps[2];
            let mut scene = [0.0; 7];
            for (i, duration) in scene.iter_mut().enumerate() {
                *duration = elapsed(boundaries[i], boundaries[i + 1])?;
            }
            Some(scene)
        } else {
            None
        };
        Some(Self {
            serial,
            milliseconds,
            scene,
        })
    }
}

struct Readback {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
    serial: u64,
}

pub struct Timer {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    pending: [Readback; 3],
    active: Option<usize>,
    serial: u64,
    period: f32,
    pub latest: Option<Sample>,
    count: u32,
}

impl Timer {
    pub fn submitted_serial(&self) -> u64 {
        self.serial
    }

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        let count = QUERIES
            + if device
                .features()
                .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES)
            {
                SCENE_QUERIES
            } else {
                0
            };
        device.features().contains(FEATURES).then(|| Self {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Frame stage timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Timestamp resolve"),
                size: wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            pending: std::array::from_fn(|_| Readback {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Asynchronous timestamp readback"),
                    size: u64::from(count) * 8,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(FREE)),
                serial: 0,
            }),
            active: None,
            serial: 0,
            period: queue.get_timestamp_period() / 1_000_000.0,
            latest: None,
            count,
        })
    }

    pub fn begin(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        device.poll(wgpu::Maintain::Poll);
        for slot in &mut self.pending {
            match slot.state.load(Ordering::Acquire) {
                READY => {
                    let data = slot.buffer.slice(..).get_mapped_range();
                    let stamps: &[u64] = bytemuck::cast_slice(&data);
                    if self.latest.is_none_or(|s| s.serial < slot.serial)
                        && let Some(sample) = Sample::read(slot.serial, stamps, self.period)
                    {
                        self.latest = Some(sample);
                    }
                    drop(data);
                    slot.buffer.unmap();
                    slot.state.store(FREE, Ordering::Release);
                }
                FAILED => slot.state.store(FREE, Ordering::Release),
                _ => (),
            }
        }
        self.active = self
            .pending
            .iter()
            .position(|s| s.state.load(Ordering::Acquire) == FREE);
        if let Some(index) = self.active {
            self.serial += 1;
            self.pending[index].serial = self.serial;
            self.pending[index].state.store(PENDING, Ordering::Release);
            self.mark(encoder, 0);
        }
    }

    pub fn mark(&self, encoder: &mut wgpu::CommandEncoder, index: u32) {
        if self.active.is_some() {
            encoder.write_timestamp(&self.queries, index);
        }
    }

    pub fn mark_scene(&self, pass: &mut wgpu::RenderPass<'_>, stage: SceneStage) {
        if self.active.is_some() && self.count > QUERIES {
            pass.write_timestamp(&self.queries, QUERIES + stage as u32);
        }
    }

    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(index) = self.active {
            self.mark(encoder, QUERIES - 1);
            encoder.resolve_query_set(&self.queries, 0..self.count, &self.resolve, 0);
            encoder.copy_buffer_to_buffer(
                &self.resolve,
                0,
                &self.pending[index].buffer,
                0,
                u64::from(self.count) * 8,
            );
        }
    }

    pub fn submitted(&mut self) {
        if let Some(index) = self.active.take() {
            let slot = &self.pending[index];
            let state = slot.state.clone();
            slot.buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    state.store(
                        if result.is_ok() { READY } else { FAILED },
                        Ordering::Release,
                    );
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_timestamps_partition_scene_time_and_reject_invalid_order() {
        let mut stamps = [10, 20, 90, 110, 120, 30, 40, 50, 60, 70, 80];
        let sample = Sample::read(1, &stamps, 0.5).unwrap();
        assert_eq!(sample.milliseconds, [5.0, 35.0, 10.0, 5.0]);
        assert_eq!(sample.scene.unwrap(), [5.0; 7]);
        let coarse = Sample::read(2, &stamps[..5], 0.5).unwrap();
        assert_eq!(coarse.milliseconds, sample.milliseconds);
        assert!(coarse.scene.is_none());
        stamps[10] = 91;
        assert!(Sample::read(3, &stamps, 0.5).is_none());
        stamps[10] = 69;
        assert!(Sample::read(4, &stamps, 0.5).is_none());
    }
}
