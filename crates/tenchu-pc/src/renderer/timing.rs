use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

pub const FEATURES: wgpu::Features =
    wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
const QUERIES: u32 = 5;
const FREE: u8 = 0;
const PENDING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;
const BYTES: u64 = QUERIES as u64 * 8;

#[derive(Clone, Copy)]
pub struct Sample {
    pub serial: u64,
    pub milliseconds: [f32; 4],
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
}

impl Timer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        device.features().contains(FEATURES).then(|| Self {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Frame stage timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: QUERIES,
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
                    size: BYTES,
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
        })
    }

    pub fn begin(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        device.poll(wgpu::Maintain::Poll);
        for slot in &mut self.pending {
            match slot.state.load(Ordering::Acquire) {
                READY => {
                    let data = slot.buffer.slice(..).get_mapped_range();
                    let stamps: &[u64] = bytemuck::cast_slice(&data);
                    if stamps.windows(2).all(|s| s[1] >= s[0])
                        && self.latest.is_none_or(|s| s.serial < slot.serial)
                    {
                        self.latest = Some(Sample {
                            serial: slot.serial,
                            milliseconds: std::array::from_fn(|i| {
                                (stamps[i + 1] - stamps[i]) as f32 * self.period
                            }),
                        });
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

    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(index) = self.active {
            self.mark(encoder, QUERIES - 1);
            encoder.resolve_query_set(&self.queries, 0..QUERIES, &self.resolve, 0);
            encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.pending[index].buffer, 0, BYTES);
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
